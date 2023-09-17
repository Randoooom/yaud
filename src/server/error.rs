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

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error(transparent)]
    SurrealdbError(#[from] surrealdb::Error),
    #[error("Internal error occurred")]
    InternalServerError,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    SMTPError(#[from] lettre::transport::smtp::Error),
}

pub type Result<T> = std::result::Result<T, ApplicationError>;
