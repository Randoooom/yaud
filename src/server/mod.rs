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
use axum::Router;
use std::env::Vars;
use tower_http::compression::CompressionLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub mod database;
pub mod error;
pub mod hook;
pub mod state;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    surrealdb_endpoint: String,
    surrealdb_username: String,
    surrealdb_password: String,
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
    #[cfg(test)]
    test_mail: String,
    #[cfg(test)]
    test_mail2: String,
    #[cfg(test)]
    test_mail_key: String,
    #[cfg(test)]
    test_mail_namespace: String,
}

lazy_static! {
    pub static ref CONFIGURATION: Config = envy::from_env::<Config>().unwrap();
}

const HOOK_INTERVAL: u64 = 10000;

pub fn init() -> std::result::Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    lazy_static::initialize(&CONFIGURATION);

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::from_default_env())
                .with(tracing_subscriber::fmt::layer())
                .init();

            let (hook_sender, hook_receiver) = kanal::unbounded_async::<()>();
            let (axum_sender, axum_receiver) = kanal::unbounded_async::<()>();

            let info = database::connect(None).await?;
            let connection = info.connection.clone();

            // as the surrealdb rust-sdk currently does not support live queries we have to adapt here
            // and are regularly checking for new hook triggers.
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        result = hook::hook(&connection) => {
                            match result {
                                Ok(()) => {},
                                Err(error) => error!("Error occurred during hook: {}", error),
                            }

                            tokio::time::sleep(std::time::Duration::from_millis(HOOK_INTERVAL)).await;
                        },
                        _ = hook_receiver.recv() => {
                            warn!("Received shutdown signal on kanal receiver");
                            break;
                        }
                    }
                }

                Ok::<(), ApplicationError>(())
            });

            tokio::spawn(async move {
                let state = ApplicationState::from(info);

                let router = Router::new()
                    .serve_dioxus_application("", ServeConfigBuilder::new_with_router(FullstackRouterConfig::<Route>::default()))
                    .layer(CompressionLayer::new().gzip(true))
                    .with_state(state);
                let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8000));

                axum::Server::bind(&addr)
                    .serve(router.into_make_service())
                    .with_graceful_shutdown(async { axum_receiver.recv().await.ok(); })
                    .await
                    .unwrap();

                Ok::<(), ApplicationError>(())
            });

            match tokio::signal::ctrl_c().await {
                Ok(()) => {}
                Err(error) => {
                    error!("Unable to listen for shutdown signal: {}", error);
                    hook_sender.send(()).await?;
                    axum_sender.send(()).await?;
                }
            }

            info!("Received shutdown signal... Shutting down...");
            // shutdown
            hook_sender.send(()).await?;
            axum_sender.send(()).await?;
            Ok(())
        })
}
