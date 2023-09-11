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
extern crate thiserror;
#[macro_use]
extern crate getset;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate serde_json;

use crate::database::ConnectionInfo;
use crate::prelude::*;
use axum::{BoxError, Router};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod database;
mod error;
mod hook;
mod state;

#[cfg(test)]
mod tests;

pub async fn router(connection: ConnectionInfo) -> std::result::Result<Router, BoxError> {
    let state = ApplicationState::from(connection);

    Ok(Router::new()
        .layer(TraceLayer::new_for_http())
        .with_state(state))
}

#[tokio::main]
async fn main() -> std::result::Result<(), BoxError> {
    let _ = std::env::var("DOMAIN").expect("DOMAIN NOT FOUND");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let address = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("starting on http://0.0.0.0:8000");
    let connection = database::connect().await?;
    axum::Server::bind(&address)
        .serve(router(connection).await?.into_make_service())
        .await
        .unwrap();

    Ok(())
}

pub mod prelude {
    pub use crate::database::DatabaseConnection;
    pub use crate::error::*;
    pub use crate::state::*;
}
