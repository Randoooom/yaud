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
use crate::prelude::*;
use aide::axum::routing::put_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;

pub fn router(state: ApplicationState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/",
            put_with(change_password, change_password_docs)
                .layer(require_session!(state, PERMISSION_NONE)),
        )
        .with_state(state)
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    old_password: String,
    new_password: String,
    token: Option<String>,
}

async fn change_password(
    State(state): State<ApplicationState>,
    Extension(account): Extension<Account>,
    Json(data): Json<ChangePasswordRequest>,
) -> Result<StatusCode> {
    WriteAccount::from(state.connection())
        .set_password(Some(data.new_password))
        .set_target(Some(&account))
        .authenticate(data.old_password.as_str(), data.token.as_deref())
        .to_owned()
        .await?;

    Ok(StatusCode::OK)
}

fn change_password_docs(transform: TransformOperation) -> TransformOperation {
    transform
}

#[cfg(test)]
mod tests {
    use crate::tests::prelude::*;
    use axum::http::StatusCode;
    use axum::BoxError;

    #[tokio::test]
    async fn test_password_change() -> Result<(), BoxError> {
        let suite = TestSuite::init().await?;
        suite.authorize_default().await;

        let response = suite
            .client()
            .put("/auth/password")
            .json(&json! {{
                "oldPassword": "password",
                "newPassword": "new_password"
            }})
            .send()
            .await;
        assert_eq!(StatusCode::OK, response.status());

        let response = suite
            .client()
            .post("/auth/login")
            .json(&json! ({
                "mail": TEST_MAIL.as_str(),
                "password": "password"
            }))
            .send()
            .await;
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());

        let response = suite
            .client()
            .post("/auth/login")
            .json(&json! ({
                "mail": TEST_MAIL.as_str(),
                "password": "new_password"
            }))
            .send()
            .await;
        assert_eq!(StatusCode::OK, response.status());

        Ok(())
    }
}
