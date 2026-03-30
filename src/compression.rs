use std::time::Duration;

use chrono::{Datelike, Timelike};
use sea_orm::{ActiveValue::Set, DeleteResult, entity::prelude::*, query::*};
use tokio::sync::watch;
use tokio::time::interval;

use crate::AppState;
use crate::entity::measurement::{Entity as MeasurementEntity, Model as MeasurementModel};

pub async fn spawn_compression_task(state: AppState, mut shutdown_rx: watch::Receiver<bool>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_hours(6));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    match compress_old_measurements(&state.db).await {
                        Ok(count) => {
                            if count > 0 {
                                tracing::info!("compressed {} hourly records", count);
                            }
                        }
                        Err(e) => {
                            tracing::error!("failed to compress measurements: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("shutting down background compression task");
                        break;
                    }
                }
            }
        }
    });
}

async fn compress_old_measurements(db: &DatabaseConnection) -> Result<usize, CompressionError> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    tracing::debug!("compression cutoff: {}", cutoff);

    let old_measurements_exist = MeasurementEntity::find()
        .filter(crate::entity::measurement::Column::Timestamp.lt(cutoff))
        .count(db)
        .await?;

    if old_measurements_exist == 0 {
        tracing::debug!("no measurements older than 30 days to compress");
        return Ok(0);
    }

    tracing::info!(
        "found {} measurements older than 30 days",
        old_measurements_exist
    );

    let deleted_count = db
        .transaction::<_, usize, CompressionError>(|tx| {
            Box::pin(async move {
                let old_records: Vec<MeasurementModel> = MeasurementEntity::find()
                    .filter(crate::entity::measurement::Column::Timestamp.lt(cutoff))
                    .all(tx)
                    .await?;

                if old_records.is_empty() {
                    return Ok(0);
                }

                let hourly_averages = compute_hourly_averages(&old_records);

                if hourly_averages.is_empty() {
                    tracing::debug!("no hourly records to insert after grouping");
                    return Ok(0);
                }

                tracing::info!("computed {} hourly averages", hourly_averages.len());
                for (timestamp, temperature) in &hourly_averages {
                    let model = crate::entity::measurement::ActiveModel {
                        timestamp: Set(*timestamp),
                        temperature: Set(temperature.clone().into()),
                        ..Default::default()
                    };
                    model.insert(tx).await?;
                }

                tracing::debug!("inserted {} hourly records", hourly_averages.len());

                let delete_result: DeleteResult = MeasurementEntity::delete_many()
                    .filter(crate::entity::measurement::Column::Timestamp.lt(cutoff))
                    .exec(tx)
                    .await?;

                tracing::debug!(
                    "deleted {} original measurements",
                    delete_result.rows_affected
                );

                Ok(delete_result.rows_affected as usize)
            })
        })
        .await
        .map_err(|e| CompressionError::Transaction(e.to_string()))?;

    Ok(deleted_count)
}

fn compute_hourly_averages(measurements: &[MeasurementModel]) -> Vec<(DateTimeUtc, f32)> {
    use std::collections::BTreeMap;

    let mut hour_groups: BTreeMap<String, (DateTimeUtc, Vec<f32>)> = BTreeMap::new();

    for m in measurements {
        let hour_bucket = format!(
            "{:04}-{:02}-{:02} {:02}:00:00",
            m.timestamp.year(),
            m.timestamp.month(),
            m.timestamp.day(),
            m.timestamp.hour()
        );

        let temp_value = *m.temperature.as_ref();

        hour_groups
            .entry(hour_bucket)
            .and_modify(|entry| {
                entry.1.push(temp_value);
                if m.timestamp < entry.0 {
                    entry.0 = m.timestamp;
                }
            })
            .or_insert((m.timestamp, vec![temp_value]));
    }

    hour_groups
        .into_iter()
        .map(|(_hour_bucket, (timestamp, temps))| {
            let avg_temp = temps.iter().sum::<f32>() / temps.len() as f32;
            (timestamp, avg_temp)
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("transaction error: {0}")]
    Transaction(String),
}
