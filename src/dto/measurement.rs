use chrono::Utc;

#[derive(serde::Serialize)]
pub struct MeasurementResponse {
    pub temperature: f32,
    pub timestamp: chrono::DateTime<Utc>,
}

impl From<crate::entity::measurement::Model> for MeasurementResponse {
    fn from(measurement: crate::entity::measurement::Model) -> Self {
        Self {
            temperature: measurement.temperature.as_ref().clone(),
            timestamp: measurement.timestamp,
        }
    }
}
