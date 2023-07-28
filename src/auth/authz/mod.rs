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
            let result = sql_span!(connection
            .query("select $permission INSIDE ->has->permission.id as result from $account",)
            .bind(("permission", permission.to_thing()))
            .bind(("account", self.id().to_thing()))
            .await?)
            .take::<bool>((0, "result"))?;

            if result {
                Ok(())
            } else {
                Err(ApplicationError::Unauthorized)
            }
        }
    }
}

#[async_trait]
pub trait GrantPermission {
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

impl GrantPermission for Account {
    #[instrument(skip(connection))]
    async fn grant_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        sql_span!(connection
            .query("RELATE $account->has->$permission")
            .bind(("account", self.id().to_thing()))
            .bind(("permission", permission.to_thing()))
            .await?
            .check()?);

        Ok(())
    }

    #[instrument(skip(connection))]
    async fn revoke_permission(
        &self,
        permission: &Permission,
        connection: &DatabaseConnection,
    ) -> Result<()> {
        sql_span!(connection
            .query("DELETE FROM has WHERE in=$account AND out=$permission")
            .bind(("account", self.id().to_thing()))
            .bind(("permission", permission.to_thing()))
            .await?
            .check()?);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use axum::BoxError;

    #[tokio::test]
    async fn test_grants() -> Result<(), BoxError> {
        let connection = crate::database::connect().await?;
        // TODO

        Ok(())
    }
}
