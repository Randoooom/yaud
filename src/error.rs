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
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Error, Debug, OperationIo)]
pub enum ApplicationError {
    #[error("Unauthorized")]
    Unauthorized,
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("Internal error occurred")]
    InternalServerError,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct ApplicationErrorResponse {
    error: String,
}

pub type Result<T> = std::result::Result<T, ApplicationError>;

macro_rules! log_test_error {
    ($error:expr) => {
        #[cfg(test)]
        {
            println!("Err: {:?}", $error.to_string());
        }
    };
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        match self {
            ApplicationError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            ),
            ApplicationError::BadRequest(error) => {
                log_test_error!(error);
                (StatusCode::BAD_REQUEST, Json(json!({ "error": error })))
            }
            ApplicationError::Forbidden(error) => {
                log_test_error!(error);
                (StatusCode::FORBIDDEN, Json(json!({ "error": error })))
            }
            _ => {
                error!("Err: {}", self.to_string());

                #[cfg(test)]
                {
                    println!("Err: {:?}", self.to_string());
                }

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "Error occurred while processing the request"})),
                )
            }
        }
        .into_response()
    }
}
