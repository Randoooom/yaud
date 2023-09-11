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
#[writer(table = "task_request", impl_full_request)]
#[get = "pub"]
pub struct TaskRequest {
    #[writer(skip)]
    id: Id,
    #[writer(editable)]
    title: String,
    #[writer(skip_full)]
    customer: Relation<Account>,
    #[writer(editable)]
    description: String,
    #[writer(editable)]
    due: DateTime<Utc>,
    #[writer(full = "TaskRequestState::Received")]
    state: TaskRequestState,
    #[writer(skip_full)]
    updated_at: DateTime<Utc>,
    #[writer(skip)]
    created_at: DateTime<Utc>,
}
