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

use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
#[cfg(not(test))]
use version_compare::{Cmp, Version};

pub type DatabaseConnection = Surreal<Client>;

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection: DatabaseConnection,
    pub database: String,
    pub namespace: String,
}

pub async fn connect(options: Option<(&str, &str)>) -> Result<ConnectionInfo> {
    // establish the connection
    let client: Surreal<Client> = Surreal::new::<Ws>(&CONFIGURATION.surrealdb_endpoint).await?;
    info!("Established connection to surrealdb");

    // authenticate
    client
        .signin(Root {
            username: CONFIGURATION.surrealdb_username.as_str(),
            password: CONFIGURATION.surrealdb_password.as_str(),
        })
        .await?;
    info!("Authenticated with surrealdb");

    #[cfg(not(test))]
    let database = "yaud".to_owned();
    #[cfg(not(test))]
    let namespace = "production".to_owned();

    #[cfg(test)]
    let (namespace, database) = if let Some(options) = options {
        (options.0.to_string(), options.1.to_string())
    } else {
        ("test".to_owned(), nanoid::nanoid!())
    };

    #[cfg(test)]
    println!(
        "Connected with database {:?} in namespace \"test\"",
        database
    );

    client
        .use_ns(namespace.as_str())
        .use_db(database.as_str())
        .await?;

    // perform the migrations
    #[cfg(not(test))]
    migrate(&client, env!("CARGO_PKG_VERSION"), Vec::new()).await?;
    // execute the up queries
    client
        .query(include_str!("./up.surrealql"))
        .await?
        .check()?;
    info!("Initiated tables");

    Ok(ConnectionInfo {
        database,
        namespace,
        connection: client,
    })
}

#[cfg(not(test))]
pub async fn migrate(
    client: &DatabaseConnection,
    current_version: &'static str,
    migrations: Vec<(&'static str, &'static str)>,
) -> Result<()> {
    // initiate the migration table and fetch possibly already existing records
    let mut responses = client
        .query(
            "DEFINE TABLE migration SCHEMALESS;
            DEFINE FIELD version     on TABLE migration TYPE string;
            DEFINE FIELD created_at  on TABLE migration TYPE datetime DEFAULT time::now();",
        )
        .query("SELECT version, created_at FROM migration ORDER BY created_at DESC LIMIT 1")
        .await?
        .check()?;
    // take the last as response, which contains the last migrated version
    let last = responses.take::<Option<String>>((1, "version"))?;

    if let Some(last) = last {
        // only proceed if the  last version is not equal to the current version
        if !last.as_str().eq(current_version) {
            // iterate through the given migrations
            for (version, migration) in migrations {
                if Version::from(last.as_str())
                    .unwrap()
                    .compare_to(Version::from(current_version).unwrap(), Cmp::Lt)
                {
                    info!("Executing surrealdb migration to {version}");
                    // execute the migration query and mark it as done
                    client
                        .query(migration)
                        .query("CREATE migration SET version = $version")
                        .bind(("version", version))
                        .await?
                        .check()?;
                }
            }
        }
    } else {
        // insert the current version as the last version
        client
            .query("CREATE migration SET version = $version")
            .bind(("version", current_version))
            .await?
            .check()?;
    }

    Ok(())
}

