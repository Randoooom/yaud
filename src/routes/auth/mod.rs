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

use crate::auth::session::{EndSession, Session};
use crate::auth::Authenticate;
use crate::database::definitions::account::Account;
use crate::error::ApplicationErrorResponse;
use crate::prelude::{ApplicationError, ApplicationState, Json, DOMAIN, HCAPTCHA_SECRET};
use aide::axum::routing::post_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::Extension;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
#[cfg(not(test))]
use hcaptcha::Hcaptcha;

pub fn router(state: ApplicationState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/login", post_with(login, login_docs))
        .api_route("/logout", post_with(logout, logout_docs))
        .with_state(state)
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[cfg_attr(not(test), derive(Hcaptcha))]
pub struct LoginRequest {
    /// the mail
    mail: String,
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
    match Account::from_mail(data.mail.as_str(), state.connection()).await? {
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

async fn logout(
    State(state): State<ApplicationState>,
    jar: CookieJar,
    Extension(account): Extension<Account>,
) -> crate::Result<CookieJar> {
    // access the cookie
    match jar.get("session_id") {
        Some(cookie) => {
            // delete the session
            EndSession::new(account.id(), state.connection()).await?;

            Ok(jar.remove(cookie.clone()))
        }
        None => Err(ApplicationError::Unauthorized),
    }
}

fn logout_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Stop the currently active session. This will automatically revoke all tokens and delete the cookie")
        .summary("Stop the current session")
        .response_with::<200, Json<LoginResponse>, _>(|transform| transform.description("Logout succeeded"))
        .response_with::<401, Json<ApplicationErrorResponse>, _>(|transform| transform.description("Invalid cookie"))
}

#[cfg(test)]
mod tests {
    use crate::tests::prelude::*;
    use axum::http::StatusCode;
    use axum::BoxError;

    #[tokio::test]
    async fn test_login() -> Result<(), BoxError> {
        let suite = TestSuite::init().await?;

        let response = suite
            .client()
            .post("/auth/login")
            .json(&json! ({
                "mail": TEST_MAIL.as_str(),
                "password": "password"
            }))
            .send()
            .await;
        assert_eq!(StatusCode::OK, response.status());

        let response = suite
            .client()
            .post("/auth/login")
            .json(&json!({
                "mail": "somethind different",
                "password": "password"
            }))
            .send()
            .await;
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());

        let response = suite
            .client()
            .post("/auth/login")
            .json(&json! {{
                "mail": TEST_MAIL.as_str(),
                "password": "wrong"
            }})
            .send()
            .await;
        assert_eq!(StatusCode::UNAUTHORIZED, response.status());

        Ok(())
    }
}
