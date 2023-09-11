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

use crate::database::ConnectionInfo;
use crate::prelude::*;

#[derive(Clone, Getters)]
#[get = "pub"]
pub struct ApplicationState {
    connection_info: ConnectionInfo,
}

impl From<ConnectionInfo> for ApplicationState {
    fn from(connection_info: ConnectionInfo) -> Self {
        Self { connection_info }
    }
}

impl ApplicationState {
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection_info.connection
    }
}
