//! Bridge errors with stable codes and safe messages.
//!
//! Errors never contain SQL, host paths, stack traces, or secrets. Rust panics
//! are converted at the boundary and logged with a diagnostic id that can be
//! quoted in a bug report.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use koncet_app::AppError;
use koncet_domain::{DomainError, ValidationIssue};

use crate::dto::{ValidationIssueDto, ValidationReportDto};

static DIAGNOSTIC_COUNTER: AtomicU64 = AtomicU64::new(1);

fn diagnostic_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let counter = DIAGNOSTIC_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{millis:x}-{counter:x}")
}

/// A typed failure crossing the bridge. Dart catches this object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    pub target: String,
    pub retryable: bool,
    pub diagnostic_id: String,
    pub issues: Vec<ValidationIssueDto>,
}

impl BridgeError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        target: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            target: target.into(),
            retryable,
            diagnostic_id: diagnostic_id(),
            issues: Vec::new(),
        }
    }

    pub fn not_initialised() -> Self {
        Self::new(
            "not_initialised",
            "KONCET has not finished starting up",
            "",
            false,
        )
    }

    pub(crate) fn invalid_id(entity: &str) -> Self {
        Self::new("invalid_id", "the identifier is not valid", entity, false)
    }

    pub(crate) fn panic(payload: Box<dyn std::any::Any + Send>) -> Self {
        let id = diagnostic_id();
        let detail = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_string());
        tracing::error!(diagnostic_id = %id, panic = %detail, "panic at the bridge boundary");
        Self {
            code: "internal_error".to_string(),
            message: "an internal error occurred; the action can be retried".to_string(),
            target: String::new(),
            retryable: false,
            diagnostic_id: id,
            issues: Vec::new(),
        }
    }

    pub(crate) fn from_app(error: AppError) -> Self {
        match error {
            AppError::Domain(domain) => Self::from_domain(domain),
            AppError::Validation(report) => {
                let dto: ValidationReportDto = report.into();
                let first = dto.issues.iter().find(|issue| issue.severity == "error");
                Self {
                    code: "validation_failed".to_string(),
                    message: first
                        .map(|issue| {
                            if issue.path.is_empty() {
                                issue.message.clone()
                            } else {
                                format!("{}: {}", issue.path, issue.message)
                            }
                        })
                        .unwrap_or_else(|| "the request is not valid".to_string()),
                    target: first.map(|issue| issue.path.clone()).unwrap_or_default(),
                    retryable: false,
                    diagnostic_id: diagnostic_id(),
                    issues: dto.issues,
                }
            }
            AppError::Storage(error) => {
                Self::new(error.code(), error.to_string(), "", error.retryable())
            }
            AppError::Conflict { code, message } => Self::new(code, message, "", false),
            AppError::NotFound(entity) => Self::new(
                "not_found",
                format!("{entity} was not found"),
                entity,
                false,
            ),
            AppError::Internal { code, message } => {
                let id = diagnostic_id();
                tracing::error!(diagnostic_id = %id, code = %code, "internal application failure");
                Self {
                    code: code.to_string(),
                    message,
                    target: String::new(),
                    retryable: false,
                    diagnostic_id: id,
                    issues: Vec::new(),
                }
            }
        }
    }

    fn from_domain(error: DomainError) -> Self {
        let retryable = matches!(
            error,
            DomainError::Persistence {
                retryable: true,
                ..
            }
        );
        let target = match &error {
            DomainError::Validation { target, .. } => target.clone(),
            DomainError::NotFound { entity, .. } => entity.clone(),
            _ => String::new(),
        };
        let message = match &error {
            DomainError::Validation { message, .. }
            | DomainError::Conflict { message, .. }
            | DomainError::Constraint { message, .. }
            | DomainError::Persistence { message, .. }
            | DomainError::Internal { message, .. } => message.clone(),
            DomainError::NotFound { entity, id } => format!("{entity} {id} was not found"),
        };
        let issues = match &error {
            DomainError::Validation {
                code,
                target,
                message,
            } => vec![ValidationIssueDto {
                code: code.clone(),
                path: target.clone(),
                message: message.clone(),
                severity: "error".to_string(),
            }],
            _ => Vec::new(),
        };
        Self {
            code: error.code().to_string(),
            message,
            target,
            retryable,
            diagnostic_id: diagnostic_id(),
            issues,
        }
    }
}

impl From<AppError> for BridgeError {
    fn from(value: AppError) -> Self {
        Self::from_app(value)
    }
}

impl From<ValidationIssue> for ValidationIssueDto {
    fn from(value: ValidationIssue) -> Self {
        Self {
            code: value.code,
            path: value.path,
            message: value.message,
            severity: match value.severity {
                koncet_domain::IssueSeverity::Error => "error".to_string(),
                koncet_domain::IssueSeverity::Warning => "warning".to_string(),
            },
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

pub type BridgeResult<T> = Result<T, BridgeError>;

/// Runs a closure, converting panics into a safe bridge error.
pub(crate) fn guard<T>(f: impl FnOnce() -> BridgeResult<T>) -> BridgeResult<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => Err(BridgeError::panic(payload)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_become_typed_internal_errors() {
        let error = guard(|| -> BridgeResult<()> { panic!("deliberate panic") }).unwrap_err();
        assert_eq!(error.code, "internal_error");
        assert!(!error.diagnostic_id.is_empty());
        assert!(!error.retryable);
    }

    #[test]
    fn validation_errors_carry_issues() {
        let mut report = koncet_domain::ValidationReport::new();
        report.error("required", "/name", "name is required");
        let error = BridgeError::from_app(AppError::from(report));
        assert_eq!(error.code, "validation_failed");
        assert_eq!(error.issues.len(), 1);
        assert_eq!(error.issues[0].path, "/name");
    }
}
