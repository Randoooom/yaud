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
use surrealdb::sql::Thing;

#[derive(Deserialize, Debug)]
struct Hook {
    id: Thing,
}

#[instrument(skip_all)]
pub async fn hook(connection: &DatabaseConnection) -> Result<()> {
    let span = info_span!("Hook");
    let _ = span.enter();

    // fetch the may pending hook
    let hook: Option<Hook> = sql_span!(
        connection
            .query("SELECT * FROM hook WHERE pending")
            .await?
            .take(0)?,
        "fetching hooks"
    );

    if let Some(hook) = hook {
        // TODO: do captain hook things

        // update the hook
        let _: Option<Hook> = sql_span!(
            connection
                .update(hook.id.clone())
                .merge(&json!({
                    "pending": false
                }))
                .await?,
            "finalizing hook"
        );
    }

    Ok(())
}
