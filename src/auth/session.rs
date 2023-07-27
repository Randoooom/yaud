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

impl Session {}

#[derive(Clone, Debug)]
pub struct WriteSession<'a> {
    target: Id,
    connection: &'a DatabaseConnection,
}

impl<'a> WriteSession<'a> {
    fn new(target: Id, connection: &'a DatabaseConnection) -> Self {
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

            sql_span!(
                self.connection
                    .query("DELETE FROM session WHERE target = $target")
                    .bind(("target", self.target.to_thing()))
                    .await?
                    .check()?,
                "end existing sessions"
            );

            Ok(sql_span!(
                self.connection
                    .create(id.to_thing())
                    .content(&Session {
                        id,
                        target: self.target,
                        iat,
                        exp,
                        refresh_token,
                        refresh_exp,
                    })
                    .await?
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::session::WriteSession;
    use crate::prelude::Id;
    use axum::BoxError;

    #[tokio::test]
    async fn test_session_start() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?;
        WriteSession::new(Id::new(("account", "test")), &connection).await?;

        Ok(())
    }
}