#[macro_export]
macro_rules! sql_span {
    ($expr: expr) => {{
        let span = info_span!("Surrealdb Request");
        let _ = span.enter();
        $expr
    }};
    ($expr: expr, $title: expr) => {{
        let span = info_span!(concat!("Surrealdb Request: ", $title));
        let _ = span.enter();
        $expr
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use lazy_static::lazy_static;
    use std::ops::Deref;
    use std::time::Duration;
    use surrealdb::opt::auth::Scope;
    use surrealdb::sql::Thing;

    lazy_static! {
        pub static ref TEST_MAIL: String = std::env::var("TEST_MAIL").unwrap();
        pub static ref TEST_MAIL2: String = std::env::var("TEST_MAIL2").unwrap();
    }

    async fn root(options: &ConnectionInfo) -> Result<DatabaseConnection> {
        let info = connect(Some((
            options.namespace.as_str(),
            options.database.as_str(),
        )))
        .await?;
        Ok(info.connection)
    }

    async fn wait_for_mail(from: i64, mail: &str) -> Result<()> {
        let tag = mail.split(".").nth(1).unwrap().split("@").next().unwrap();

        match reqwest::Client::new()
            .get("https://api.testmail.app/api/json")
            .query(&[
                ("apikey", std::env::var("TEST_MAIL_KEY").unwrap()),
                ("namespace", std::env::var("TEST_MAIL_NAMESPACE").unwrap()),
                ("tag", tag.to_string()),
                ("timestamp_from", from.to_string()),
                ("livequery", "true".to_string()),
            ])
            .timeout(Duration::from_secs(5))
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(ApplicationError::Unauthorized),
        }
    }

    #[tokio::test]
    async fn test_signup() -> Result<()> {
        let info = connect(None).await?;
        let connection = &info.connection;

        connection
            .signup(Scope {
                namespace: info.namespace.as_str(),
                database: info.database.as_str(),
                scope: "account",
                params: &json!({
                    "first": "first",
                    "last": "last",
                    "mail": TEST_MAIL.as_str(),
                    "password": "password"
                }),
            })
            .await?;

        connection
            .signin(Scope {
                namespace: info.namespace.as_str(),
                database: info.database.as_str(),
                scope: "account",
                params: &json!({
                    "mail": TEST_MAIL.as_str(),
                    "password": "password"
                }),
            })
            .await?;

        assert!(connection
            .signin(Scope {
                namespace: info.namespace.as_str(),
                database: info.database.as_str(),
                scope: "account",
                params: &json!({
                    "mail": TEST_MAIL.as_str(),
                    "password": "passwrd"
                }),
            })
            .await
            .is_err());

        Ok(())
    }

    async fn init(info: &ConnectionInfo) -> Result<DatabaseConnection> {
        let connection = info.connection.clone();
        let root = connection.clone();

        connection
            .signup(Scope {
                namespace: info.namespace.as_str(),
                database: info.database.as_str(),
                scope: "account",
                params: &json!({
                    "first": "first",
                    "last": "last",
                    "mail": *TEST_MAIL,
                    "password": "password"
                }),
            })
            .await?;

        root.query(
            "LET $account = SELECT * FROM account WHERE mail = $mail;\
                 FOR $permission IN $permissions {\
                    RELATE $account->has->(type::thing(\"permission\", $permission));
                 };",
        )
        .bind(("mail", TEST_MAIL.as_str()))
        .await?
        .check()?;

        Ok(connection)
    }

    async fn second(info: &ConnectionInfo) -> Result<DatabaseConnection> {
        let connection = connect(Some((info.namespace.as_str(), info.database.as_str())))
            .await?
            .connection;

        connection
            .signup(Scope {
                namespace: info.namespace.as_str(),
                database: info.database.as_str(),
                scope: "account",
                params: &json!({
                    "first": "second",
                    "last": "last",
                    "mail": TEST_MAIL2.as_str(),
                    "password": "password"
                }),
            })
            .await?;

        Ok(connection)
    }

    #[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
    struct Account {
        id: Thing,
        first_name: String,
        last_name: String,
        mail: String,
        password: String,
        options: AccountOptions,
        updated_at: String,
        created_at: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
    struct AccountOptions {
        notify_task_request_created: bool,
        notify_task_created: bool,
        notify_message_created: bool,
        notify_state_updated: bool,
    }

    async fn fetch_account(connection: &DatabaseConnection) -> Result<Account> {
        let account: Option<Account> = connection
            .query("SELECT * FROM account WHERE mail = $mail")
            .bind(("mail", TEST_MAIL.deref()))
            .await?
            .take(0)?;

        Ok(account.unwrap())
    }

    #[tokio::test]
    async fn test_account_update() -> Result<()> {
        let connection = init(&connect(None).await?).await?;
        let account = fetch_account(&connection).await?;

        let updated: Account = connection
            .update(account.id.clone())
            .merge(&json! ({
                "first_name": "test",
                "last_name": "test",
                "options": {
                    "notify_task_request_created": true,
                    "notify_task_created": true,
                    "notify_message_created": true,
                    "notify_state_updated": true,
                }
            }))
            .await?
            .unwrap();
        assert_eq!("test", updated.first_name.as_str());
        assert_eq!("test", updated.last_name.as_str());
        // ref https://github.com/surrealdb/surrealdb/issues/2161
        // assert_eq!(false, updated.options.notify_task_request_created);
        // assert_eq!(false, updated.options.notify_task_created);
        assert_eq!(true, updated.options.notify_state_updated);
        assert_eq!(true, updated.options.notify_message_created);

        assert!(connection
            .update::<Option<Account>>(account.id.clone())
            .merge(&json! ({
                "mail": "test"
            }))
            .await
            .is_err());

        Ok(())
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    struct TaskRequest {
        id: Thing,
        title: String,
        description: String,
        due: Option<String>,
        state: String,
    }

    async fn create_task_request(connection: &DatabaseConnection) -> Result<TaskRequest> {
        let request: Vec<TaskRequest> = connection
            .create("task_request")
            .content(&json!({
                "title": "title",
                "description": "description"
            }))
            .await?;

        Ok(request.first().unwrap().clone())
    }

    #[tokio::test]
    async fn test_task_request_creation() -> Result<()> {
        let info = connect(None).await?;
        let admin = init(&info).await?;
        let client = second(&info).await?;
        let root = root(&info).await?;

        let from = Utc::now().timestamp();
        create_task_request(&client).await?;
        assert!(wait_for_mail(from, TEST_MAIL.as_str()).await.is_err());
        crate::hook::mail::mail_hook(&root).await?;

        admin
            .query(
                "UPDATE account SET options.notify_task_request_created = true WHERE mail = $mail",
            )
            .bind(("mail", TEST_MAIL.as_str()))
            .await?
            .check()?;
        let from = Utc::now().timestamp();
        create_task_request(&client).await?;
        crate::hook::mail::mail_hook(&root).await?;
        assert!(wait_for_mail(from, TEST_MAIL.as_str()).await.is_ok());

        Ok(())
    }

    #[derive(Deserialize, Serialize, Clone, Debug)]
    pub struct Message {
        id: Thing,
        content: String,
        reference: Thing,
        author: Thing,
        internal: bool,
    }

    #[tokio::test]
    async fn test_send_message() -> Result<()> {
        let info = connect(None).await?;
        let client = second(&info).await?;
        let admin = init(&info).await?;

        // as record() types are not supported in the rust-sdk this will always fail
        // let request = create_task_request(&client).await?;
        // let message: Vec<Message> = client
        //     .create("message")
        //     .content(&json!({
        //         "content": "test",
        //         "reference": &request.id
        //     }))
        //     .await?;
        // let message = message.first().unwrap();
        //
        // let messages: Vec<Message> = admin.select("message").await?;
        // assert_eq!(1, messages.len());
        //
        // let _: Vec<Message> = admin
        //     .create("message")
        //     .content(&json!({
        //         "content": "test",
        //         "reference": &request.id,
        //         "internal": true
        //     }))
        //     .await?;
        // let messages: Vec<Message> = client.select("message").await?;
        // assert_eq!(1, messages.len());

        Ok(())
    }
}
