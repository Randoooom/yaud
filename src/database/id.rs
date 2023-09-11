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

use crate::error::ApplicationError;
use schemars::gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use surrealdb::opt::{IntoResource, Resource};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, PartialEq)]
pub struct Id {
    pub table: String,
    pub id: String,
}

impl From<Thing> for Id {
    fn from(thing: Thing) -> Self {
        Self {
            table: thing.tb,
            id: thing.id.to_string(),
        }
    }
}

impl TryFrom<(&str, &str)> for Id {
    type Error = ApplicationError;

    fn try_from((force, id): (&str, &str)) -> Result<Self, Self::Error> {
        let mut split = id.split(':');
        let table = split
            .next()
            .ok_or(ApplicationError::BadRequest("invalid id".to_owned()))?;
        // for security reasons we can't allow every table
        if !table.eq(force) {
            return Err(ApplicationError::Unauthorized);
        }

        let id = split
            .next()
            .ok_or(ApplicationError::BadRequest("invalid id".to_owned()))?;

        Ok(Self {
            table: table.to_string(),
            id: id.to_string(),
        })
    }
}

impl Id {
    pub fn new((table, id): (&str, &str)) -> Self {
        Self {
            table: table.to_string(),
            id: id.to_string(),
        }
    }

    pub fn to_thing(&self) -> Thing {
        Thing::from((self.table.as_str(), self.id.as_str()))
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_value = serde_json::value::Value::deserialize(deserializer).unwrap();

        if let Some(string) = raw_value.as_str() {
            let mut split = string.split(':');
            let table = split
                .next()
                .ok_or(serde::de::Error::custom("Invalid id format"))?
                .to_string();
            let id = split
                .next()
                .ok_or(serde::de::Error::custom("Invalid id format"))?
                .to_string();

            return Ok(Self { table, id });
        }

        if raw_value.is_object() {
            let thing = serde_json::from_value::<Thing>(raw_value).unwrap();
            return Ok(Self {
                table: thing.tb,
                id: thing.id.to_string(),
            });
        }

        Err(serde::de::Error::custom("Invalid datatype"))
    }
}

impl ToString for Id {
    fn to_string(&self) -> String {
        format!("{}:{}", &self.table, &self.id)
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl JsonSchema for Id {
    fn schema_name() -> String {
        "Id".to_owned()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            format: Some("string".to_string()),
            ..Default::default()
        }
        .into()
    }
}

impl<R> IntoResource<Option<R>> for &Id {
    fn into_resource(self) -> surrealdb::Result<Resource> {
        Ok(Resource::RecordId(self.to_thing()))
    }
}
