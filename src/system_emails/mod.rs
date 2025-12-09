use crate::{
    api::ApiState,
    bus::client::BusClient,
    models::{ApiUserRepository, Error, Label, MessageRepository},
};
use askama::Template;
use axum::extract::FromRef;
use email_address::EmailAddress;
use std::sync::Arc;
use tracing::{error, warn};

#[derive(Template)]
#[template(path = "password_reset.j2")]
struct HtmlTemplate<'a> {
    password_reset_link: &'a str,
    name: &'a str,
}

#[derive(Template)]
#[template(path = "password_reset.txt")]
struct TxtTemplate<'a> {
    password_reset_link: &'a str,
    name: &'a str,
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
) -> Result<(), Error> {
    let repo = ApiUserRepository::from_ref(api_state);

    let reset_data = match repo.initiate_password_reset(&email_address).await {
        Err(Error::NotFound(_)) => {
            warn!(
                email = email_address.as_str(),
                "Requested password reset link for non-existent account"
            );
            return Ok(());
        }
        Err(e) => return Err(e),
        Ok(ok) => ok,
    };

    let link = format!(
        "https://{}/login/password/reset/{}#{}",
        api_state.api_server_name(),
        reset_data.pw_reset_id,
        reset_data.reset_secret
    );

    let html = HtmlTemplate {
        password_reset_link: &link,
        name: &reset_data.user_name,
    }
    .render()?;

    let text = TxtTemplate {
        password_reset_link: &link,
        name: &reset_data.user_name,
    }
    .render()?;

    send_internal_email(
        api_state,
        InternalEmail {
            to: email_address,
            subject: "Remails password reset".to_string(),
            text,
            html,
            label: "password-reset".parse().unwrap(),
        },
    )
    .await?;

    Ok(())
}

async fn send_internal_email(api_state: &ApiState, email: InternalEmail) -> Result<(), Error> {
    let message_repo = MessageRepository::from_ref(api_state);
    let bus = Arc::<BusClient>::from_ref(api_state);

    let message_id = message_repo
        .create_system_email(
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
