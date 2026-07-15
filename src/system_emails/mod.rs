mod blocked_orgs;
mod password_reset;

pub use blocked_orgs::send_blocked_orgs_email;
pub use password_reset::send_password_reset_email;

use crate::{
    bus::client::BusClient,
    models::{Error, InternalEmail, MessageRepository},
};
use tracing::error;

async fn send_internal_email(
    message_repo: &MessageRepository,
    bus: &BusClient,
    max_check_retries: i32,
    max_delivery_retries: i32,
    email: InternalEmail,
) -> Result<(), Error> {
    let message_id = message_repo
        .create_system_email(email, max_check_retries, max_delivery_retries)
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
