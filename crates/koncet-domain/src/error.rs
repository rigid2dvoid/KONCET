//! Typed domain errors and validation issues.
//!
//! Errors carry stable codes and structured targets, never host paths or secrets.
//! The UI layer owns human-facing copy and may replace `message` with localised text.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable machine-readable error class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Validation,
    Conflict,
    NotFound,
    Constraint,
    Persistence,
    Internal,
}

impl ErrorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Conflict => "conflict",
            Self::NotFound => "not_found",
            Self::Constraint => "constraint",
            Self::Persistence => "persistence",
            Self::Internal => "internal",
        }
    }
}

/// An exhaustive domain error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DomainError {
    /// A field, path, or reference is invalid.
    Validation {
        /// Stable code such as `out_of_range` or `invalid_format`.
        code: String,
        /// JSON Pointer-like location, for example `/service/env/APP_ENV`.
        target: String,
        message: String,
    },
    /// A state transition or uniqueness rule was violated.
    Conflict { code: String, message: String },
    /// A referenced entity does not exist.
    NotFound { entity: String, id: String },
    /// A documented limit was exceeded.
    Constraint { code: String, message: String },
    /// Storage failed; `retryable` tells the UI whether to offer a retry.
    Persistence {
        code: String,
        message: String,
        retryable: bool,
    },
    /// An invariant was violated. Never surfaced as a panic across FFI.
    Internal { code: String, message: String },
}

impl DomainError {
    pub fn validation(
        code: impl Into<String>,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Validation {
            code: code.into(),
            target: target.into(),
            message: message.into(),
        }
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Conflict {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn constraint(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Constraint {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Internal {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Validation { .. } => ErrorKind::Validation,
            Self::Conflict { .. } => ErrorKind::Conflict,
            Self::NotFound { .. } => ErrorKind::NotFound,
            Self::Constraint { .. } => ErrorKind::Constraint,
            Self::Persistence { .. } => ErrorKind::Persistence,
            Self::Internal { .. } => ErrorKind::Internal,
        }
    }

    /// Stable code used by the UI to choose copy and behaviour.
    pub fn code(&self) -> &str {
        match self {
            Self::Validation { code, .. }
            | Self::Conflict { code, .. }
            | Self::Constraint { code, .. }
            | Self::Persistence { code, .. }
            | Self::Internal { code, .. } => code,
            Self::NotFound { .. } => "not_found",
        }
    }

    /// Structured parameters for localisation; contains no secrets.
    pub fn params(&self) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();
        match self {
            Self::Validation { target, .. } => {
                params.insert("target".into(), target.clone());
            }
            Self::NotFound { entity, id } => {
                params.insert("entity".into(), entity.clone());
                params.insert("id".into(), id.clone());
            }
            Self::Persistence { retryable, .. } => {
                params.insert("retryable".into(), retryable.to_string());
            }
            _ => {}
        }
        params
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation {
                code,
                target,
                message,
            } => {
                write!(f, "validation[{code}] at {target}: {message}")
            }
            Self::Conflict { code, message } => write!(f, "conflict[{code}]: {message}"),
            Self::NotFound { entity, id } => write!(f, "not found: {entity} {id}"),
            Self::Constraint { code, message } => write!(f, "constraint[{code}]: {message}"),
            Self::Persistence { code, message, .. } => {
                write!(f, "persistence[{code}]: {message}")
            }
            Self::Internal { code, message } => write!(f, "internal[{code}]: {message}"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Error,
    Warning,
}

/// A single validation finding, suitable for inline display in the editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    /// JSON Pointer-like path. Empty string means the whole document.
    pub path: String,
    pub message: String,
    pub severity: IssueSeverity,
}

impl ValidationIssue {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Error,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            path: path.into(),
            message: message.into(),
            severity: IssueSeverity::Warning,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == IssueSeverity::Error
    }
}

/// Result of validating a document. Blocking issues are errors; warnings do not
/// prevent a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    pub fn error(
        &mut self,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(ValidationIssue::error(code, path, message));
    }

    pub fn warning(
        &mut self,
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.push(ValidationIssue::warning(code, path, message));
    }

    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(ValidationIssue::is_error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| issue.is_error())
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| !issue.is_error())
    }

    pub fn first_error(&self) -> Option<&ValidationIssue> {
        self.issues.iter().find(|issue| issue.is_error())
    }

    /// Converts the first blocking issue into a typed error, if any.
    pub fn to_error(&self) -> Option<DomainError> {
        self.first_error().map(|issue| {
            DomainError::validation(
                issue.code.clone(),
                issue.path.clone(),
                issue.message.clone(),
            )
        })
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let errors = self.errors().count();
        let warnings = self.warnings().count();
        write!(f, "{errors} error(s), {warnings} warning(s)")
    }
}

/// Adds a human-readable validation failure to a `Result`.
pub trait ValidationExt<T> {
    fn validate_with(self, report: &ValidationReport) -> Result<T, DomainError>;
}

impl<T> ValidationExt<T> for Option<T> {
    fn validate_with(self, report: &ValidationReport) -> Result<T, DomainError> {
        match (self, report.to_error()) {
            (Some(value), _) => Ok(value),
            (None, Some(error)) => Err(error),
            (None, None) => Err(DomainError::internal(
                "validation_report_empty",
                "validation failed without a blocking issue",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_errors_block() {
        let mut report = ValidationReport::new();
        report.warning("firewall_allow_missing", "/firewall", "no allow rule");
        assert!(report.is_valid());
        report.error("out_of_range", "/service/port", "port is zero");
        assert!(!report.is_valid());
        assert_eq!(report.to_error().unwrap().code(), "out_of_range");
    }

    #[test]
    fn errors_serialise_without_secrets_and_with_codes() {
        let error = DomainError::conflict("active_run_exists", "a run is already active");
        let value = serde_json::to_value(&error).unwrap();
        assert_eq!(value["kind"], "conflict");
        assert_eq!(value["code"], "active_run_exists");
        assert_eq!(error.kind(), ErrorKind::Conflict);
    }
}
