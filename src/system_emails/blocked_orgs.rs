use crate::{
    bus::client::BusClient,
    models::{ApiUserRepository, BlockedOrg, Error, InternalEmail, MessageRepository},
    system_emails::send_internal_email,
};
use askama::Template;

#[derive(Template)]
#[template(path = "blocked_orgs.html")]
struct BlockedOrgsHtml<'a> {
    orgs: &'a [BlockedOrg],
    api_server_name: &'a str,
}

#[derive(Template)]
#[template(path = "blocked_orgs.txt")]
struct BlockedOrgsTxt<'a> {
    orgs: &'a [BlockedOrg],
    api_server_name: &'a str,
}

pub async fn send_blocked_orgs_email(
    user_repo: &ApiUserRepository,
    message_repo: &MessageRepository,
    bus: &BusClient,
    api_server_name: &str,
    orgs: &[BlockedOrg],
) -> Result<(), Error> {
    let recipients = user_repo.get_super_admin_emails().await?;
    if recipients.is_empty() {
        return Ok(());
    }

    let html = BlockedOrgsHtml {
        orgs,
        api_server_name,
    }
    .render()?;
    let text = BlockedOrgsTxt {
        orgs,
        api_server_name,
    }
    .render()?;

    for to in recipients {
        send_internal_email(
            message_repo,
            bus,
            3,
            3,
            InternalEmail {
                to,
                subject: "Remails organizations automatically blocked".to_string(),
                text: text.clone(),
                html: html.clone(),
                label: "blocked-orgs".parse().unwrap(),
            },
        )
        .await?;
    }

    Ok(())
}
