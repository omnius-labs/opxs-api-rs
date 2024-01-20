use std::sync::Arc;

use aws_lambda_events::event::sqs::SqsEvent;
use chrono::Duration;
use core_base::{clock::SystemClockUtc, random_bytes::RandomBytesProviderImpl, tsid::TsidProviderImpl};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use core_cloud::aws::{secrets::SecretsReaderImpl, ses::SesSenderImpl};
use opxs_email_send::{EmailSendJobBatchSqsMessage, EmailSendJobRepository, Executor};

mod shared;

use shared::*;

async fn handler_sub(ms: &[EmailSendJobBatchSqsMessage]) -> Result<(), Error> {
    let info = AppInfo::new()?;
    info!("info: {}", info);

    let sdk_config = aws_config::load_from_env().await;
    let secret_reader = Box::new(SecretsReaderImpl {
        client: aws_sdk_secretsmanager::Client::new(&sdk_config),
    });
    let conf = AppConfig::load(&info.mode, secret_reader).await?;
    let db = Arc::new(
        PgPoolOptions::new()
            .max_connections(100)
            .idle_timeout(Some(Duration::minutes(15).to_std().unwrap()))
            .connect(&conf.postgres.url)
            .await?,
    );
    let system_clock = Arc::new(SystemClockUtc {});
    let tsid_provider = Arc::new(TsidProviderImpl::new(SystemClockUtc, RandomBytesProviderImpl, 16));

    let executor = Executor {
        email_send_job_repository: Arc::new(EmailSendJobRepository {
            db: db.clone(),
            system_clock,
            tsid_provider,
        }),
        ses_sender: Arc::new(SesSenderImpl {
            client: aws_sdk_sesv2::Client::new(&aws_config::load_from_env().await),
            configuration_set_name: Some(conf.ses.configuration_set_name),
        }),
    };
    executor.execute(ms).await?;

    Ok(())
}

async fn handler(event: LambdaEvent<serde_json::Value>) -> Result<(), Error> {
    let (event, _context) = event.into_parts();

    let mut ms: Vec<EmailSendJobBatchSqsMessage> = Vec::new();

    if let Ok(event) = serde_json::from_value::<SqsEvent>(event.clone()) {
        info!("sqs event");
        for v in event.records.into_iter().flat_map(|n| n.body).collect::<Vec<_>>() {
            let m = serde_json::from_str::<EmailSendJobBatchSqsMessage>(&v)?;
            ms.push(m);
        }
    } else {
        info!("raw event");
        let m = serde_json::from_value::<EmailSendJobBatchSqsMessage>(event)?;
        ms.push(m);
    }

    info!("messages: {:?}", ms);
    handler_sub(&ms).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    if cfg!(debug_assertions) {
        tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).with_target(false).init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_target(false)
            .json()
            .init();
    }

    info!("----- start -----");
    run(service_fn(handler)).await
}
