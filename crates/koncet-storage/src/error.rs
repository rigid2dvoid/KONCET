//! Typed storage errors. No SQL text, host paths, or secrets cross this boundary.

use std::fmt;

use koncet_domain::DomainError;

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Corrupt(String),
    Conflict(String),
    Validation(String),
    NotFound(&'static str),
    InvalidSettingKey(String),
    LockHeld { pid: u32 },
    LockStale { pid: u32 },
    SchemaTooNew(i64),
    MigrationChecksum { version: i64 },
    NotInitialised,
}

impl StorageError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Sqlite(error) => match error {
                rusqlite::Error::SqliteFailure(failure, _)
                    if failure.code == rusqlite::ErrorCode::DatabaseBusy =>
                {
                    "db_busy"
                }
                _ => "db_error",
            },
            Self::Io(_) => "io_error",
            Self::Json(_) => "corrupt_data",
            Self::Corrupt(_) => "corrupt_data",
            Self::Conflict(_) => "conflict",
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::InvalidSettingKey(_) => "invalid_setting_key",
            Self::LockHeld { .. } => "lock_held",
            Self::LockStale { .. } => "lock_stale",
            Self::SchemaTooNew(_) => "schema_too_new",
            Self::MigrationChecksum { .. } => "migration_checksum",
            Self::NotInitialised => "db_not_initialised",
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(self.code(), "db_busy")
    }

    pub fn to_domain(&self) -> DomainError {
        let message = self.to_string();
        match self {
            Self::Corrupt(_) | Self::Json(_) => DomainError::Persistence {
                code: self.code().to_string(),
                message,
                retryable: false,
            },
            Self::SchemaTooNew(_) | Self::MigrationChecksum { .. } | Self::NotInitialised => {
                DomainError::Persistence {
                    code: self.code().to_string(),
                    message,
                    retryable: false,
                }
            }
            Self::Conflict(_) => DomainError::conflict(self.code(), message),
            Self::Validation(_) => DomainError::validation("invalid_data", "", message),
            Self::NotFound(_) => DomainError::conflict(self.code(), message),
            Self::LockHeld { .. } | Self::LockStale { .. } => {
                DomainError::conflict(self.code(), message)
            }
            _ => DomainError::Persistence {
                code: self.code().to_string(),
                message,
                retryable: self.retryable(),
            },
        }
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "database operation failed: {error}"),
            Self::Io(error) => write!(f, "storage I/O failed: {error}"),
            Self::Json(error) => write!(f, "stored JSON is invalid: {error}"),
            Self::Corrupt(message) => write!(f, "stored data is inconsistent: {message}"),
            Self::Conflict(message) => write!(f, "storage conflict: {message}"),
            Self::Validation(message) => write!(f, "invalid data: {message}"),
            Self::NotFound(entity) => write!(f, "{entity} was not found"),
            Self::InvalidSettingKey(key) => write!(f, "setting key is not allowed: {key}"),
            Self::LockHeld { pid } => {
                write!(
                    f,
                    "another KONCET instance is using this data directory (pid {pid})"
                )
            }
            Self::LockStale { pid } => write!(
                f,
                "a stale KONCET lock from pid {pid} is present; confirm removal and retry"
            ),
            Self::SchemaTooNew(version) => {
                write!(
                    f,
                    "database schema version {version} is newer than this app"
                )
            }
            Self::MigrationChecksum { version } => {
                write!(f, "migration {version} checksum does not match")
            }
            Self::NotInitialised => write!(f, "database has not been initialised"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}

impl From<std::io::Error> for StorageError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<StorageError> for rusqlite::Error {
    fn from(value: StorageError) -> Self {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(value))
    }
}

pub type StorageResult<T> = Result<T, StorageError>;
