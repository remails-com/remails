use crate::{
    api::ApiState,
    bus::client::BusClient,
    models::{ApiUserRepository, Error, InternalEmail, MessageRepository},
    system_emails::send_internal_email,
};
use askama::Template;
use axum::extract::FromRef;
use email_address::EmailAddress;
use std::sync::Arc;
use tracing::warn;

#[derive(Template)]
#[template(path = "password_reset.html")]
struct PasswordResetHtml<'a> {
    password_reset_link: &'a str,
    name: &'a str,
}

#[derive(Template)]
#[template(path = "password_reset.txt")]
struct PasswordResetTxt<'a> {
    password_reset_link: &'a str,
    name: &'a str,
}

pub async fn send_password_reset_email(
    api_state: &ApiState,
    email_address: EmailAddress,
) -> Result<(), Error> {
    let repo = ApiUserRepository::from_ref(api_state);
    let message_repo = MessageRepository::from_ref(api_state);
    let bus = Arc::<BusClient>::from_ref(api_state);

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

    let html = PasswordResetHtml {
        password_reset_link: &link,
        name: &reset_data.user_name,
    }
    .render()?;

    let text = PasswordResetTxt {
        password_reset_link: &link,
        name: &reset_data.user_name,
    }
    .render()?;

    send_internal_email(
        &message_repo,
        &bus,
        api_state.retry_config.max_check_retries,
        api_state.retry_config.max_delivery_retries,
        InternalEmail {
            to: email_address,
            subject: "Remails password reset".to_string(),
            text,
            html,
            label: "password-reset".parse().unwrap(),
        },
    )
    .await
}
