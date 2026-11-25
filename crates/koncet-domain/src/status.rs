//! Status and state-machine enumerations.
//!
//! Wire representations are `snake_case` strings shared with SQLite and the Dart
//! boundary. Display strings are not stored.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = ParseStateError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    other => Err(ParseStateError {
                        kind: stringify!($name),
                        value: other.to_string(),
                    }),
                }
            }
        }
    };
}

/// Returned when a persisted status string is not recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseStateError {
    pub kind: &'static str,
    pub value: String,
}

impl fmt::Display for ParseStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {} value: {}", self.kind, self.value)
    }
}

impl std::error::Error for ParseStateError {}

string_enum!(RunStatus {
    Running => "running",
    Paused => "paused",
    Completed => "completed",
    Cancelled => "cancelled",
    Failed => "failed",
});

impl RunStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Paused)
    }
}

string_enum!(RunResult {
    Healthy => "healthy",
    Degraded => "degraded",
    BuildFailed => "build_failed",
    RuntimeFailed => "runtime_failed",
    Unknown => "unknown",
});

impl RunResult {
    /// Result corresponding to a deployment becoming healthy or degraded.
    pub const fn from_deployment(status: DeploymentStatus) -> Self {
        match status {
            DeploymentStatus::Healthy => Self::Healthy,
            DeploymentStatus::Degraded => Self::Degraded,
            DeploymentStatus::Failed => Self::RuntimeFailed,
            _ => Self::Unknown,
        }
    }
}

string_enum!(BuildStageStatus {
    Pending => "pending",
    Running => "running",
    Succeeded => "succeeded",
    Failed => "failed",
    Skipped => "skipped",
});

string_enum!(BuildStageName {
    Validate => "validate",
    Prepare => "prepare",
    Build => "build",
    Package => "package",
});

impl BuildStageName {
    pub const ORDERED: [Self; 4] = [Self::Validate, Self::Prepare, Self::Build, Self::Package];

    pub const fn ordinal(self) -> u8 {
        match self {
            Self::Validate => 1,
            Self::Prepare => 2,
            Self::Build => 3,
            Self::Package => 4,
        }
    }

    /// Default stage duration before `duration_scale` is applied.
    pub const fn default_duration_ms(self) -> u64 {
        match self {
            Self::Validate => 2_000,
            Self::Prepare => 6_000,
            Self::Build => 12_000,
            Self::Package => 3_000,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Validate => "Validate",
            Self::Prepare => "Prepare",
            Self::Build => "Build",
            Self::Package => "Package",
        }
    }
}

string_enum!(ContainerState {
    Created => "created",
    Starting => "starting",
    Running => "running",
    Stopping => "stopping",
    Stopped => "stopped",
    Failed => "failed",
    Removed => "removed",
});

impl ContainerState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Removed)
    }

    pub const fn is_live(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Stopping)
    }
}

string_enum!(HealthStatus {
    Unknown => "unknown",
    Starting => "starting",
    Healthy => "healthy",
    Unhealthy => "unhealthy",
});

string_enum!(DeploymentStatus {
    Pending => "pending",
    Deploying => "deploying",
    Healthy => "healthy",
    Degraded => "degraded",
    Failed => "failed",
    Stopped => "stopped",
});

impl DeploymentStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

/// Restart policy names use kebab-case in scenario JSON, matching the schema
/// reference and the stable wire strings. Other status enums are snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestartPolicyKind {
    No,
    OnFailure,
    Always,
}

impl RestartPolicyKind {
    pub const ALL: &'static [Self] = &[Self::No, Self::OnFailure, Self::Always];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::No => "no",
            Self::OnFailure => "on-failure",
            Self::Always => "always",
        }
    }
}

impl fmt::Display for RestartPolicyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RestartPolicyKind {
    type Err = ParseStateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "no" => Ok(Self::No),
            "on-failure" => Ok(Self::OnFailure),
            "always" => Ok(Self::Always),
            other => Err(ParseStateError {
                kind: "RestartPolicyKind",
                value: other.to_string(),
            }),
        }
    }
}

string_enum!(FirewallAction {
    Allow => "allow",
    Deny => "deny",
});

string_enum!(FirewallDefaultPolicy {
    Allow => "allow",
    Deny => "deny",
});

string_enum!(TrafficDirection {
    Inbound => "inbound",
});

string_enum!(TrafficProtocol {
    Tcp => "tcp",
    Udp => "udp",
});

string_enum!(TrafficSource {
    Proxy => "proxy",
    Monitor => "monitor",
    External => "external",
});

/// Stable primary reason codes shared with the UI and history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    BuildFailed,
    ImageMissing,
    ContainerFailed,
    RestartExhausted,
    HealthcheckFailed,
    RouteUnreachable,
    FirewallDenied,
    ProxyMisconfigured,
    UserStopped,
    AppTerminated,
}

impl FailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuildFailed => "build_failed",
            Self::ImageMissing => "image_missing",
            Self::ContainerFailed => "container_failed",
            Self::RestartExhausted => "restart_exhausted",
            Self::HealthcheckFailed => "healthcheck_failed",
            Self::RouteUnreachable => "route_unreachable",
            Self::FirewallDenied => "firewall_denied",
            Self::ProxyMisconfigured => "proxy_misconfigured",
            Self::UserStopped => "user_stopped",
            Self::AppTerminated => "app_terminated",
        }
    }
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FailureReason {
    type Err = ParseStateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|reason| reason.as_str() == value)
            .ok_or_else(|| ParseStateError {
                kind: "FailureReason",
                value: value.to_string(),
            })
    }
}

impl FailureReason {
    pub const ALL: &'static [Self] = &[
        Self::BuildFailed,
        Self::ImageMissing,
        Self::ContainerFailed,
        Self::RestartExhausted,
        Self::HealthcheckFailed,
        Self::RouteUnreachable,
        Self::FirewallDenied,
        Self::ProxyMisconfigured,
        Self::UserStopped,
        Self::AppTerminated,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_round_trip_through_strings() {
        for status in RunStatus::ALL {
            assert_eq!(status.to_string().parse::<RunStatus>().unwrap(), *status);
        }
        assert_eq!(RestartPolicyKind::OnFailure.to_string(), "on-failure");
        assert_eq!(BuildStageName::Build.ordinal(), 3);
    }

    #[test]
    fn terminal_run_statuses_are_exhaustive() {
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(!RunStatus::Paused.is_terminal());
    }

    #[test]
    fn restart_policy_uses_the_documented_kebab_case() {
        let policy = crate::scenario::RestartPolicy {
            policy: RestartPolicyKind::OnFailure,
            max_retries: Some(3),
        };
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("\"on-failure\""), "{json}");
        let parsed: crate::scenario::RestartPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, policy);
        assert_eq!(RestartPolicyKind::OnFailure.to_string(), "on-failure");
    }

    #[test]
    fn stage_durations_match_the_catalogue() {
        let total: u64 = BuildStageName::ORDERED
            .iter()
            .map(|stage| stage.default_duration_ms())
            .sum();
        assert_eq!(total, 23_000);
    }
}
