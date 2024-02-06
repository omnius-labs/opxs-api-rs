use std::{path::Path, sync::Arc};

use aws_lambda_events::sqs::SqsEvent;
use chrono::Duration;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use core_base::{clock::SystemClockUtc, random_bytes::RandomBytesProviderImpl, tsid::TsidProviderImpl};
use core_cloud::aws::s3::S3ClientImpl;

use opxs_base::{AppConfig, AppInfo};
use opxs_image_convert::{Executor, ImageConvertJobRepository, ImageConvertJobSqsMessage};

const APPLICATION_NAME: &str = "opxs-batch-image-convert";

async fn handler_sub(job_ids: &[String]) -> Result<(), Error> {
    let info = AppInfo::new()?;
    info!("info: {}", info);

    let conf = AppConfig::load(APPLICATION_NAME, &info.mode).await?;
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
        image_convert_job_repository: Arc::new(ImageConvertJobRepository {
            db: db.clone(),
            system_clock,
            tsid_provider,
        }),
        s3_client: Arc::new(S3ClientImpl {
            client: aws_sdk_s3::Client::new(&aws_config::load_from_env().await),
            bucket: conf.image_convert.s3.bucket,
        }),
    };
    executor.execute(job_ids).await?;

    Ok(())
}

async fn handler(event: LambdaEvent<serde_json::Value>) -> Result<(), Error> {
    let (event, _context) = event.into_parts();

    let mut job_ids: Vec<String> = Vec::new();

    if let Ok(event) = serde_json::from_value::<SqsEvent>(event.clone()) {
        info!("sqs event");
        for v in event.records.into_iter().flat_map(|n| n.body).collect::<Vec<_>>() {
            info!("{:?}", v);
            let m = serde_json::from_str::<ImageConvertJobSqsMessage>(&v)?;
            for v in m.records {
                let p = Path::new(&v.s3.object.key);
                let job_id = p.file_name().ok_or(anyhow::anyhow!("file name is not found"))?;
                job_ids.push(job_id.to_str().unwrap().to_string());
            }
        }
    } else {
        info!("raw event");
        let key = event
            .get("key")
            .ok_or(anyhow::anyhow!("key is not found"))?
            .as_str()
            .ok_or(anyhow::anyhow!("key is not string"))?
            .to_string();
        let p = Path::new(&key);
        let job_id = p.file_name().ok_or(anyhow::anyhow!("file name is not found"))?;
        job_ids.push(job_id.to_str().unwrap().to_string());
    }

    info!("messages: {:?}", job_ids);
    handler_sub(&job_ids).await?;

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
