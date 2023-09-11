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

use crate::prelude::DatabaseConnection;
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
}

impl TestSuite {
    pub async fn init() -> Result<Self, BoxError> {
        let connection_info = crate::database::connect().await?;
        let client = TestClient::new(crate::router(connection_info.clone()).await?);
        let connection = connection_info.connection;

        Ok(Self { client, connection })
    }
}
