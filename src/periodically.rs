use crate::{
    MoneyBird,
    bus::client::BusClient,
    models::{self, InviteRepository, MessageRepository},
    moneybird,
};
use chrono::Duration;
use sqlx::PgPool;
use std::error::Error;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

pub struct Periodically {
    message_repository: MessageRepository,
    invite_repository: InviteRepository,
    moneybird: MoneyBird,
    bus_client: BusClient,
}

pub fn run_periodically<F, E, Fut>(task: F, period: Duration, cancel: CancellationToken)
where
    F: Fn() -> Fut + Send + 'static,
    E: Error,
    Fut: Future<Output = Result<(), E>> + Send,
{
    tokio::spawn(async move {
        loop {
            select!(
                _ = cancel.cancelled() => {
                    tracing::info!("Task cancelled");
                    return;
                },
                _ = tokio::time::sleep(period.to_std().unwrap()) => {
                    task().await.unwrap();
                }
            )
        }
    });
}

impl Periodically {
    pub async fn new(pool: PgPool, bus_client: BusClient) -> Result<Self, moneybird::Error> {
        Ok(Self {
            message_repository: MessageRepository::new(pool.clone()),
            invite_repository: InviteRepository::new(pool.clone()),
            moneybird: MoneyBird::new(pool).await?,
            bus_client,
        })
    }

    /// Retry all messages that are ready to be retried
    pub async fn retry_messages(&self) -> Result<(), models::Error> {
        debug!("Retrying messages");
        let messages = self
            .message_repository
            .find_messages_ready_for_retry()
            .await?;

        for message_id in messages {
            tracing::info!(message_id = message_id.to_string(), "Retrying message");
            match self.message_repository.get_ready_to_send(message_id).await {
                Ok(bus_message) => {
                    self.bus_client.try_send(&bus_message).await;
                }
                Err(e) => {
                    error!(message_id = message_id.to_string(), "{e:?}");
                }
            }
        }

        Ok(())
    }

    /// Clean up invites which have been expired for more than a day
    pub async fn clean_up_invites(&self) -> Result<(), models::Error> {
        self.invite_repository
            .remove_expired_before(chrono::Utc::now() - Duration::days(1))
            .await
    }

    /// Reset quotas for all organizations where the quota is ready to be reset
    pub async fn reset_all_quotas(&self) -> Result<(), moneybird::Error> {
        self.moneybird.reset_all_quotas().await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        Environment, HandlerConfig,
        bus::{client::BusMessage, server::Bus},
        handler::{Handler, RetryConfig, dns::DnsResolver},
        models::{MessageId, MessageStatus},
        test::{TestProjects, random_port},
    };
    use chrono::Duration;
    use mailcrab::TestMailServerHandle;
    use std::{collections::HashSet, net::Ipv4Addr, sync::Arc};
    use tokio::select;
    use tokio_util::sync::CancellationToken;

