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

use crate::database::definitions::account::{Account, WriteAccount};
use crate::prelude::DatabaseConnection;
use crate::routes::auth::LoginResponse;
use axum::BoxError;
use axum_test_helper::TestClient;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref TEST_MAIL: String = std::env::var("TEST_MAIL").unwrap();
}

#[derive(Getters)]
#[get = "pub"]
pub struct TestSuite {
    client: TestClient,
    connection: DatabaseConnection,
    account: Account,
}

impl TestSuite {
    pub async fn init() -> Result<Self, BoxError> {
        let connection = crate::database::connect().await?;
        let client = TestClient::new(crate::router(connection.clone()).await?);
        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first"))
            .set_last_name(Some("last"))
            .set_mail(Some(TEST_MAIL.as_str()))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        Ok(Self {
            client,
            connection,
            account,
        })
    }

    pub async fn authorize_default(&self) -> LoginResponse {
        let response = self
            .client
            .post("/auth/login")
            .json(&json! ({
                "mail": TEST_MAIL.as_str(),
                "password": "password"
            }))
            .send()
            .await;

        response.json::<LoginResponse>().await
    }
}

pub mod prelude {
    pub use crate::tests::TestSuite;
    pub use crate::tests::TEST_MAIL;
}
