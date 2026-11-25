//! Application-layer errors.
//!
//! Every variant maps to a stable code that survives the FFI boundary. Host
//! paths, SQL text, and stack traces never appear in messages.

use std::fmt;

use koncet_domain::{DomainError, ValidationIssue, ValidationReport};
use koncet_storage::StorageError;

/// Result alias for application use cases.
pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    /// A domain invariant or state transition rejected the request.
    Domain(DomainError),
    /// One or more validation findings blocked the request.
    Validation(ValidationReport),
    /// Persistence failed.
    Storage(StorageError),
    /// A product-level conflict such as an existing active run.
    Conflict { code: &'static str, message: String },
    /// A requested entity does not exist.
    NotFound(&'static str),
    /// An unexpected internal condition. Never used for user input.
    Internal { code: &'static str, message: String },
}

impl AppError {
    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::Conflict {
            code,
            message: message.into(),
        }
    }

    pub fn internal(code: &'static str, message: impl Into<String>) -> Self {
        Self::Internal {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(entity: &'static str) -> Self {
        Self::NotFound(entity)
    }

    /// Stable machine-readable code.
    pub fn code(&self) -> &str {
        match self {
            Self::Domain(error) => error.code(),
            Self::Validation(_) => "validation_failed",
            Self::Storage(error) => error.code(),
            Self::Conflict { code, .. } | Self::Internal { code, .. } => code,
            Self::NotFound(_) => "not_found",
        }
    }

    /// Safe, human-readable text. The UI owns final copy.
    pub fn message(&self) -> String {
        match self {
            Self::Domain(error) => error.to_string(),
            Self::Validation(report) => report
                .first_error()
                .map(|issue| {
                    if issue.path.is_empty() {
                        issue.message.clone()
                    } else {
                        format!("{}: {}", issue.path, issue.message)
                    }
                })
                .unwrap_or_else(|| report.to_string()),
            Self::Storage(error) => error.to_string(),
            Self::Conflict { message, .. } | Self::Internal { message, .. } => message.clone(),
            Self::NotFound(entity) => format!("{entity} was not found"),
        }
    }

    pub fn retryable(&self) -> bool {
        match self {
            Self::Domain(DomainError::Persistence { retryable, .. }) => *retryable,
            Self::Storage(error) => error.retryable(),
            _ => false,
        }
    }

    /// Validation findings, when the request failed validation.
    pub fn issues(&self) -> &[ValidationIssue] {
        match self {
            Self::Validation(report) => &report.issues,
            _ => &[],
        }
    }

    /// Normalises every failure into the domain error shape used by the bridge.
    pub fn to_domain_error(&self) -> DomainError {
        match self {
            Self::Domain(error) => error.clone(),
            Self::Validation(_) => self
                .issues()
                .iter()
                .find(|issue| issue.is_error())
                .map(|issue| {
                    DomainError::validation(
                        issue.code.clone(),
                        issue.path.clone(),
                        issue.message.clone(),
                    )
                })
                .unwrap_or_else(|| {
                    DomainError::internal("validation_failed", "validation failed without a code")
                }),
            Self::Storage(error) => error.to_domain(),
            Self::Conflict { code, message } => {
                DomainError::conflict((code).to_string(), message.clone())
            }
            Self::NotFound(entity) => DomainError::NotFound {
                entity: (*entity).to_string(),
                id: String::new(),
            },
            Self::Internal { code, message } => {
                DomainError::internal((*code).to_string(), message.clone())
            }
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for AppError {}

impl From<DomainError> for AppError {
    fn from(value: DomainError) -> Self {
        Self::Domain(value)
    }
}

impl From<ValidationReport> for AppError {
    fn from(value: ValidationReport) -> Self {
        Self::Validation(value)
    }
}

impl From<StorageError> for AppError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        let error = AppError::conflict("active_run_exists", "already running");
        assert_eq!(error.code(), "active_run_exists");
        assert!(!error.retryable());

        let mut report = ValidationReport::new();
        report.error("required", "/name", "name is required");
        let error = AppError::from(report);
        assert_eq!(error.code(), "validation_failed");
        assert_eq!(error.message(), "/name: name is required");
        assert_eq!(error.issues().len(), 1);
    }
}
