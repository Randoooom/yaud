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
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use surrealdb::sql::Thing;

#[derive(Deserialize, Debug, Clone)]
pub struct Hook {
    id: Thing,
    token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HookRequest {
    token: String,
}

pub async fn hook(
    State(state): State<ApplicationState>,
    Json(data): Json<HookRequest>,
) -> Result<StatusCode> {
    let connection = state.connection();
    // authorize the request
    authorize_request(data.token.as_str(), connection).await?;

    // TODO: hooks

    Ok(StatusCode::OK)
}

#[instrument(skip_all)]
pub async fn authorize_request(token: &str, connection: &DatabaseConnection) -> Result<()> {
    let hook: Option<Hook> = connection
        .query("SELECT * FROM hook WHERE token = $authentication")
        .bind(("authentication", token))
        .await?
        .take(0)?;

    hook.ok_or(ApplicationError::Unauthorized)
        .and_then(|_| Ok(()))
}

#[cfg(test)]
mod tests {
    use crate::hook::Hook;
    use crate::prelude::*;
    use crate::tests::TestSuite;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_authentication() -> Result<()> {
        let suite = TestSuite::init().await?;

        let response = suite
            .client()
            .post("/hook")
            .json(&json!({
                "token": "test"
            }))
            .send()
            .await;
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());

        let hook: Vec<Hook> = suite.connection().create("hook").await?;
        let response = suite
            .client()
            .post("/hook")
            .json(&json!({
                "token": hook.first().unwrap().token
            }))
            .send()
            .await;
        assert_eq!(StatusCode::OK, response.status());

        Ok(())
    }
}
