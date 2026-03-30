use std::fmt::{Display, Formatter};

use crate::{dto::measurement::MeasurementResponse, entity::measurement::Entity as Measurement};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use axum_thiserror_tracing::IntoResponse;
use chrono::{DateTime, TimeZone as _, Utc};
use sea_orm::{
    ColumnTrait as _, DbErr, EntityOrSelect, EntityTrait, PaginatorTrait as _, QueryFilter as _,
};
use serde::Deserialize;

use crate::AppState;
#[derive(Debug, thiserror::Error, IntoResponse)]
pub enum ApiError {
    #[error("Not found")]
    #[status(StatusCode::NOT_FOUND)]
    NotFound,

    #[error("Database error")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    DatabaseError(#[from] DbErr),
}

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    page: u64,
    #[serde(default = "default_per_page")]
    per_page: u64,
    #[serde(default)]
    start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    end_date: Option<DateTime<Utc>>,
}

fn default_page() -> u64 {
    0
}
fn default_per_page() -> u64 {
    100
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/measurements", get(get_measurements))
        .route("/chart", get(get_graph))
}
pub async fn get_measurements(
    State(state): State<AppState>,
    Query(page): Query<Pagination>,
) -> Result<axum::Json<Vec<MeasurementResponse>>, ApiError> {
    let mut query = Measurement::find();

    if let Some(start) = page.start_date {
        query = query.filter(crate::entity::measurement::Column::Timestamp.gte(start));
    }
    if let Some(end) = page.end_date {
        query = query.filter(crate::entity::measurement::Column::Timestamp.lte(end));
    }

    let measurements = query
        .order_by_id_desc()
        .paginate(&state.db, page.per_page.min(100))
        .fetch_page(page.page)
        .await?;
    Ok(Json(measurements.into_iter().map(|v| v.into()).collect()))
}

#[derive(Deserialize, Clone, Copy)]
pub enum GraphSpans {
    #[serde(rename = "5m")]
    FiveMinute,
    #[serde(rename = "30m")]
    ThirtyMinute,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "6h")]
    SixHours,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1m")]
    OneMonth,
    #[serde(rename = "1y")]
    OneYear,
}

impl Display for GraphSpans {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GraphSpans::FiveMinute => "5m",
            GraphSpans::ThirtyMinute => "30m",
            GraphSpans::OneHour => "1h",
            GraphSpans::SixHours => "6h",
            GraphSpans::OneDay => "1d",
            GraphSpans::OneWeek => "1w",
            GraphSpans::OneMonth => "1m",
            GraphSpans::OneYear => "1y",
        };
        write!(f, "{}", s)
    }
}

#[derive(Deserialize)]
pub struct GraphQuery {
    pub span: GraphSpans,
}

pub async fn get_graph(
    State(state): State<AppState>,
    Query(page): Query<GraphQuery>,
) -> Result<axum::Json<Vec<MeasurementResponse>>, ApiError> {
    Ok(Json(get_chart_data(page.span, &state).await?))
}

pub async fn get_chart_data(
    span: GraphSpans,
    state: &AppState,
) -> Result<Vec<MeasurementResponse>, ApiError> {
    let now = chrono::Utc::now();
    let (span_duration, bucket_seconds) = match span {
        GraphSpans::FiveMinute => (chrono::Duration::minutes(5), 1), // no aggregation
        GraphSpans::ThirtyMinute => (chrono::Duration::minutes(30), 10), // 10s buckets  → ~180 points
        GraphSpans::OneHour => (chrono::Duration::hours(1), 30), // 30s buckets  → ~120 points
        GraphSpans::SixHours => (chrono::Duration::hours(6), 120), // 2m buckets   → ~180 points
        GraphSpans::OneDay => (chrono::Duration::days(1), 600),  // 10m buckets  → ~144 points
        GraphSpans::OneWeek => (chrono::Duration::weeks(1), 3600), // 1h buckets   → ~168 points
        GraphSpans::OneMonth => (chrono::Duration::days(30), 21600), // 6h buckets   → ~120 points
        GraphSpans::OneYear => (chrono::Duration::days(365), 86400), // 1d buckets   → ~365 points
    };

    let start = now - span_duration;

    let measurements = Measurement::find()
        .filter(crate::entity::measurement::Column::Timestamp.gte(start))
        .order_by_id_desc()
        .all(&state.db)
        .await?;

    if bucket_seconds == 1 {
        // No aggregation for short spans
        return Ok(measurements.into_iter().map(|v| v.into()).collect());
    }

    // Group into time buckets and average
    let mut buckets: std::collections::BTreeMap<i64, (f32, u32)> =
        std::collections::BTreeMap::new();

    for m in measurements {
        let ts = m.timestamp.timestamp();
        let bucket = (ts / bucket_seconds) * bucket_seconds;
        let entry = buckets.entry(bucket).or_insert((0.0, 0));
        entry.0 += m.temperature.as_ref();
        entry.1 += 1;
    }

    Ok(buckets
        .into_iter()
        .rev()
        .map(|(bucket_ts, (sum, count))| {
            let avg = sum / count as f32;
            MeasurementResponse {
                timestamp: chrono::DateTime::from_timestamp(bucket_ts, 0).unwrap(),
                temperature: avg,
            }
        })
        .collect())
}
