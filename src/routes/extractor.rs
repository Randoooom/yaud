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
use axum::response::IntoResponse;
use axum_jsonschema::JsonSchemaRejection;
use serde::Serialize;

#[derive(FromRequest, OperationIo)]
#[from_request(via(axum_jsonschema::Json), rejection(ApplicationError))]
#[aide(
    input_with = "axum_jsonschema::Json<T>",
    output_with = "axum_jsonschema::Json<T>",
    json_schema
)]
pub struct Json<T>(pub T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

impl From<JsonSchemaRejection> for ApplicationError {
    fn from(rejection: JsonSchemaRejection) -> Self {
        let message = match rejection {
            JsonSchemaRejection::Json(err) => err.to_string(),
            JsonSchemaRejection::Serde(err) => err.to_string(),
            JsonSchemaRejection::Schema(err) => {
                serde_json::to_string(&serde_json::json!({ "schema": err })).unwrap()
            }
        };

        Self::BadRequest(message)
    }
}
