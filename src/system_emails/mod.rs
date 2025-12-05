use crate::{
    api::ApiState,
    bus::client::BusClient,
    models,
    models::{Label, MessageRepository},
};
use askama::Template;
use axum::extract::FromRef;
use email_address::EmailAddress;
use std::sync::Arc;
use tracing::error;

#[derive(Template)]
#[template(path = "password_reset.j2")]
struct HtmlTemplate {
    password_reset_link: String,
}

#[derive(Template)]
#[template(path = "password_reset.txt")]
struct TxtTemplate {
    password_reset_link: String,
}

struct InternalEmail {
    to: EmailAddress,
    subject: String,
    text: String,
    html: String,
    label: Label,
}

pub async fn send_password_reset_email(
    api_state: &ApiState,
    email_address: EmailAddress,
) -> Result<(), models::Error> {
    // TODO implement password reset logic
    send_internal_email(
        api_state,
        InternalEmail {
            to: email_address,
            subject: "test".to_string(),
            text: "test".to_string(),
            html: "test".to_string(),
            label: "password-reset".parse().unwrap(),
        },
    )
    .await?;

    Ok(())
}

async fn send_internal_email(
    api_state: &ApiState,
    email: InternalEmail,
) -> Result<(), models::Error> {
    let message_repo = MessageRepository::from_ref(api_state);
    let bus = Arc::<BusClient>::from_ref(api_state);

    let message_id = message_repo
        .create_internal(
            email.to,
            email.subject,
            email.text,
            email.html,
            email.label,
            api_state.retry_config.max_automatic_retries,
        )
        .await?;

    match message_repo.get_ready_to_send(message_id).await {
        Ok(bus_message) => {
            bus.try_send(&bus_message).await;
        }
        Err(e) => {
            error!(message_id = message_id.to_string(), "{e:?}");
        }
    }

    Ok(())
}
