//! Row decoding helpers. Stored text is treated as untrusted on read.

use std::fmt;
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::{StorageError, StorageResult};

pub(crate) fn parse_id<T>(value: &str, field: &str) -> StorageResult<T>
where
    T: FromStr<Err = uuid::Error>,
{
    value
        .parse::<T>()
        .map_err(|_| StorageError::Corrupt(format!("{field} is not a valid identifier")))
}

pub(crate) fn parse_state<T>(value: &str, field: &str) -> StorageResult<T>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    value
        .parse::<T>()
        .map_err(|error| StorageError::Corrupt(format!("invalid {field}: {error}")))
}

pub(crate) fn parse_enum<T>(value: &str, field: &str) -> StorageResult<T>
where
    T: DeserializeOwned,
{
    let text = serde_json::to_string(value).expect("strings are serialisable");
    serde_json::from_str::<T>(&text)
        .map_err(|_| StorageError::Corrupt(format!("invalid {field}: {value}")))
}

pub(crate) fn to_json<T: Serialize>(value: &T) -> StorageResult<String> {
    serde_json::to_string(value).map_err(StorageError::Json)
}

pub(crate) fn from_json<T: DeserializeOwned>(text: &str, field: &str) -> StorageResult<T> {
    serde_json::from_str(text)
        .map_err(|error| StorageError::Corrupt(format!("invalid {field} JSON: {error}")))
}

pub(crate) fn from_json_value(value: Value, field: &str) -> StorageResult<Value> {
    if value.is_object() {
        Ok(value)
    } else {
        Err(StorageError::Corrupt(format!(
            "{field} must be a JSON object"
        )))
    }
}

pub(crate) fn non_negative(value: i64, field: &str) -> StorageResult<u64> {
    u64::try_from(value).map_err(|_| StorageError::Corrupt(format!("{field} must not be negative")))
}

pub(crate) fn positive(value: i64, field: &str) -> StorageResult<u32> {
    u32::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| StorageError::Corrupt(format!("{field} must be a positive integer")))
}

pub(crate) fn positive_or_zero(value: i64, field: &str) -> StorageResult<u32> {
    u32::try_from(value).map_err(|_| StorageError::Corrupt(format!("{field} is out of range")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use koncet_domain::{RunStatus, Severity};

    #[test]
    fn state_enums_round_trip() {
        let running: RunStatus = parse_state("running", "run status").unwrap();
        assert_eq!(running, RunStatus::Running);
        let debug: Severity = parse_enum("debug", "severity").unwrap();
        assert_eq!(debug, Severity::Debug);
    }

    #[test]
    fn invalid_stored_values_become_corruption_errors() {
        assert!(parse_state::<RunStatus>("nope", "run status").is_err());
        assert!(parse_enum::<Severity>("loud", "severity").is_err());
        assert!(parse_id::<koncet_domain::RunId>("nope", "run id").is_err());
    }
}
