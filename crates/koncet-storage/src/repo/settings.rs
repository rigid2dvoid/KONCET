//! Typed application settings stored as JSON values.

use std::collections::BTreeMap;

use koncet_domain::TimestampUtc;
use rusqlite::OptionalExtension;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::codec;
use crate::db::Database;
use crate::error::{StorageError, StorageResult};

/// Rejects oversized preference values.
pub const MAX_SETTING_BYTES: usize = 64 * 1024;

/// Keys that must never be used; credentials do not belong in the database.
const FORBIDDEN_KEY_PARTS: &[&str] = &["secret", "token", "password", "credential", "api_key"];

impl Database {
    pub fn get_setting_json(&self, key: &str) -> StorageResult<Option<Value>> {
        validate_key(key)?;
        let text: Option<String> = self
            .conn()
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()?;
        text.map(|text| codec::from_json(&text, "setting"))
            .transpose()
    }

    pub fn get_setting<T: DeserializeOwned>(&self, key: &str) -> StorageResult<Option<T>> {
        self.get_setting_json(key)?
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    StorageError::Corrupt(format!("invalid setting value: {error}"))
                })
            })
            .transpose()
    }

    pub fn set_setting<T: Serialize>(
        &mut self,
        key: &str,
        value: &T,
        now: TimestampUtc,
    ) -> StorageResult<()> {
        let value = serde_json::to_value(value).map_err(StorageError::Json)?;
        self.set_setting_json(key, &value, now)
    }

    pub fn set_setting_json(
        &mut self,
        key: &str,
        value: &Value,
        now: TimestampUtc,
    ) -> StorageResult<()> {
        validate_key(key)?;
        if value.is_null() {
            return Err(StorageError::Validation(
                "settings cannot store JSON null; delete the key instead".into(),
            ));
        }
        let text = serde_json::to_string(value).map_err(StorageError::Json)?;
        if text.len() > MAX_SETTING_BYTES {
            return Err(StorageError::Validation(format!(
                "setting value exceeds {MAX_SETTING_BYTES} bytes"
            )));
        }
        self.conn().execute(
            "INSERT INTO app_settings(key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at",
            rusqlite::params![key, text, now.as_millis()],
        )?;
        Ok(())
    }

    pub fn delete_setting(&mut self, key: &str) -> StorageResult<bool> {
        validate_key(key)?;
        let changed = self
            .conn()
            .execute("DELETE FROM app_settings WHERE key = ?1", [key])?;
        Ok(changed > 0)
    }

    pub fn list_settings(&self) -> StorageResult<BTreeMap<String, Value>> {
        let mut statement = self
            .conn()
            .prepare("SELECT key, value_json FROM app_settings ORDER BY key")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut settings = BTreeMap::new();
        for row in rows {
            let (key, text) = row?;
            settings.insert(key, codec::from_json(&text, "setting")?);
        }
        Ok(settings)
    }
}

/// Validates a preference key. Secrets are rejected by name.
pub fn validate_key(key: &str) -> StorageResult<()> {
    let valid = !key.is_empty()
        && key.len() <= 64
        && key.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        && key
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic());
    if !valid {
        return Err(StorageError::InvalidSettingKey(key.to_string()));
    }
    let lowered = key.to_ascii_lowercase();
    if FORBIDDEN_KEY_PARTS
        .iter()
        .any(|part| lowered.contains(part))
    {
        return Err(StorageError::InvalidSettingKey(key.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_bounded_and_secret_free() {
        assert!(validate_key("theme_mode").is_ok());
        assert!(validate_key("Theme").is_err());
        assert!(validate_key("").is_err());
        assert!(validate_key("api_token").is_err());
        assert!(validate_key(&"a".repeat(65)).is_err());
    }
}
