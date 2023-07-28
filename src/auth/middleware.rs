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

use crate::auth::session::Session;
use crate::database::definitions::account::Account;
use crate::prelude::*;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;

async fn require_session<B>(
    State(state): State<ApplicationState>,
    jar: CookieJar,
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    match jar.get("session") {
        Some(cookie) => {
            let session_id = cookie.value();
            let connection = state.connection();
            let extensions = request.extensions_mut();

            // verify the session
            if let Ok(session) = Session::validate_session(session_id, connection).await {
                // fetch the account
                let account: Account = connection.select(session.target()).await.unwrap();

                // TODO: permissions

                extensions.insert(account);
                extensions.insert(session);

                return next.run(request).await;
            };

            ApplicationError::Unauthorized.into_response()
        }
        None => ApplicationError::Unauthorized.into_response(),
    }
}
