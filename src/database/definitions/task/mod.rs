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

use crate::database::definitions::account::Account;
use crate::database::definitions::task::state::TaskState;
use crate::prelude::*;
use chrono::{DateTime, Utc};

pub mod request;
pub mod state;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Deserialize, Serialize, JsonSchema, Debug, Clone, PartialEq, Getters)]
#[get = "pub"]
pub struct Task {
    id: Id,
    title: String,
    customer: Relation<Account>,
    description: String,
    due: Option<DateTime<Utc>>,
    state: TaskState,
    priority: TaskPriority,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}
