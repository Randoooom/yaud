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
use crate::prelude::*;
use std::future::{Future, IntoFuture};
use std::pin::Pin;

pub mod permission;

#[async_trait]
pub trait Authorize {
    async fn has_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()>;
}

#[async_trait]
impl Authorize for Account {
    #[instrument(skip(connection))]
    async fn has_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        if permission.eq(&PERMISSION_NONE) {
            Ok(())
        } else {
            let result = sql_span!(
                connection
                    .query("select * from fn::has_permission($account, $permission)",)
                    .bind(("permission", permission.to_thing()))
                    .bind(("account", self.id().to_thing()))
                    .await?
            )
            .take::<Option<bool>>(0)?
            .unwrap();

            if result {
                Ok(())
            } else {
                Err(ApplicationError::Unauthorized)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct GrantPermission<'a> {
    permission: &'a Permission,
    target: &'a Id,
    connection: &'a DatabaseConnection,
}

impl<'a> From<(&'a Permission, &'a Id, &'a DatabaseConnection)> for GrantPermission<'a> {
    fn from(value: (&'a Permission, &'a Id, &'a DatabaseConnection)) -> Self {
        Self {
            permission: value.0,
            target: value.1,
            connection: value.2,
        }
    }
}

impl<'a> IntoFuture for GrantPermission<'a> {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            sql_span!(self
                .connection
                .query("RELATE $account->has->$permission")
                .bind(("account", self.target.to_thing()))
                .bind(("permission", self.permission.to_thing()))
                .await?
                .check()?);

            Ok(())
        })
    }
}

#[derive(Clone, Debug)]
pub struct RevokePermission<'a> {
    permission: &'a Permission,
    target: &'a Id,
    connection: &'a DatabaseConnection,
}

impl<'a> From<(&'a Permission, &'a Id, &'a DatabaseConnection)> for RevokePermission<'a> {
    fn from(value: (&'a Permission, &'a Id, &'a DatabaseConnection)) -> Self {
        Self {
            permission: value.0,
            target: value.1,
            connection: value.2,
        }
    }
}

impl<'a> IntoFuture for RevokePermission<'a> {
    type Output = Result<()>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + Sync + 'a>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            sql_span!(self
                .connection
                .query("DELETE FROM has WHERE in=$account AND out=$permission")
                .bind(("account", self.target.to_thing()))
                .bind(("permission", self.permission.to_thing()))
                .await?
                .check()?);

            Ok(())
        })
    }
}

#[async_trait]
pub trait WritePermissions {
    async fn grant_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()>;

    async fn revoke_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()>;
}

#[async_trait]
impl WritePermissions for Account {
    #[instrument(skip(connection))]
    async fn grant_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        GrantPermission::from((permission, self.id(), connection)).await
    }

    #[instrument(skip(connection))]
    async fn revoke_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        RevokePermission::from((permission, self.id(), connection)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::authz::{Authorize, WritePermissions};
    use crate::database::definitions::account::WriteAccount;
    use crate::prelude::TASK_REQUEST_VIEW;
    use axum::BoxError;

    #[tokio::test]
    async fn test_grants() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?.connection;
        let account = WriteAccount::from(&connection)
            .set_first_name(Some("first"))
            .set_last_name(Some("last"))
            .set_mail(Some("test@test.de"))
            .set_password(Some("password".to_owned()))
            .to_owned()
            .await?;

        assert!(account
            .has_permission(&TASK_REQUEST_VIEW, &connection)
            .await
            .is_err());
        account
            .grant_permission(&TASK_REQUEST_VIEW, &connection)
            .await?;
        assert!(account
            .has_permission(&TASK_REQUEST_VIEW, &connection)
            .await
            .is_ok());
        account
            .revoke_permission(&TASK_REQUEST_VIEW, &connection)
            .await?;
        assert!(account
            .has_permission(&TASK_REQUEST_VIEW, &connection)
            .await
            .is_err());

        Ok(())
    }
}