    #[sqlx::test(fixtures(
        path = "./fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "smtp_credentials",
            "messages",
            "k8s_nodes"
        )
    ))]
    async fn retry_sending_messages(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle {
            token,
            rx: mut mailcrab_rx,
        } = mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let bus_port = Bus::spawn_random_port().await;
        let bus_client = BusClient::new(bus_port, "localhost".to_owned()).unwrap();
        let config = HandlerConfig {
            allow_plain: true,
            domain: "test".to_owned(),
            resolver: DnsResolver::mock("localhost", mailcrab_port),
            environment: Environment::Development,
            retry: RetryConfig {
                delay: Duration::minutes(60),
                max_automatic_retries: 3,
            },
            spf_include: "include:spf.remails.net".to_owned(),
        };
        let handler = Handler::new(
            pool.clone(),
            Arc::new(config),
            bus_client.clone(),
            CancellationToken::new(),
        )
        .await;
        handler.spawn();

        let periodically = Periodically::new(pool.clone(), bus_client.clone())
            .await
            .unwrap();

        let mut stream = bus_client.receive().await.unwrap();

        let message_repo = MessageRepository::new(pool.clone());

        let message_held_id = "10d5ad5f-04ae-489b-9f5a-f5d7e73bc12a".parse().unwrap();
        let message_reattempt_id = "c1e03226-8aad-42a9-8c43-380a5b25cb79".parse().unwrap();
        let message_out_of_attempts = "458ed4ab-e0e0-4a18-8462-d98d038ad5ed".parse().unwrap();
        let message_on_timeout = "2b7ca359-18da-4d90-90c5-ed43f7944585".parse().unwrap();

        let (org_id, project_id) = TestProjects::Org1Project1.get_ids();

        let get_message_status = async |id: MessageId| {
            message_repo
                .find_by_id(org_id, project_id, id)
                .await
                .unwrap()
                .status()
                .to_owned()
        };

        assert_eq!(
            get_message_status(message_held_id).await,
            MessageStatus::Held
        );
        assert_eq!(
            get_message_status(message_reattempt_id,).await,
            MessageStatus::Reattempt
        );
        assert_eq!(
            get_message_status(message_out_of_attempts).await,
            MessageStatus::Reattempt
        );
        assert_eq!(
            get_message_status(message_on_timeout).await,
            MessageStatus::Reattempt
        );

        periodically.retry_messages().await.unwrap();
        BusClient::wait_for_attempt(2, &mut stream).await;

        // Await exactly 4 message at mailcrab (two email with two recipients each)
        for _ in 0..4 {
            mailcrab_rx.recv().await.unwrap();
        }

        assert_eq!(
            get_message_status(message_held_id).await,
            MessageStatus::Delivered
        );
        assert_eq!(
            get_message_status(message_reattempt_id,).await,
            MessageStatus::Delivered
        );
        assert_eq!(
            get_message_status(message_out_of_attempts).await,
            MessageStatus::Reattempt
        );
        assert_eq!(
            get_message_status(message_on_timeout).await,
            MessageStatus::Reattempt
        );

        bus_client
            .send(&BusMessage::EmailReadyToSend(
                message_out_of_attempts,
                "127.0.0.1".parse().unwrap(),
            ))
            .await
            .unwrap();
        bus_client
            .send(&BusMessage::EmailReadyToSend(
                message_on_timeout,
                "127.0.0.1".parse().unwrap(),
            ))
            .await
            .unwrap();

        BusClient::wait_for_attempt(2, &mut stream).await;
        // Await 4 message at mailcrab (two email with two recipients each)
        for _ in 0..4 {
            mailcrab_rx.recv().await.unwrap();
        }

        assert_eq!(
            get_message_status(message_out_of_attempts).await,
            MessageStatus::Delivered
        );
        assert_eq!(
            get_message_status(message_on_timeout).await,
            MessageStatus::Delivered
        );
    }

    #[sqlx::test(fixtures(
        path = "./fixtures",
        scripts(
            "organizations",
            "projects",
            "org_domains",
            "proj_domains",
            "smtp_credentials",
            "messages",
            "k8s_nodes"
        )
    ))]
    async fn quotas_retries(pool: PgPool) {
        let mailcrab_port = random_port();
        let TestMailServerHandle {
            token,
            rx: mut mailcrab_rx,
        } = mailcrab::development_mail_server(Ipv4Addr::new(127, 0, 0, 1), mailcrab_port).await;
        let _drop_guard = token.drop_guard();

        let org_id = TestProjects::Org1Project1.org_id();

        let bus_port = Bus::spawn_random_port().await;
        let bus_client = BusClient::new(bus_port, "localhost".to_owned()).unwrap();
        let config = HandlerConfig {
            allow_plain: true,
            domain: "test".to_owned(),
            resolver: DnsResolver::mock("localhost", mailcrab_port),
            retry: RetryConfig {
                delay: Duration::minutes(60),
                max_automatic_retries: 3,
            },
            environment: Environment::Development,
            spf_include: "include:spf.remails.net".to_owned(),
        };
        let handler = Handler::new(
            pool.clone(),
            Arc::new(config),
            bus_client.clone(),
            CancellationToken::new(),
        )
        .await;
        handler.spawn();

        tokio::time::sleep(core::time::Duration::from_secs(1)).await;

        let periodically = Periodically::new(pool.clone(), bus_client).await.unwrap();
        periodically.retry_messages().await.unwrap();

        let mut senders = HashSet::new();
        loop {
            select! {
                Ok(recv) = mailcrab_rx.recv() => {
                    senders.insert(recv.envelope_from.as_str().to_string());
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    break
                },
            }
        }

        assert!(senders.contains("email-held@test-org-1-project-1.com"));
        assert!(senders.contains("email-reattempt-2@test-org-1-project-1.com"));
        assert_eq!(senders.len(), 2);

        let remaining = sqlx::query_scalar!(
            r#"
            SELECT total_message_quota - used_message_quota as "remaining!" FROM organizations WHERE id = $1
            "#,
            *org_id
        )
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(remaining, 799);
    }
}
