use std::str::FromStr;

use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "measurements")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub timestamp: DateTimeUtc,
    pub temperature: Measurement,
}

impl ActiveModelBehavior for ActiveModel {}
#[derive(Debug, Clone, PartialEq, PartialOrd, DeriveValueType)]
pub struct Measurement(f32);

impl AsRef<f32> for Measurement {
    fn as_ref(&self) -> &f32 {
        &self.0
    }
}

impl From<f32> for Measurement {
    fn from(value: f32) -> Self {
        Measurement(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MeasurementError {
    #[error("invalid format")]
    InvalidFormat,
    #[error("invalid degree value")]
    InvalidDegreeValue,
    #[error("invalid fraction value")]
    InvalidFractionValue,
}

impl FromStr for Measurement {
    type Err = MeasurementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Remove any trailing 'C'
        let s = s.trim().replace("C", "").replace("'", "");

        // Split the string into two parts
        let parts: Vec<&str> = s.split('.').collect();

        // Check if there are two parts
        if parts.len() != 2 {
            return Err(MeasurementError::InvalidFormat);
        }

        // Parse the first part as a float for the degrees
        let degrees: f32 = parts[0]
            .parse()
            .map_err(|_| MeasurementError::InvalidDegreeValue)?;

        // Parse the second part as an integer for the fraction
        let fraction: u8 = parts[1]
            .parse()
            .map_err(|_| MeasurementError::InvalidFractionValue)?;
        // Check if the fraction is more than 100
        if fraction > 100 {
            return Err(MeasurementError::InvalidFractionValue);
        }

        // Create and return the Measurement instance
        Ok(Measurement(degrees + (fraction as f32 / 100.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_measurement_with_trailing_c() {
        let result: Result<Measurement, _> = "23.45C".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(23.0 + 0.45));
    }

    #[test]
    fn test_valid_measurement_with_trailing_c_2() {
        let result: Result<Measurement, _> = "23.45'C".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(23.0 + 0.45));
    }

    #[test]
    fn test_valid_measurement_without_trailing_c() {
        let result: Result<Measurement, _> = "23.45".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(23.0 + 0.45));
    }

    #[test]
    fn test_valid_measurement_with_whitespace() {
        let result: Result<Measurement, _> = "  23.45C  ".parse();
        assert!(result.is_ok());
    }

    #[test]
    fn test_valid_measurement_fraction_1() {
        let result: Result<Measurement, _> = "10.01C".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(10.0 + 0.01));
    }

    #[test]
    fn test_valid_measurement_fraction_100() {
        let result: Result<Measurement, _> = "10.100C".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(10.0 + 1.0));
    }

    #[test]
    fn test_invalid_format_no_dot() {
        let result: Result<Measurement, _> = "2345".parse();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MeasurementError::InvalidFormat
        ));
    }

    #[test]
    fn test_invalid_format_multiple_dots() {
        let result: Result<Measurement, _> = "23.45.67".parse();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MeasurementError::InvalidFormat
        ));
    }

    #[test]
    fn test_invalid_degree_value() {
        let result: Result<Measurement, _> = "abc.45".parse();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MeasurementError::InvalidDegreeValue
        ));
    }

    #[test]
    fn test_invalid_fraction_value() {
        let result: Result<Measurement, _> = "23.abc".parse();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MeasurementError::InvalidFractionValue
        ));
    }

    #[test]
    fn test_invalid_fraction_too_large() {
        let result: Result<Measurement, _> = "23.101C".parse();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MeasurementError::InvalidFractionValue
        ));
    }

    #[test]
    fn test_negative_degrees() {
        let result: Result<Measurement, _> = "-5.50C".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Measurement(-5.0 + 0.50));
    }
}
