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
use crate::database::definitions::task::request::{
    EditTaskRequest, TaskRequest, TaskRequestState, WriteTaskRequest, WriteTaskRequestRequest,
};
use crate::database::page::PagingOptions;
use crate::prelude::*;
use aide::axum::routing::{get_with, post_with, put_with};
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Extension;

pub fn router(state: ApplicationState) -> ApiRouter {
    ApiRouter::new()
        .api_route(
            "/",
            post_with(create_request, create_request_docs)
                .layer(require_session!(state, PERMISSION_NONE)),
        )
        .api_route(
            "/",
            get_with(get_request_page, get_request_page_docs)
                .layer(require_session!(state, TASK_REQUEST_VIEW)),
        )
        .api_route(
            "/:id",
            put_with(put_request, put_request_docs).layer(require_session!(state, PERMISSION_NONE)),
        )
        .api_route(
            "/with/:state",
            get_with(
                get_request_page_with_state,
                get_request_page_with_state_docs,
            )
            .layer(require_session!(state, TASK_REQUEST_VIEW)),
        )
        .with_state(state)
}

async fn create_request(
    Extension(account): Extension<Account>,
    State(state): State<ApplicationState>,
    Json(data): Json<WriteTaskRequestRequest>,
) -> Result<(StatusCode, Json<TaskRequest>)> {
    let request = WriteTaskRequest::from(state.connection())
        .with_request(data)
        .set_customer(Some(Relation::ForeignKey(account.id().clone())))
        .to_owned()
        .await?;

    Ok((StatusCode::CREATED, Json(request)))
}

fn create_request_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Create a new task request")
        .summary("Create a new task request")
        .response::<201, Json<TaskRequest>>()
}

async fn get_request_page(
    State(state): State<ApplicationState>,
    Query(data): Query<PagingOptions>,
) -> Result<Json<Page<TaskRequest>>> {
    let page = data
        .execute::<(&str, &str), TaskRequest>(
            "SELECT * FROM task_request %%% FETCH customer",
            &[],
            state.connection(),
        )
        .await?;

    Ok(Json(page))
}

fn get_request_page_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Obtain a page from all TaskRequests")
        .summary("Obtain a page of TaskRequests")
        .response::<200, Json<Page<TaskRequest>>>()
}

async fn get_request_page_with_state(
    State(state): State<ApplicationState>,
    Query(data): Query<PagingOptions>,
    Path(task_request_state): Path<TaskRequestState>,
) -> Result<Json<Page<TaskRequest>>> {
    let page = data
        .execute::<_, TaskRequest>(
            "SELECT * FROM task_request WHERE state=$state %%% FETCH customer",
            &[("state", task_request_state)],
            state.connection(),
        )
        .await?;

    Ok(Json(page))
}

fn get_request_page_with_state_docs(transform: TransformOperation) -> TransformOperation {
    transform
        .description("Obtain a page from all TaskRequests matching the given state")
        .summary("Obtain a page of TaskRequests with a specific state")
        .response::<200, Json<Page<TaskRequest>>>()
}

async fn put_request(
    State(state): State<ApplicationState>,
    Extension(connection): Extension<DatabaseConnection>,
    Path(id): Path<String>,
    Json(data): Json<EditTaskRequest>,
) -> Result<Json<TaskRequest>> {
    let id = Id::try_from(("task_request", id.as_str()))?;

    println!(
        "{:?}",
        state
            .connection()
            .query("SELECT * FROM $issuer")
            .await?
            .take::<Option<Account>>(0)
    );
    println!(
        "{:?}",
        connection
            .query("SELECT * FROM $issuer")
            .await?
            .take::<Option<Account>>(0)
    );

    // only allow this if the account is either the owner of the request or has the permission to do it
    // This part is completely covered by the database backend and we just need to map the err here to 401
    let request = data
        .to_writer(&connection)
        .set_target(Some(&id))
        .to_owned()
        .await
        .map_err(|_| ApplicationError::Unauthorized)?;

    Ok(Json(request))
}

fn put_request_docs(transform: TransformOperation) -> TransformOperation {
    transform
}

#[cfg(test)]
mod tests {
    use crate::database::definitions::task::request::{
        TaskRequest, TaskRequestState, WriteTaskRequest,
    };
    use crate::prelude::{Page, Relation};
    use crate::tests::TestSuite;
    use axum::http::StatusCode;
    use axum::BoxError;
    use chrono::Utc;

    #[tokio::test]
    async fn test_create() -> Result<(), BoxError> {
        let suite = TestSuite::init().await?;
        suite.authorize_default().await;

        let response = suite
            .client()
            .post("/task/request")
            .json(&json! {{
                "title": "title",
                "description": "description",
                "due": Utc::now()
            }})
            .send()
            .await;

        assert_eq!(StatusCode::CREATED, response.status());
        let request = response.json::<TaskRequest>().await;

        let fetched: Option<TaskRequest> = suite.connection().select(request.id()).await?;
        assert!(fetched.is_some());
        assert_eq!(request, fetched.unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_get() -> Result<(), BoxError> {
        let suite = TestSuite::init().await?;
        suite.authorize_default().await;

        let request = WriteTaskRequest::from(suite.connection())
            .set_title(Some("title".to_owned()))
            .set_description(Some("description".to_owned()))
            .set_due(Some(Utc::now()))
            .set_state(Some(TaskRequestState::Received))
            .set_customer(Some(Relation::ForeignKey(suite.account().id().clone())))
            .to_owned()
            .await?;

        let response = suite.client().get("/task/request").send().await;
        assert_eq!(StatusCode::OK, response.status());

        let page = response.json::<Page<TaskRequest>>().await;
        assert_eq!(1, page.total);
        assert_eq!(1, page.data.len());
        assert_eq!(&request, page.data.first().unwrap());

        Ok(())
    }
}
