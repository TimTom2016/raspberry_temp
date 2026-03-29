use std::time::Duration;

use crate::{AppState, entity::measurement::Measurement};
use sea_orm::ActiveModelTrait as _;
use sea_orm::ActiveValue::Set;
use tokio::{process::Command, sync::watch, time::interval};

#[derive(Debug, thiserror::Error)]
pub enum MeasurementError {
    #[error("failed to execute vcgencmd")]
    Io(#[from] std::io::Error),
    #[error("failed to parse measurement value : {0}")]
    Parse(#[from] crate::entity::measurement::MeasurementError),
    #[error("failed to decode from utf-8 : {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub async fn take_measurement() -> Result<Measurement, MeasurementError> {
    let raw_value = Command::new("vcgencmd")
        .arg("measure_temp")
        .output()
        .await?;
    let output = String::from_utf8(raw_value.stdout)?;
    let value = output.trim().replace("temp=", "").replace("'C", "");
    Ok(value.parse()?)
}

pub async fn spawn_measurement_task(
    state: AppState,
    mut shutdown_rx: watch::Receiver<bool>,
    interval_secs: u64,
) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match take_measurement().await {
                        Ok(measurement) => {
                            tracing::info!("took measurement: {:?}", measurement);
                            let model = crate::entity::measurement::ActiveModel {
                                timestamp: Set(chrono::Utc::now()),
                                temperature: Set(measurement),
                                ..Default::default()
                            };
                            if let Err(e) = model.insert(&state.db).await {
                                tracing::error!("failed to insert measurement: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("failed to take measurement: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("shutting down background measurement task");
                        break;
                    }
                }
            }
        }
    });
}
