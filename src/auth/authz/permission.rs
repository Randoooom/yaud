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
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Permission {
    id: Id,
}

impl Permission {
    pub fn id(&self) -> String {
        self.id.to_string().replace('⟨', "").replace('⟩', "")
    }

    pub fn to_thing(&self) -> Thing {
        Thing::from(("permission", self.id.id.to_string().as_str()))
    }
}

macro_rules! permissions {
    ($(($ident:ident, $name:expr)),*) => {
        lazy_static::lazy_static! {
            $(
                pub static ref $ident: Permission = {
                    Permission {
                        id: Id::new(("permission", $name))
                    }
                };
            )*

            pub static ref PERMISSION_NONE: Permission = Permission {
                                                id: Id::new(("permission", "none")),
                                            };

            pub static ref PERMISSIONS: Vec<&'static Permission> = {
                vec![
                        $(
                            $ident.deref(),
                        )*
                    ]
            };
        }
    };
}

permissions!((TASK_REQUEST_VIEW, "task.request.view"));

#[derive(Clone, Debug)]
pub struct PermissionHandler<'a> {
    connection: &'a DatabaseConnection,
}

impl<'a> From<&'a DatabaseConnection> for PermissionHandler<'a> {
    fn from(connection: &'a DatabaseConnection) -> Self {
        Self { connection }
    }
}

impl<'a> IntoFuture for PermissionHandler<'a> {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            // fetch all currently available permissions
            let permissions: Vec<Permission> = self.connection.select("permission").await?;

            let mut query = String::new();
            PERMISSIONS
                .iter()
                .filter(|permission| !permissions.iter().any(|p| p.id().eq(&permission.id())))
                .for_each(|permission| {
                    query.push_str(
                        format!("CREATE type::thing('permission', '{}');", &permission.id.id)
                            .as_str(),
                    )
                });
            if !query.is_empty() {
                // execute the query
                self.connection.query(query.as_str()).await?.check()?;
            }

            Ok(())
        })
    }
}
