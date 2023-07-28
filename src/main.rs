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

#[macro_use]
extern crate serde;
#[macro_use]
extern crate schemars;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate aide;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate getset;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate axum_macros;

use crate::prelude::ApplicationState;
use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use axum::http::{header, Method};
use axum::{BoxError, Extension, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod auth;
mod database;
mod error;
mod routes;
mod state;

pub async fn router() -> Result<Router, BoxError> {
    let connection = database::connect().await?;
    let state = ApplicationState::from(connection);

    aide::gen::extract_schemas(true);
    let mut api = OpenApi::default();

    Ok(ApiRouter::new()
        .nest_api_service("/docs", routes::openapi::router(state.clone()))
        .finish_api_with(&mut api, routes::openapi::transform_api)
        .layer(
            CorsLayer::new()
                .allow_origin([std::env::var("DOMAIN").unwrap().parse().unwrap()])
                .allow_methods(vec![
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::HEAD,
                    Method::OPTIONS,
                ])
                .allow_headers(vec![
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::CONTENT_DISPOSITION,
                ])
                .expose_headers(vec![header::CONTENT_DISPOSITION]),
        )
        .layer(Extension(Arc::new(api))))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let address = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("starting on http://0.0.0.0:8000");
    axum::Server::bind(&address)
        .serve(router().await?.into_make_service())
        .await
        .unwrap();

    Ok(())
}

pub mod prelude {
    pub use crate::auth::authz::permission::*;
    pub use crate::database::id::Id;
    pub use crate::database::page::Page;
    pub use crate::database::DatabaseConnection;
    pub use crate::error::*;
    pub use crate::routes::extractor::*;
    pub use crate::sql_span;
    pub use crate::state::*;
}
