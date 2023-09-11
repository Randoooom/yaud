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

use crate::auth::authz::WritePermissions;
use crate::database::definitions::account::{Account, WriteAccount};
use crate::prelude::{DatabaseConnection, PERMISSIONS};
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
        let connection_info = crate::database::connect().await?;
        let client = TestClient::new(crate::router(connection_info.clone()).await?);
        let connection = connection_info.connection;

        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first"))
            .set_last_name(Some("last"))
            .set_mail(Some(TEST_MAIL.as_str()))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        // grant all permissions
        for permission in PERMISSIONS.iter() {
            account.grant_permission(permission, &connection).await?;
        }

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

#[cfg(test)]
mod reproduce {
    use axum::BoxError;
    use surrealdb::opt::RecordId;
    use surrealdb::opt::Resource::RecordId;
    use surrealdb::sql::Thing;

    #[tokio::test]
    async fn reproduce() -> Result<(), BoxError> {
        let connection = &crate::database::connect().await?.connection;

        #[derive(Deserialize, Serialize, Debug, Clone)]
        struct Person {
            id: Thing,
            name: String,
        }

        #[derive(Deserialize, Serialize, Debug, Clone)]
        struct Note {
            id: Thing,
            content: String,
            owner: Thing,
        }

        connection
            .query("DEFINE TABLE person SCHEMAFULL")
            .query("DEFINE FIELD name ON TABLE person TYPE string ASSERT $value IS NOT NULL")
            .query("DEFINE TABLE note SCHEMAFULL")
            .query("DEFINE FIELD content ON TABLE note TYPE string ASSERT $value IS NOT NULL")
            .query("DEFINE FIELD owner ON TABLE note TYPE record(person) ASSERT $value IS NOT NULL")
            .await?
            .check()?;

        let person: Person = connection
            .create(("person", "john"))
            .content(json! ({
               "name": "john"
            }))
            .await?
            .unwrap();

        // doesn't work
        let note: Vec<Note> = connection
            .create("note")
            .content(json! ({
               "content": "insert here",
                "owner": RecordId::from(person.id.clone())
            }))
            .await?;

        // also doesn't work
        let note: Vec<Note> = connection
            .create("note")
            .content(json! ({
               "content": "insert here",
                "owner": "person:john"
            }))
            .await?;

        Ok(())
    }
}
