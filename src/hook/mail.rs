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

use crate::hook::ActionType;
use crate::prelude::*;
use crate::CONFIGURATION;
use lazy_static::lazy_static;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use surrealdb::sql::Thing;

lazy_static! {
    pub static ref TRANSPORT: AsyncSmtpTransport<Tokio1Executor> = {
        AsyncSmtpTransport::<Tokio1Executor>::relay(CONFIGURATION.smtp_host.as_str())
            .unwrap()
            .credentials(Credentials::new(
                CONFIGURATION.smtp_username.clone(),
                CONFIGURATION.smtp_password.clone(),
            ))
            .build()
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MailState {
    Pending,
    Processing,
    Delivered,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Mail {
    id: Thing,
    recipient: String,
    #[serde(rename = "type")]
    ty: ActionType,
    state: MailState,
    locale: String,
}

#[instrument(skip_all)]
pub async fn mail_hook(connection: &DatabaseConnection) -> Result<()> {
    // collect all mail with the status "pending" and update them to "processing"
    let mails: Vec<Mail> = sql_span!(connection
        .query("SELECT * FROM mail WHERE state = $pending")
        .query("UPDATE mail SET state = $processing WHERE state = $pending")
        .bind(("pending", MailState::Pending))
        .bind(("processing", MailState::Processing))
        .await?
        .check()?
        .take(0)?);

    // send the mails
    for mail in mails {
        let id = mail.id.clone();

        match send_mail(mail, connection).await {
            Ok(()) => {}
            Err(error) => {
                #[cfg(test)]
                panic!("{:?}", error);

                error!("Error while sending mail: {}", error);

                // set the state of the failed mail to pending
                let _: Option<Mail> = connection
                    .update(id)
                    .merge(&json!({
                        "state": MailState::Pending
                    }))
                    .await?;
            }
        };
    }

    Ok(())
}
#[instrument(skip_all)]
async fn send_mail(mail: Mail, connection: &DatabaseConnection) -> Result<()> {
    let message = Message::builder()
        .from(CONFIGURATION.smtp_username.as_str().parse().unwrap())
        .to(mail.recipient.parse().unwrap())
        .subject(t!(
            format!("mail.{}.title", mail.ty.as_ref()).as_str(),
            locale = &mail.locale,
            name = &mail.recipient
        ))
        .body(t!(
            format!("mail.{}.body", mail.ty.as_ref()).as_str(),
            locale = &mail.locale,
            name = &mail.recipient
        ))
        .unwrap();

    // send the mail
    TRANSPORT.send(message).await?;
    // set the status to delivered
    let _: Option<Mail> = connection
        .update(mail.id)
        .merge(&json!({
            "state": MailState::Delivered
        }))
        .await?;

    Ok(())
}
