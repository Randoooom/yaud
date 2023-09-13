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
#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate strum;
#[macro_use]
extern crate lazy_static;

use crate::error::ApplicationError;
use std::ops::Deref;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod database;
mod error;
mod hook;

const HOOK_INTERVAL: u64 = 10000;

i18n!("locales", fallback = "en");

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    lazy_static::initialize(&CONFIGURATION);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (sender, receiver) = kanal::unbounded_async();

    let info = database::connect(None).await?;
    let connection = info.connection;

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
                _ = receiver.recv() => {
                    warn!("Received shutdown signal on kanal receiver");
                    break;
                }
            }
        }

        Ok::<(), ApplicationError>(())
    });

    match tokio::signal::ctrl_c().await {
        Ok(()) => {}
        Err(error) => {
            error!("Unable to listen for shutdown signal: {}", error);
            sender.send(true).await?;
        }
    }

    info!("Received shutdown signal... Shutting down...");
    // shutdown
    sender.send(true).await?;
    Ok(())
}

pub mod prelude {
    pub use crate::database::DatabaseConnection;
    pub use crate::error::*;
    pub use crate::sql_span;
}
