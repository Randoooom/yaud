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

use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
#[cfg(not(test))]
use version_compare::{Cmp, Version};

pub mod definitions;
pub mod id;
pub mod page;

const SURREALDB_ENDPOINT: &str = "SURREALDB_ENDPOINT";
const SURREALDB_USERNAME: &str = "SURREALDB_USERNAME";
const SURREALDB_PASSWORD: &str = "SURREALDB_PASSWORD";

pub type DatabaseConnection = Surreal<Client>;

pub async fn connect() -> Result<DatabaseConnection> {
    // establish the connection
    let client: Surreal<Client> = Surreal::new::<Ws>(
        std::env::var(SURREALDB_ENDPOINT)
            .unwrap_or_else(|_| panic!("Missing {SURREALDB_ENDPOINT} env variable")),
    )
    .await?;
    info!("Established connection to surrealdb");

    // authenticate
    client
        .signin(Root {
            username: std::env::var(SURREALDB_USERNAME)
                .unwrap_or_else(|_| panic!("Missing {SURREALDB_USERNAME} env variable"))
                .as_str(),
            password: std::env::var(SURREALDB_PASSWORD)
                .unwrap_or_else(|_| panic!("Missing {SURREALDB_PASSWORD} env variable"))
                .as_str(),
        })
        .await?;
    info!("Authenticated with surrealdb");

    // use namespace and database
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            let db = nanoid::nanoid!();
            println!("Connected with database {:?} in namespace \"test\"", db);

            client
                .use_ns("test")
                .use_db(db)
                .await?;
        } else {
            client
                .use_ns("production")
                .use_db("yaud")
                .await?;
        }
    }

    // perform the migrations
    #[cfg(not(test))]
    migrate(&client, env!("CARGO_PKG_VERSION"), Vec::new()).await?;
    // execute the up queries
    client.query(include_str!("./up.surrealql")).await?;
    info!("Initiated tables");
    // initialize the permissions
    PermissionHandler::from(&client).await?;

    Ok(client)
}

#[cfg(not(test))]
pub async fn migrate(
    client: &DatabaseConnection,
    current_version: &'static str,
    migrations: Vec<(&'static str, &'static str)>,
) -> Result<()> {
    // initiate the migration table and fetch possibly already existing records
    let mut responses = client
        .query(
            "DEFINE TABLE migration SCHEMALESS;
            DEFINE FIELD version     on TABLE migration TYPE string ASSERT $value IS NOT NULL;
            DEFINE FIELD created_at  on TABLE migration TYPE datetime VALUE time::now();",
        )
        .query("SELECT version, created_at FROM migration ORDER BY created_at DESC LIMIT 1")
        .await?
        .check()?;
    // take the last as response, which contains the last migrated version
    let last = responses.take::<Option<String>>((1, "version"))?;

    if let Some(last) = last {
        // only proceed if the  last version is not equal to the current version
        if !last.as_str().eq(current_version) {
            // iterate through the given migrations
            for (version, migration) in migrations {
                if Version::from(last.as_str())
                    .unwrap()
                    .compare_to(Version::from(current_version).unwrap(), Cmp::Lt)
                {
                    info!("Executing surrealdb migration to {version}");
                    // execute the migration query and mark it as done
                    client
                        .query(migration)
                        .query("CREATE migration SET version = $version")
                        .bind(("version", version))
                        .await?
                        .check()?;
                }
            }
        }
    } else {
        // insert the current version as the last version
        client
            .query("CREATE migration SET version = $version")
            .bind(("version", current_version))
            .await?
            .check()?;
    }

    Ok(())
}

#[macro_export]
macro_rules! sql_span {
    ($expr: expr) => {{
        let span = info_span!("Surrealdb Request");
        let _ = span.enter();
        $expr
    }};
    ($expr: expr, $title: expr) => {{
        let span = info_span!(concat!("Surrealdb Request: ", $title));
        let _ = span.enter();
        $expr
    }};
}
