use crate::{dto::measurement::MeasurementResponse, entity::measurement::Entity as Measurement};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use axum_thiserror_tracing::IntoResponse;
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait as _, DbErr, EntityTrait, PaginatorTrait as _, QueryFilter as _};
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
    Router::new().route("/measurements", get(get_measurements))
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
