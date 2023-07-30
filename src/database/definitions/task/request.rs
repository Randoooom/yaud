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
 *
 */

use crate::database::definitions::account::Account;
use crate::prelude::*;
use chrono::{DateTime, Utc};

#[derive(Deserialize, Serialize, Debug, Clone, JsonSchema, PartialEq)]
pub enum TaskRequestState {
    Received,
    Evaluation,
    Accepted,
    Rejected,
}

#[derive(Deserialize, Serialize, JsonSchema, Debug, Clone, PartialEq, Getters, DataWriter)]
#[writer(table = "task_request")]
#[get = "pub"]
pub struct TaskRequest {
    id: Id,
    title: String,
    customer: Relation<Account>,
    description: String,
    due: Option<DateTime<Utc>>,
    state: TaskRequestState,
    updated_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}
