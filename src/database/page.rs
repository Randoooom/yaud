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

use crate::database::DatabaseConnection;
use crate::prelude::*;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;

#[derive(Deserialize, Serialize, Debug, Clone, JsonSchema)]
pub struct Page<T>
where
    T: JsonSchema + Serialize,
{
    /// the result
    pub data: Vec<T>,
    /// the total count of pages
    pub pages: u64,
    /// the total count of elements
    pub total: u64,
    /// the offset for the next page
    pub next_page_offset: u64,
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PagingOptions {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

impl<'a> PagingOptions {
    #[instrument(skip(connection))]
    pub fn execute<P, T>(
        self,
        query: &'a str,
        bindings: &'a [P],
        connection: &'a DatabaseConnection,
    ) -> PagingRequest<'a, P, T>
    where
        T: DeserializeOwned + JsonSchema + Serialize + Send + Sync,
        P: Serialize + Send + Sync + std::fmt::Debug,
    {
        PagingRequest {
            options: self,
            query,
            bindings,
            connection,
            response: PhantomData,
        }
    }
}

impl Default for PagingOptions {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    20
}

#[derive(Debug)]
pub struct PagingRequest<'a, P, T>
where
    P: Serialize + Send + Sync,
    T: DeserializeOwned + JsonSchema + Serialize + Send + Sync,
{
    pub options: PagingOptions,
    pub query: &'a str,
    pub bindings: &'a [P],
    pub connection: &'a DatabaseConnection,
    response: PhantomData<T>,
}

impl<'a, P, T> IntoFuture for PagingRequest<'a, P, T>
where
    P: Serialize + Send + Sync,
    T: DeserializeOwned + JsonSchema + Serialize + Send + Sync + 'a,
{
    type Output = Result<Page<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    #[instrument(skip_all)]
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            // calculate offset
            let offset = (self.options.page - 1) * &self.options.page_size;
            // build the query
            let query = {
                let limit = format!("limit {} start {offset}", &self.options.page_size);

                if self.query.contains("%%%") {
                    self.query.replace("%%%", limit.as_str())
                } else {
                    format!("{} {}", self.query, limit)
                }
            };
            let count_query = format!(
                "SELECT * FROM count(({}))",
                self.query.split("%%%").next().unwrap()
            );

            // setup the database request
            let mut request = self.connection.query(count_query).query(query);
            // apply the bindings
            for binding in self.bindings.iter() {
                request = request.bind(binding)
            }

            // process the request
            let mut response = sql_span!(request.await?.check()?);
            // extract count
            let total = response
                .take::<Option<u64>>(0)?
                .ok_or(ApplicationError::InternalServerError)?;
            // parse the entries
            let data = response.take::<Vec<T>>(1)?;

            Ok(Page {
                data,
                pages: (total as f64 / self.options.page_size as f64).ceil() as u64,
                total,
                next_page_offset: offset + self.options.page_size,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::database::page::PagingOptions;
    use axum::BoxError;

    #[tokio::test]
    async fn test_paging() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?.connection;
        connection
            .query("DEFINE TABLE test SCHEMALESS;")
            .await?
            .check()?;
        let entries = 1..30;
        let mut request = connection.query("CREATE test");
        for _ in entries {
            request = request.query("CREATE test");
        }
        request.await?.check()?;

        let options = PagingOptions::default();
        let request = options.execute::<(&str, &str), serde_json::Value>(
            "SELECT * FROM test",
            &[],
            &connection,
        );
        let response = request.await?;
        assert_eq!(response.total, 30);
        assert_eq!(response.next_page_offset, 20);
        assert_eq!(response.data.len(), 20);

        Ok(())
    }
}
