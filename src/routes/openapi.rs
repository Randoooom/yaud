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
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::{ApiKeyLocation, OpenApi, SecurityScheme};
use aide::redoc::Redoc;
use aide::transform::TransformOpenApi;
use axum::http::header::AUTHORIZATION;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Extension;
use std::ops::Deref;
use std::sync::Arc;

pub fn router(state: ApplicationState) -> ApiRouter {
    aide::gen::infer_responses(true);

    let router = ApiRouter::new()
        .api_route(
            "/",
            get_with(
                Redoc::new("/docs/private/api.json")
                    .with_title("Redoc")
                    .axum_handler(),
                |op| op.description("This documentation page."),
            ),
        )
        .route("/private/api.json", get(serve_docs))
        .with_state(state);

    aide::gen::infer_responses(false);

    router
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}

pub fn transform_api(api: TransformOpenApi) -> TransformOpenApi {
    api.title("yaud")
        .summary("Yeet another useless dashboard")
        .description("ADVERTISE HERE PLEASE")
        .security_scheme(
            "AccessToken",
            SecurityScheme::ApiKey {
                location: ApiKeyLocation::Header,
                name: AUTHORIZATION.to_string(),
                description: Some("PASETOv4Local".to_owned()),
                extensions: Default::default(),
            },
        )
}
