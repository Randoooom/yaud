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

use crate::auth::DeriveEncryptionKey;
use crate::database::definitions::account::{Account, WriteAccount};
use crate::prelude::*;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Extension;
use totp_rs::{Algorithm, TOTP};

pub fn router(state: ApplicationState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/",
            get_with(get_qr_code, get_qr_code_docs)
                .put_with(toggle, toggle_docs)
                .layer(require_session!(state, PERMISSION_NONE)),
        )
        .with_state(state)
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
struct QrCodeRequest {
    password: String,
}

#[derive(Serialize, JsonSchema, Debug, Clone)]
struct QrCodeResponse {
    data: String,
}

async fn get_qr_code(
    Extension(account): Extension<Account>,
    Json(data): Json<QrCodeRequest>,
) -> Result<Json<QrCodeResponse>> {
    // access the secret
    let secret = crate::auth::decrypt(
        &account.derive_key(data.password.as_str())?,
        account.secret().as_str(),
    );
    // build the totp
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.as_bytes().to_vec(),
        Some("Freelance Dashboard".to_owned()),
        account.mail().clone(),
    )
    .map_err(|_| ApplicationError::Unauthorized)?;

    Ok(Json(QrCodeResponse {
        data: totp.get_qr().map_err(|_| ApplicationError::Unauthorized)?,
    }))
}

fn get_qr_code_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Get the totp secret data encoded in qrcode format")
        .summary("Access the totp qrcode")
        .response::<200, Json<QrCodeResponse>>()
        .response_with::<401, Json<ApplicationErrorResponse>, _>(|transform| {
            transform.description("Invalid credentials")
        })
}

#[derive(Deserialize, Debug, Clone)]
struct ToggleRequest {
    password: String,
    token: String,
}

async fn toggle(
    State(state): State<ApplicationState>,
    Extension(account): Extension<Account>,
    Json(data): Json<ToggleRequest>,
) -> Result<StatusCode> {
    let _ = WriteAccount::from(state.connection())
        .set_totp(!account.totp().active().clone())
        .authenticate(data.password.as_str(), Some(data.token.as_str()))
        .to_owned()
        .await?;

    Ok(StatusCode::OK)
}

fn toggle_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Toggle the current totp status")
        .summary("Toggle the current totp status")
        .response::<200, StatusCode>()
        .response_with::<401, Json<ApplicationErrorResponse>, _>(|transform| {
            transform.description("Invalid credentials")
        })
}
