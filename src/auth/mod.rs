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
use crate::prelude::*;
use argon2::password_hash::Error::Password;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chacha20poly1305::aead::{Aead, Key, OsRng};
use chacha20poly1305::{AeadCore, KeyInit, XChaCha20Poly1305, XNonce};
use totp_rs::{Algorithm, TOTP};

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
    async fn login(&self, password: &str, token: Option<&str>) -> Result<()>;
    async fn logout(&self, connection: &DatabaseConnection) -> Result<()>;
    async fn start_session(&self, connection: &DatabaseConnection) -> Result<()>;
    async fn fetch_session(&self, connection: &DatabaseConnection) -> Result<()>;
}

impl Authenticate for Account {
    #[instrument(skip_all)]
    async fn login(&self, password: &str, token: Option<&str>) -> Result<()> {
        // try to derive the key
        let key = self.derive_key(password)?;
        // compare the hashes
        Argon2::default().verify_password(&key, &PasswordHash::new(self.password().as_str())?)?;

        // verify the totp token, if enabled
        if self.totp().active() && !self.totp().reactivate() {
            return if let Some(token) = token {
                if TOTP::new(
                    Algorithm::SHA1,
                    6,
                    1,
                    30,
                    self.read_secret(password)?.as_bytes().to_vec(),
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
        todo!()
    }

    async fn start_session(&self, connection: &DatabaseConnection) -> Result<()> {
        todo!()
    }

    async fn fetch_session(&self, connection: &DatabaseConnection) -> Result<()> {
        todo!()
    }
}
