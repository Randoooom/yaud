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

use crate::auth::session::Session;
use crate::auth::Authenticate;
use crate::database::definitions::account::Account;
use crate::error::ApplicationErrorResponse;
use crate::prelude::{ApplicationError, ApplicationState, Json, DOMAIN, HCAPTCHA_SECRET};
use aide::axum::routing::post_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use hcaptcha::Hcaptcha;

pub fn router(state: ApplicationState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/login", post_with(login, login_docs))
        .with_state(state)
}

#[derive(Deserialize, JsonSchema, Debug, Clone, Hcaptcha)]
pub struct LoginRequest {
    /// the username
    username: String,
    /// the password
    password: String,
    /// the totp token for optional enabled totp authentication
    token: Option<String>,
    #[captcha]
    #[cfg(not(test))]
    hcaptcha: String,
}

#[derive(Serialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    reactivate_totp: bool,
    session: Session,
}

async fn login(
    State(state): State<ApplicationState>,
    jar: CookieJar,
    Json(data): Json<LoginRequest>,
) -> crate::Result<(CookieJar, Json<LoginResponse>)> {
    // verify the captcha value
    #[cfg(not(test))]
    data.valid_response(&HCAPTCHA_SECRET, None).await?;

    // fetch the account
    match Account::from_username(data.username.as_str(), state.connection()).await? {
        Some(account) => {
            // start the login process
            account
                .login(data.password.as_str(), data.token.as_deref())
                .await?;

            // start a new session
            let session = account.start_session(state.connection()).await?;
            // build the session cookie
            let cookie = Cookie::build("session_id", session.id.to_string())
                .same_site(SameSite::Strict)
                .http_only(true)
                .secure(true)
                .domain(DOMAIN.as_str())
                .finish();
            let response = LoginResponse {
                reactivate_totp: account.totp().reactivate().clone(),
                session,
            };

            Ok((jar.add(cookie), Json(response)))
        }
        None => Err(ApplicationError::Unauthorized),
    }
}

fn login_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Start a new session in order to be able to authenticate and authorize further requests")
        .summary("Start a new session")
        .response_with::<200, Json<LoginResponse>, _>(|transform| transform.description("Login succeeded"))
        .response_with::<401, Json<ApplicationErrorResponse>, _>(|transform| transform.description("Invalid credentials"))
}
