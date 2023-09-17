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
#[cfg(feature = "ssr")]
#[macro_use]
extern crate thiserror;
#[cfg(feature = "ssr")]
#[macro_use]
extern crate tracing;
#[cfg(feature = "ssr")]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate rust_i18n;
#[macro_use]
extern crate strum;

#[cfg(feature = "ssr")]
mod server;
mod web;

i18n!("locales", fallback = "en");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "web")]
    web::init()?;

    #[cfg(feature = "ssr")]
    server::init()?;

    Ok(())
}

pub mod prelude {
    #[cfg(feature = "ssr")]
    pub use crate::server::database::DatabaseConnection;
    #[cfg(feature = "ssr")]
    pub use crate::server::error::*;
    #[cfg(feature = "ssr")]
    pub use crate::server::hook::ActionType;
    #[cfg(feature = "ssr")]
    pub use crate::server::state::ApplicationState;
    #[cfg(feature = "ssr")]
    pub use crate::server::CONFIGURATION;
    #[cfg(feature = "ssr")]
    pub use crate::sql_span;

    pub use crate::web::route::Route;
    pub use crate::{handler_navigate_to, optional_handler};
    pub use dioxus::prelude::*;
    pub use dioxus_fullstack::prelude::*;
    pub use dioxus_router::prelude::*;
}
