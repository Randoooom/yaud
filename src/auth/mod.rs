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

use crate::auth::session::{EndSession, Session, WriteSession};
use crate::database::definitions::account::Account;
use crate::prelude::*;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chacha20poly1305::aead::{Aead, Key, OsRng};
use chacha20poly1305::{AeadCore, KeyInit, XChaCha20Poly1305, XNonce};
use totp_rs::{Algorithm, TOTP};

pub mod authz;
pub mod middleware;
pub mod session;

/// This encrypt the given data with the given key (argon2dwr hash) using xChaCha20Poly1305
#[instrument(skip_all)]
pub fn encrypt(key: &[u8; 32], data: &str) -> String {
    // setup the cipher
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let cipher = XChaCha20Poly1305::new(Key::<XChaCha20Poly1305>::from_slice(key));

    // encrypt the data
    let encrypted = cipher.encrypt(&nonce, data.as_bytes()).unwrap();
    // encode
    format!(
        "{}:{}",
        openssl::base64::encode_block(nonce.as_slice()),
        openssl::base64::encode_block(encrypted.as_slice())
    )
}

/// This decrypts the given data with the given key (argon2d hash) using xChaCha20Poly1305
#[instrument(skip_all)]
pub fn decrypt(key: &[u8; 32], data: &str) -> String {
    // prepare the cipher
    let cipher = XChaCha20Poly1305::new(Key::<XChaCha20Poly1305>::from_slice(key));

    // read the encrypted data
    let mut split = data.split(':');
    let nonce = openssl::base64::decode_block(split.next().unwrap()).unwrap();
    let data = openssl::base64::decode_block(split.next().unwrap()).unwrap();

    // decrypt the secret
    let nonce = XNonce::from_slice(nonce.as_slice());
    let decrypted = cipher.decrypt(nonce, data.as_slice()).unwrap();

    // convert to the utf8 encoded base32
    String::from_utf8(decrypted).unwrap()
}

pub trait DeriveEncryptionKey {
    fn derive_key(&self, password: &str) -> Result<[u8; 32]>;
}

impl DeriveEncryptionKey for Account {
    #[instrument(skip_all)]
    fn derive_key(&self, password: &str) -> Result<[u8; 32]> {
        let mut target = [0u8; 32];
        Argon2::default().hash_password_into(
            password.as_bytes(),
            self.nonce().as_bytes(),
            &mut target,
        )?;

        Ok(target)
    }
}

#[async_trait]
pub trait Authenticate {
    fn login(&self, password: &str, token: Option<&str>) -> Result<()>;
    async fn logout(&self, connection: &DatabaseConnection) -> Result<()>;
    async fn start_session(&self, connection: &DatabaseConnection) -> Result<Session>;
}

#[async_trait]
impl Authenticate for Account {
    #[instrument(skip_all)]
    fn login(&self, password: &str, token: Option<&str>) -> Result<()> {
        // try to derive the key
        let key = self.derive_key(password)?;
        // compare the hashes
        Argon2::default().verify_password(&key, &PasswordHash::new(self.password().as_str())?)?;

        // verify the totp token, if enabled
        if *self.totp().active() && !self.totp().reactivate() {
            return if let Some(token) = token {
                if TOTP::new(
                    Algorithm::SHA1,
                    6,
                    1,
                    30,
                    decrypt(&self.derive_key(password)?, self.secret().as_str())
                        .as_bytes()
                        .to_vec(),
                    None,
                    "".to_owned(),
                )
                .unwrap()
                .check_current(token)?
                {
                    Ok(())
                } else {
                    Err(ApplicationError::Unauthorized)
                }
            } else {
                Err(ApplicationError::Forbidden("TOTP is required".to_owned()))
            };
        }

        Ok(())
    }

    async fn logout(&self, connection: &DatabaseConnection) -> Result<()> {
        EndSession::new(self.id(), connection).await
    }

    async fn start_session(&self, connection: &DatabaseConnection) -> Result<Session> {
        WriteSession::new(self.id(), connection).await
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::{Authenticate, DeriveEncryptionKey};
    use crate::database::definitions::account::WriteAccount;
    use axum::BoxError;
    use chrono::{Duration, Local};
    use totp_rs::{Algorithm, TOTP};

    #[tokio::test]
    async fn test_login() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?;
        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first"))
            .set_last_name(Some("last"))
            .set_mail(Some("test@test.de"))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        assert!(account.login("password", None).is_ok());
        assert!(account.login("password1", None).is_err());
        assert!(account.login("password", Some("123456")).is_ok());

        assert!(WriteAccount::from(&connection)
            .set_target(Some(&account))
            .set_password(Some("different".to_owned()))
            .to_owned()
            .await
            .is_err());
        let account = WriteAccount::from(&connection)
            .set_target(Some(&account))
            .set_password(Some("different".to_owned()))
            .authenticate("password", None)
            .to_owned()
            .await?;
        assert!(account.login("different", None).is_ok());
        assert!(account.login("password", None).is_err());

        let account = WriteAccount::from(&connection)
            .set_target(Some(&account))
            .set_totp(true)
            .to_owned()
            .await?;
        let key = account.derive_key("different")?;
        let secret = crate::auth::decrypt(&key, account.secret().as_str());
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret.as_bytes().to_vec(),
            None,
            "".to_owned(),
        )
        .unwrap();
        let token = totp.generate_current().unwrap();
        let invalid_token =
            totp.generate((Local::now() - Duration::seconds(300)).timestamp() as u64);
        assert!(account.login("different", None).is_err());
        assert!(account.login("different", Some("123456")).is_err());
        assert!(account
            .login("different", Some(invalid_token.as_str()))
            .is_err());
        assert!(account.login("different", Some(token.as_str())).is_ok());
        assert!(account.login("password", Some(token.as_str())).is_err());

        Ok(())
    }
}
