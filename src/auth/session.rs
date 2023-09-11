/*
 *     Copyright (C) 2023  Fritz Ochsmann
 *
 *     This program is free software: you can redistribute it and/or modify
 *     it under the terms of the GNU Affero General Public License as published
 *     by the Free Software Foundation, either version 3 of the License, or
 *     (at your option) any later version.
 *
 *     This program is distributed in the hope that it will be useful,
 *     but WITHOUT ANY WARRANTY; without even the implied warranty of
 *     MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *     GNU Affero General Public License for more details.
 *
 *     You should have received a copy of the GNU Affero General Public License
 *     along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use crate::database::definitions::account::Account;
use crate::database::ConnectionInfo;
use crate::prelude::*;
use chrono::{DateTime, Duration, Utc};
use std::future::{Future, IntoFuture};
use std::pin::Pin;

const ALPHABET: [char; 62] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
    'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B',
    'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z',
];

// 15m
const SESSION_LENGTH: i64 = 900;
// 20m
const REFRESH_LENGTH: i64 = 1200;

#[derive(Clone, Debug, Getters, Deserialize, Serialize, JsonSchema)]
pub struct Session {
    pub id: Id,
    #[get = "pub"]
    target: Id,
    #[get = "pub"]
    iat: DateTime<Utc>,
    #[get = "pub"]
    exp: DateTime<Utc>,
    refresh_token: String,
    refresh_exp: DateTime<Utc>,
}

impl Session {
    // Check whether a session is valid or not
    #[instrument(skip(connection))]
    pub async fn validate_session(id: &str, connection: &DatabaseConnection) -> Result<Session> {
        // fetch the session
        let session: Option<Session> =
            sql_span!(connection.select(&Id::try_from(("session", id))?).await?);

        match session {
            Some(session) => {
                if session.is_valid(connection).await.is_ok() {
                    Ok(session)
                } else {
                    Err(ApplicationError::Unauthorized)
                }
            }
            None => Err(ApplicationError::Unauthorized),
        }
    }

    #[instrument(skip_all)]
    pub async fn is_valid(&self, connection: &DatabaseConnection) -> Result<()> {
        if Utc::now() >= self.exp {
            // the session is not anymore valid, so we end it.
            self.end(connection).await?;

            Err(ApplicationError::Unauthorized)
        } else {
            Ok(())
        }
    }

    /// Ends the given session
    #[instrument(skip_all)]
    pub async fn end(&self, connection: &DatabaseConnection) -> Result<()> {
        let _: Option<Session> = sql_span!(connection.delete(&self.id).await?);

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn refresh(
        self,
        refresh_token: &str,
        connection: &DatabaseConnection,
    ) -> Result<Session> {
        if self.refresh_token.eq(refresh_token) {
            // start a new session, this automatically ends the current session
            WriteSession::new(&self.target, connection).await
        } else {
            self.end(connection).await?;

            Err(ApplicationError::Unauthorized)
        }
    }

    #[cfg(test)]
    pub fn refresh_token(&self) -> String {
        self.refresh_token.clone()
    }

    #[instrument(skip_all)]
    pub async fn start_database_session(
        &self,
        info: &ConnectionInfo,
    ) -> Result<DatabaseConnection> {
        let connection = info.connection.clone();
        connection.set("issuer", self.id.to_thing()).await?;

        Ok(connection)
    }
}

#[derive(Clone, Debug)]
pub struct EndSession<'a> {
    target: &'a Id,
    connection: &'a DatabaseConnection,
}

impl<'a> EndSession<'a> {
    pub fn new(target: &'a Id, connection: &'a DatabaseConnection) -> Self {
        Self { target, connection }
    }
}

impl<'a> IntoFuture for EndSession<'a> {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            sql_span!(self
                .connection
                .query("DELETE FROM session WHERE target = $target")
                .bind(("target", self.target.to_thing()))
                .await?
                .check()?);

            Ok(())
        })
    }
}

#[derive(Clone, Debug)]
pub struct WriteSession<'a> {
    target: &'a Id,
    connection: &'a DatabaseConnection,
}

impl<'a> WriteSession<'a> {
    pub fn new(target: &'a Id, connection: &'a DatabaseConnection) -> Self {
        Self { target, connection }
    }
}

impl<'a> IntoFuture for WriteSession<'a> {
    type Output = Result<Session>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    #[instrument(skip_all)]
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            let iat = Utc::now();
            let exp = iat + Duration::seconds(SESSION_LENGTH);
            let refresh_exp = iat + Duration::seconds(REFRESH_LENGTH);
            // generate session id
            let session_id = nanoid::nanoid!(64, &ALPHABET);
            let id = Id::new(("session", session_id.as_str()));
            let refresh_token = nanoid::nanoid!(64, &ALPHABET);

            // end currently active sessions for the target
            EndSession::new(self.target, self.connection).await?;

            // Ok(sql_span!(self
            //     .connection
            //     .create(id.to_thing())
            //     .content(&Session {
            //         id,
            //         target: self.target.clone(),
            //         iat,
            //         exp,
            //         refresh_token,
            //         refresh_exp,
            //     })
            //     .await?
            //     .unwrap()))
            let account = self
                .connection
                .select::<Option<Account>>(self.target)
                .await?
                .unwrap();

            Ok(sql_span!(self
                .connection
                .create(id.to_thing())
                .content(&json! ({
                    "target": account,
                    "iat": iat,
                    "exp": exp,
                    "refresh_token": refresh_token,
                    "refresh_exp": refresh_exp
                }))
                .await?
                .unwrap()))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::session::{Session, WriteSession};
    use crate::database::definitions::account::WriteAccount;
    use crate::prelude::Id;
    use crate::tests::TEST_MAIL;
    use axum::BoxError;

    #[tokio::test]
    async fn test_session_start() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?.connection;
        WriteSession::new(&Id::new(("account", "test")), &connection).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_session_validation() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?.connection;
        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first"))
            .set_last_name(Some("last"))
            .set_mail(Some(TEST_MAIL.as_str()))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        let session = WriteSession::new(account.id(), &connection).await?;
        assert!(session.is_valid(&connection).await.is_ok());
        let cloned_session = session.clone();

        let refreshed_session = session
            .refresh(cloned_session.refresh_token.as_str(), &connection)
            .await?;
        assert!(
            Session::validate_session(cloned_session.id.to_string().as_str(), &connection)
                .await
                .is_err()
        );
        assert_ne!(refreshed_session.id, cloned_session.id);
        assert_ne!(
            refreshed_session.refresh_token,
            cloned_session.refresh_token
        );

        Ok(())
    }
}
