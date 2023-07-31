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

use crate::auth::{Authenticate, DeriveEncryptionKey};
use crate::prelude::*;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{DateTime, Utc};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use totp_rs::Secret;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Getters, Default, JsonSchema)]
#[get = "pub"]
pub struct TotpData {
    active: bool,
    reactivate: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Getters, JsonSchema)]
#[get = "pub"]
pub struct Account {
    id: Id,
    first_name: String,
    last_name: String,
    mail: String,
    password: String,
    nonce: String,
    secret: String,
    totp: TotpData,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

impl Account {
    #[instrument(skip(connection))]
    pub async fn from_mail(mail: &str, connection: &DatabaseConnection) -> Result<Option<Account>> {
        let account = sql_span!(connection
            .query("SELECT * FROM account WHERE mail = $mail")
            .bind(("mail", mail))
            .await?
            .take::<Option<Account>>(0)?);

        Ok(account)
    }
}

#[derive(Clone, Debug, Serialize, Getters, Setters, MutGetters)]
pub struct WriteAccount<'a> {
    #[get = "pub"]
    #[set = "pub"]
    #[serde(skip_serializing_if = "Option::is_none")]
    first_name: Option<&'a str>,
    #[get = "pub"]
    #[set = "pub"]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_name: Option<&'a str>,
    #[get = "pub"]
    #[set = "pub"]
    #[serde(skip_serializing_if = "Option::is_none")]
    mail: Option<&'a str>,
    #[get = "pub"]
    #[set = "pub"]
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(skip)]
    old_password: Option<&'a str>,
    #[serde(skip)]
    token: Option<&'a str>,
    #[serde(skip)]
    connection: &'a DatabaseConnection,
    #[serde(skip)]
    #[set = "pub"]
    target: Option<&'a Account>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[get_mut = "pub"]
    totp: Option<TotpData>,
}

impl<'a> From<&'a DatabaseConnection> for WriteAccount<'a> {
    fn from(connection: &'a DatabaseConnection) -> Self {
        Self {
            first_name: None,
            last_name: None,
            mail: None,
            password: None,
            connection,
            target: None,
            secret: None,
            nonce: None,
            totp: None,
            old_password: None,
            token: None,
        }
    }
}

impl<'a> WriteAccount<'a> {
    pub fn set_totp(&mut self, value: bool) -> &mut Self {
        if let Some(ref mut data) = self.totp_mut() {
            data.active = value;
        } else {
            self.totp = Some(TotpData {
                active: value,
                reactivate: false,
            })
        }

        self
    }

    pub fn authenticate(
        &mut self,
        password: &'a str,
        token: Option<&'a str>,
    ) -> &mut WriteAccount<'a> {
        self.token = token;
        self.old_password = Some(password);

        self
    }
}

impl<'a> IntoFuture for WriteAccount<'a> {
    type Output = Result<Account>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    #[instrument(skip_all)]
    fn into_future(mut self) -> Self::IntoFuture {
        Box::pin(async move {
            // if the password gets changed and the target is not null we have to check the old password
            if self.password.is_some() {
                if let Some(account) = &self.target {
                    if let Some(old_password) = &self.old_password {
                        // verify the credentials
                        account.login(old_password, self.token)?;
                    } else {
                        return Err(ApplicationError::Unauthorized);
                    }
                }
            };

            // check if sensible data has to get changed
            if let Some(password) = self.password {
                // generate a random nonce
                let nonce = SaltString::generate(&mut OsRng).to_string();
                self.nonce = Some(nonce.clone());
                // init hasher
                let hasher = Argon2::default();

                // hash the first time
                let mut key = [0u8; 32];
                hasher.hash_password_into(password.as_bytes(), nonce.as_bytes(), &mut key)?;
                // hash the second time
                self.password = Some(
                    hasher
                        .hash_password(&key, &SaltString::generate(&mut OsRng))?
                        .to_string(),
                );

                // regenerate the secret
                let secret = Secret::generate_secret();
                self.secret = Some(crate::auth::encrypt(&key, secret.to_string().as_str()));

                // if totp was active the user has to refresh it
                // this will allow just on login without the totp
                if self.target.is_some_and(|account| account.totp().active) {
                    self.totp = Some(TotpData {
                        active: true,
                        reactivate: true,
                    })
                }
            }

            // other fields don't really matter here so we can proceed with merging the data
            let account: Account = if let Some(target) = self.target {
                sql_span!(
                    self.connection
                        .update(target.id.to_thing())
                        .merge(self)
                        .await?
                )
            } else {
                sql_span!(self.connection.create("account").content(self).await?)
            };

            Ok(account)
        })
    }
}

#[cfg(test)]
mod test {
    use crate::database::definitions::account::WriteAccount;
    use axum::BoxError;

    #[tokio::test]
    async fn test_write() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?;

        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first name"))
            .set_last_name(Some("last name"))
            .set_mail(Some("test@test.de"))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        assert_eq!(account.first_name, "first name".to_owned());
        assert_eq!(account.last_name, "last name".to_owned());
        assert_eq!(account.mail, "test@test.de".to_owned());
        assert_ne!(account.password, "password".to_owned());

        Ok(())
    }
}
