//! Scenario v1: strict, versioned configuration aggregate.
//!
//! The stored form is canonical JSON with sorted keys. Validation is pure and is
//! shared by the editor and run start. Only schema/invariant violations block a
//! run; surprising but valid configurations produce warnings.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{DomainError, ValidationReport};
use crate::id::{ProjectId, ScenarioId};
use crate::project::{ProjectTemplate, SourceTree};
use crate::status::{
    BuildStageName, FirewallAction, FirewallDefaultPolicy, RestartPolicyKind, TrafficDirection,
    TrafficProtocol, TrafficSource,
};
use crate::time::TimestampUtc;

pub const SCENARIO_SCHEMA_VERSION: u32 = 1;

/// Versioned scenario owned by a project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioRecord {
    pub id: ScenarioId,
    pub project_id: ProjectId,
    pub revision: u32,
    pub schema_version: u32,
    pub scenario: Scenario,
    pub canonical_json: String,
    pub created_at: TimestampUtc,
    pub updated_at: TimestampUtc,
}

/// A complete scenario document. Unknown fields are rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub seed_policy: SeedPolicy,
    pub limits: Limits,
    pub build: BuildConfig,
    pub service: ServiceConfig,
    pub proxy: ProxyConfig,
    pub firewall: FirewallConfig,
    #[serde(default)]
    pub faults: Vec<Fault>,
    pub monitoring: MonitoringConfig,
}

impl Scenario {
    /// Parses and validates a scenario document.
    pub fn from_json(json: &str) -> Result<Self, DomainError> {
        let scenario: Scenario = serde_json::from_str(json).map_err(|error| {
            DomainError::validation(
                "invalid_json",
                "",
                format!("scenario is not valid v1 JSON: {error}"),
            )
        })?;
        let report = scenario.validate();
        if let Some(error) = report.to_error() {
            return Err(error);
        }
        Ok(scenario)
    }

    /// Validates all fields, returning errors and warnings.
    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::new();
        if self.schema_version != SCENARIO_SCHEMA_VERSION {
            report.error(
                "unsupported_schema_version",
                "/schema_version",
                format!("schema_version must be exactly {SCENARIO_SCHEMA_VERSION}"),
            );
        }

        let name = self.name.trim();
        if name.is_empty() {
            report.error("required", "/name", "scenario name is required");
        } else if name.chars().count() > 80 {
            report.error(
                "too_long",
                "/name",
                "scenario name must be at most 80 characters",
            );
        }

        if self.description.chars().count() > 500 {
            report.error(
                "too_long",
                "/description",
                "description must be at most 500 characters",
            );
        }

        self.seed_policy.validate(&mut report);
        self.limits.validate(&mut report);
        self.build.validate(&mut report);
        self.service.validate(&mut report);
        self.proxy.validate(&self.service, &mut report);
        self.firewall
            .validate(&self.proxy, &self.service, &mut report);
        self.validate_faults(&mut report);
        self.monitoring.validate(&mut report);
        report
    }

    fn validate_faults(&self, report: &mut ValidationReport) {
        if self.faults.len() > 3 {
            report.error("too_many", "/faults", "at most 3 faults are supported");
        }
        for (index, fault) in self.faults.iter().enumerate() {
            fault.validate(index, report);
        }
    }

    /// Validates inputs that are only known at run start.
    pub fn validate_for_run(
        &self,
        sources: &SourceTree,
        expected_template: Option<ProjectTemplate>,
    ) -> ValidationReport {
        let mut report = self.validate();
        let entrypoint = self.build.profile.entrypoint();
        if sources.files().is_empty() {
            report.error(
                "empty_source_tree",
                "/files",
                "the source tree is empty; add at least the profile entrypoint",
            );
        } else if let Some(file) = sources.get(entrypoint) {
            if file.content.trim().is_empty() {
                report.error(
                    "empty_entrypoint",
                    "/files",
                    format!("the build profile requires a non-empty {entrypoint}"),
                );
            }
        } else {
            report.error(
                "missing_entrypoint",
                "/files",
                format!("the build profile requires {entrypoint}"),
            );
        }
        if let Some(template) = expected_template {
            let expected = template.as_str();
            if self.build.profile.as_str() != expected {
                report.warning(
                    "profile_differs_from_template",
                    "/build/profile",
                    format!(
                        "the project template is {expected} but the scenario builds {}",
                        self.build.profile
                    ),
                );
            }
        }
        report
    }

    /// Canonical storage form: sorted keys, two-space indentation, trailing newline.
    pub fn canonical_json(&self) -> String {
        let mut value = serde_json::to_value(self).expect("scenario is always serialisable");
        canonicalise_numbers(&mut value);
        let mut text = serde_json::to_string_pretty(&value).expect("scenario JSON is valid");
        text.push('\n');
        text
    }

    /// Resolves the policy to the seed recorded on a run.
    pub fn resolve_seed(&self, random_seed: u64) -> Result<u64, DomainError> {
        self.seed_policy.resolve(random_seed)
    }

    /// Repository is derived from immutable identifiers, never the project name.
    pub fn image_repository(&self, project_id: ProjectId) -> String {
        format!("koncet/{}/app", project_id.short())
    }
}

fn canonicalise_numbers(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Number(number) => {
            if number.is_f64() {
                if let Some(float) = number.as_f64() {
                    if float.fract() == 0.0 && float.is_finite() {
                        // Keep an explicit decimal point for canonical output so an
                        // integral scale reads as `1.0`, not `1`.
                        let repr = format!("{float:.1}");
                        if let Ok(replacement) = serde_json::Number::from_str(&repr) {
                            *number = replacement;
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(canonicalise_numbers),
        serde_json::Value::Object(map) => map.values_mut().for_each(canonicalise_numbers),
        _ => {}
    }
}

/// `random` draws once at run start; `fixed` replays the stored seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedPolicy {
    pub mode: SeedMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

impl SeedPolicy {
    pub fn random() -> Self {
        Self {
            mode: SeedMode::Random,
            value: None,
        }
    }

    pub fn fixed(value: impl Into<String>) -> Self {
        Self {
            mode: SeedMode::Fixed,
            value: Some(value.into()),
        }
    }

    pub fn resolve(&self, random_seed: u64) -> Result<u64, DomainError> {
        match self.mode {
            SeedMode::Random => Ok(random_seed & MAX_SEED),
            SeedMode::Fixed => {
                let value = self.value.as_deref().ok_or_else(|| {
                    DomainError::validation(
                        "required",
                        "/seed_policy/value",
                        "fixed seed mode requires a value",
                    )
                })?;
                value.parse::<u64>().map_err(|_| {
                    DomainError::validation(
                        "invalid_format",
                        "/seed_policy/value",
                        "fixed seed must be a non-negative decimal integer",
                    )
                })
            }
        }
    }

    fn validate(&self, report: &mut ValidationReport) {
        match self.mode {
            SeedMode::Random => {
                if self.value.is_some() {
                    report.error(
                        "incompatible",
                        "/seed_policy/value",
                        "random seed mode must not include a value",
                    );
                }
            }
            SeedMode::Fixed => match self.value.as_deref() {
                None => report.error(
                    "required",
                    "/seed_policy/value",
                    "fixed seed mode requires a value",
                ),
                Some(value) => {
                    let valid_chars = value.chars().all(|c| c.is_ascii_digit());
                    let leading_zero = value.len() > 1 && value.starts_with('0');
                    if !valid_chars || value.is_empty() || leading_zero {
                        report.error(
                            "invalid_format",
                            "/seed_policy/value",
                            "seed must be an unsigned decimal integer without leading zeros",
                        );
                    } else if value
                        .parse::<u64>()
                        .map(|seed| seed > MAX_SEED)
                        .unwrap_or(true)
                    {
                        report.error(
                            "out_of_range",
                            "/seed_policy/value",
                            "seed must be between 0 and 9223372036854775807",
                        );
                    }
                }
            },
        }
    }
}

/// Largest supported seed: 2^63 − 1.
pub const MAX_SEED: u64 = i64::MAX as u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeedMode {
    Random,
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_max_sim_seconds")]
    pub max_sim_seconds: u32,
    #[serde(default = "default_true")]
    pub stop_on_terminal_failure: bool,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_sim_seconds: default_max_sim_seconds(),
            stop_on_terminal_failure: true,
        }
    }
}

impl Limits {
    fn validate(&self, report: &mut ValidationReport) {
        if !(60..=86_400).contains(&self.max_sim_seconds) {
            report.error(
                "out_of_range",
                "/limits/max_sim_seconds",
                "max_sim_seconds must be between 60 and 86400",
            );
        }
    }
}

const fn default_max_sim_seconds() -> u32 {
    3_600
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildConfig {
    pub profile: BuildProfile,
    #[serde(default = "default_duration_scale")]
    pub duration_scale: f64,
    #[serde(default)]
    pub failure_injections: Vec<FailureInjection>,
}

impl BuildConfig {
    fn validate(&self, report: &mut ValidationReport) {
        if !self.duration_scale.is_finite() || !(0.25..=4.0).contains(&self.duration_scale) {
            report.error(
                "out_of_range",
                "/build/duration_scale",
                "duration_scale must be between 0.25 and 4.0",
            );
        } else {
            let scaled = self.duration_scale * 100.0;
            if (scaled - scaled.round()).abs() > 1e-9 {
                report.error(
                    "invalid_format",
                    "/build/duration_scale",
                    "duration_scale supports at most two decimal places",
                );
            }
        }

        if self.failure_injections.len() > 2 {
            report.error(
                "too_many",
                "/build/failure_injections",
                "at most 2 failure injections are supported",
            );
        }
        let mut stages = BTreeSet::new();
        for (index, injection) in self.failure_injections.iter().enumerate() {
            let pointer = format!("/build/failure_injections/{index}");
            if !stages.insert(injection.stage) {
                report.error(
                    "duplicate",
                    format!("{pointer}/stage"),
                    "only one failure injection per stage is allowed",
                );
            }
            if injection.message.trim().is_empty() || injection.message.chars().count() > 200 {
                report.error(
                    "out_of_range",
                    format!("{pointer}/message"),
                    "message must be between 1 and 200 characters",
                );
            }
            if injection.at_percent > 100 {
                report.error(
                    "out_of_range",
                    format!("{pointer}/at_percent"),
                    "at_percent must be between 0 and 100",
                );
            }
            if let Some(code) = &injection.code {
                if !is_failure_code(code) {
                    report.error(
                        "invalid_format",
                        format!("{pointer}/code"),
                        "code must match [a-z0-9_]{3,40}",
                    );
                }
            }
        }
    }
}

const fn default_duration_scale() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildProfile {
    StaticWeb,
    HttpService,
}

impl BuildProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaticWeb => "static-web",
            Self::HttpService => "http-service",
        }
    }

    pub const fn entrypoint(self) -> &'static str {
        match self {
            Self::StaticWeb => "index.html",
            Self::HttpService => "src/service.js",
        }
    }

    pub const fn baseline_image_bytes(self) -> u64 {
        match self {
            Self::StaticWeb => 18 * 1024 * 1024,
            Self::HttpService => 42 * 1024 * 1024,
        }
    }
}

impl fmt::Display for BuildProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureInjection {
    pub stage: BuildStageName,
    pub at_percent: u8,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl FailureInjection {
    pub fn code(&self) -> &str {
        self.code.as_deref().unwrap_or("injected_build_failure")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub port: u32,
    pub cpu_millicores: u32,
    pub memory_mb: u32,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub startup_delay_s: u32,
    pub restart_policy: RestartPolicy,
    pub health_check: HealthCheck,
}

impl ServiceConfig {
    fn validate(&self, report: &mut ValidationReport) {
        if !is_service_name(&self.name) {
            report.error(
                "invalid_format",
                "/service/name",
                "service name must match [a-z][a-z0-9-]{2,39}",
            );
        }
        if self.command.trim().is_empty() {
            report.error("required", "/service/command", "command is required");
        } else if self.command.chars().count() > 200 {
            report.error(
                "too_long",
                "/service/command",
                "command must be at most 200 characters",
            );
        } else if self.command.chars().any(char::is_control) {
            report.error(
                "invalid_format",
                "/service/command",
                "command must not contain control characters",
            );
        }
        if !(1..=65_535).contains(&self.port) {
            report.error(
                "out_of_range",
                "/service/port",
                "port must be between 1 and 65535",
            );
        }
        if !(50..=4_000).contains(&self.cpu_millicores) {
            report.error(
                "out_of_range",
                "/service/cpu_millicores",
                "cpu_millicores must be between 50 and 4000",
            );
        }
        if !(32..=4_096).contains(&self.memory_mb) {
            report.error(
                "out_of_range",
                "/service/memory_mb",
                "memory_mb must be between 32 and 4096",
            );
        }
        if self.startup_delay_s > 300 {
            report.error(
                "out_of_range",
                "/service/startup_delay_s",
                "startup_delay_s must be between 0 and 300",
            );
        }
        self.validate_env(report);
        self.restart_policy.validate(report);
        self.health_check.validate(report);
    }

    fn validate_env(&self, report: &mut ValidationReport) {
        if self.env.len() > 50 {
            report.error(
                "too_many",
                "/service/env",
                "at most 50 environment entries are allowed",
            );
        }
        for (key, value) in &self.env {
            let pointer = format!("/service/env/{key}");
            if !is_env_key(key) {
                report.error(
                    "invalid_format",
                    pointer.clone(),
                    "environment keys must match [A-Z_][A-Z0-9_]*",
                );
            }
            if value.len() > 4 * 1024 {
                report.error(
                    "too_long",
                    pointer,
                    "environment values must be at most 4 KiB",
                );
            }
        }
        let encoded = serde_json::to_vec(&self.env)
            .map(|bytes| bytes.len())
            .unwrap_or(0);
        if encoded > 64 * 1024 {
            report.error(
                "too_large",
                "/service/env",
                "environment configuration must be at most 64 KiB",
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestartPolicy {
    pub policy: RestartPolicyKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            policy: RestartPolicyKind::OnFailure,
            max_retries: Some(3),
        }
    }
}

impl RestartPolicy {
    fn validate(&self, report: &mut ValidationReport) {
        match self.policy {
            RestartPolicyKind::OnFailure => match self.max_retries {
                None => report.error(
                    "required",
                    "/service/restart_policy/max_retries",
                    "max_retries is required for on-failure",
                ),
                Some(value) if !(1..=10).contains(&value) => report.error(
                    "out_of_range",
                    "/service/restart_policy/max_retries",
                    "max_retries must be between 1 and 10",
                ),
                Some(_) => {}
            },
            RestartPolicyKind::No | RestartPolicyKind::Always => {
                if let Some(value) = self.max_retries {
                    if !(1..=10).contains(&value) {
                        report.error(
                            "out_of_range",
                            "/service/restart_policy/max_retries",
                            "max_retries must be between 1 and 10 when present",
                        );
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthCheck {
    pub path: String,
    pub interval_s: u32,
    pub timeout_s: u32,
    pub retries: u32,
}

impl HealthCheck {
    fn validate(&self, report: &mut ValidationReport) {
        if !is_http_path(&self.path, 200) {
            report.error(
                "invalid_format",
                "/service/health_check/path",
                "health path must start with / and be at most 200 characters",
            );
        }
        if !(1..=60).contains(&self.interval_s) {
            report.error(
                "out_of_range",
                "/service/health_check/interval_s",
                "interval_s must be between 1 and 60",
            );
        }
        if !(1..=30).contains(&self.timeout_s) {
            report.error(
                "out_of_range",
                "/service/health_check/timeout_s",
                "timeout_s must be between 1 and 30",
            );
        }
        if self.timeout_s >= self.interval_s {
            report.error(
                "incompatible",
                "/service/health_check/timeout_s",
                "timeout_s must be less than interval_s",
            );
        }
        if !(1..=10).contains(&self.retries) {
            report.error(
                "out_of_range",
                "/service/health_check/retries",
                "retries must be between 1 and 10",
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyConfig {
    pub enabled: bool,
    pub listener_port: u32,
    pub host: String,
    pub path: String,
    pub target_port: u32,
}

impl ProxyConfig {
    fn validate(&self, service: &ServiceConfig, report: &mut ValidationReport) {
        if !(1..=65_535).contains(&self.listener_port) {
            report.error(
                "out_of_range",
                "/proxy/listener_port",
                "listener_port must be between 1 and 65535",
            );
        }
        if !(1..=65_535).contains(&self.target_port) {
            report.error(
                "out_of_range",
                "/proxy/target_port",
                "target_port must be between 1 and 65535",
            );
        }
        if !is_koncet_hostname(&self.host) {
            report.error(
                "invalid_format",
                "/proxy/host",
                "host must be a lowercase DNS name ending in .koncet.local",
            );
        }
        if !is_http_path(&self.path, 200) {
            report.error(
                "invalid_format",
                "/proxy/path",
                "proxy path must start with /, be at most 200 characters, and contain no query or fragment",
            );
        }
        if self.target_port != service.port {
            report.error(
                "incompatible",
                "/proxy/target_port",
                "target_port must equal service.port",
            );
        }
        if self.enabled && self.listener_port == service.port {
            report.error(
                "port_conflict",
                "/proxy/listener_port",
                "listener_port must differ from the service port",
            );
        }
        if !self.enabled {
            report.warning(
                "proxy_disabled",
                "/proxy/enabled",
                "with the proxy disabled there is no route and the deployment cannot become healthy end to end",
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirewallConfig {
    #[serde(default = "default_firewall_policy")]
    pub default_policy: FirewallDefaultPolicy,
    #[serde(default)]
    pub rules: Vec<FirewallRule>,
}

impl Default for FirewallConfig {
    fn default() -> Self {
        Self {
            default_policy: FirewallDefaultPolicy::Deny,
            rules: Vec::new(),
        }
    }
}

const fn default_firewall_policy() -> FirewallDefaultPolicy {
    FirewallDefaultPolicy::Deny
}

impl FirewallConfig {
    fn validate(
        &self,
        proxy: &ProxyConfig,
        service: &ServiceConfig,
        report: &mut ValidationReport,
    ) {
        if self.rules.len() > 10 {
            report.error(
                "too_many",
                "/firewall/rules",
                "at most 10 firewall rules are allowed",
            );
        }
        let mut ids = BTreeSet::new();
        for (index, rule) in self.rules.iter().enumerate() {
            let pointer = format!("/firewall/rules/{index}");
            if !is_firewall_rule_id(&rule.id) {
                report.error(
                    "invalid_format",
                    format!("{pointer}/id"),
                    "rule id must match [a-z][a-z0-9-]{2,40}",
                );
            }
            if !ids.insert(rule.id.as_str()) {
                report.error(
                    "duplicate",
                    format!("{pointer}/id"),
                    "firewall rule ids must be unique",
                );
            }
            if !(1..=65_535).contains(&rule.port) {
                report.error(
                    "out_of_range",
                    format!("{pointer}/port"),
                    "port must be between 1 and 65535",
                );
            }
            if rule.protocol == TrafficProtocol::Udp {
                report.warning(
                    "unreachable_rule",
                    format!("{pointer}/protocol"),
                    "only tcp traffic is simulated in v0.1; udp rules never match",
                );
            }
        }

        if proxy.enabled {
            let matching = self.rules.iter().find(|rule| {
                rule.direction == TrafficDirection::Inbound
                    && rule.protocol == TrafficProtocol::Tcp
                    && rule.port == service.port
                    && rule.source == TrafficSource::Proxy
            });
            match matching {
                None if self.default_policy == FirewallDefaultPolicy::Deny => report.warning(
                    "firewall_allow_missing",
                    "/firewall/rules",
                    "the proxy has no matching allow rule; the deployment will be degraded (firewall_denied)",
                ),
                Some(rule) if rule.action == FirewallAction::Deny => report.warning(
                    "firewall_denied_by_rule",
                    "/firewall/rules",
                    format!(
                        "rule {} denies proxy traffic to port {}; the deployment will be degraded",
                        rule.id, service.port
                    ),
                ),
                _ => {}
            }
        }
    }

    /// First matching inbound rule wins; otherwise the default policy applies.
    pub fn evaluate(
        &self,
        source: TrafficSource,
        protocol: TrafficProtocol,
        port: u32,
    ) -> FirewallDecision {
        for rule in &self.rules {
            if rule.direction == TrafficDirection::Inbound
                && rule.protocol == protocol
                && rule.port == port
                && rule.source == source
            {
                return FirewallDecision {
                    allowed: rule.action == FirewallAction::Allow,
                    rule_id: Some(rule.id.clone()),
                    default_policy: self.default_policy,
                };
            }
        }
        FirewallDecision {
            allowed: self.default_policy == FirewallDefaultPolicy::Allow,
            rule_id: None,
            default_policy: self.default_policy,
        }
    }
}

/// Outcome of evaluating virtual traffic against the firewall.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirewallDecision {
    pub allowed: bool,
    pub rule_id: Option<String>,
    pub default_policy: FirewallDefaultPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirewallRule {
    pub id: String,
    pub action: FirewallAction,
    pub direction: TrafficDirection,
    pub protocol: TrafficProtocol,
    pub port: u32,
    pub source: TrafficSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fault {
    pub kind: FaultKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_s: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub once: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u8>,
}

impl Fault {
    pub fn process_crash(after_s: u32, once: bool, exit_code: u8) -> Self {
        Self {
            kind: FaultKind::ProcessCrash,
            after_s: Some(after_s),
            duration_s: None,
            at_s: None,
            once: Some(once),
            exit_code: Some(exit_code),
        }
    }

    pub fn healthcheck_failure(after_s: u32, duration_s: u32) -> Self {
        Self {
            kind: FaultKind::HealthcheckFailure,
            after_s: Some(after_s),
            duration_s: Some(duration_s),
            at_s: None,
            once: None,
            exit_code: None,
        }
    }

    pub fn startup_stall(at_s: u32, duration_s: u32) -> Self {
        Self {
            kind: FaultKind::StartupStall,
            after_s: None,
            duration_s: Some(duration_s),
            at_s: Some(at_s),
            once: None,
            exit_code: None,
        }
    }

    fn validate(&self, index: usize, report: &mut ValidationReport) {
        let pointer = format!("/faults/{index}");
        match self.kind {
            FaultKind::ProcessCrash => {
                require_range(report, &pointer, "after_s", self.after_s, 1, 3_600);
                reject_field(report, &pointer, "at_s", self.at_s.is_some());
                reject_field(report, &pointer, "duration_s", self.duration_s.is_some());
                if let Some(code) = self.exit_code {
                    if code == 0 {
                        report.error(
                            "out_of_range",
                            format!("{pointer}/exit_code"),
                            "process_crash exit_code must be between 1 and 255",
                        );
                    }
                }
            }
            FaultKind::HealthcheckFailure => {
                require_range(report, &pointer, "after_s", self.after_s, 0, 3_600);
                require_range(report, &pointer, "duration_s", self.duration_s, 1, 3_600);
                reject_field(report, &pointer, "at_s", self.at_s.is_some());
                reject_field(report, &pointer, "once", self.once.is_some());
                reject_field(report, &pointer, "exit_code", self.exit_code.is_some());
            }
            FaultKind::StartupStall => {
                require_range(report, &pointer, "at_s", self.at_s, 0, 300);
                require_range(report, &pointer, "duration_s", self.duration_s, 1, 600);
                reject_field(report, &pointer, "after_s", self.after_s.is_some());
                reject_field(report, &pointer, "once", self.once.is_some());
                reject_field(report, &pointer, "exit_code", self.exit_code.is_some());
            }
        }
    }
}

fn require_range(
    report: &mut ValidationReport,
    pointer: &str,
    field: &str,
    value: Option<u32>,
    min: u32,
    max: u32,
) {
    match value {
        None => report.error(
            "required",
            format!("{pointer}/{field}"),
            format!("{field} is required for this fault"),
        ),
        Some(value) if !(min..=max).contains(&value) => report.error(
            "out_of_range",
            format!("{pointer}/{field}"),
            format!("{field} must be between {min} and {max}"),
        ),
        Some(_) => {}
    }
}

fn reject_field(report: &mut ValidationReport, pointer: &str, field: &str, present: bool) {
    if present {
        report.error(
            "incompatible",
            format!("{pointer}/{field}"),
            format!("{field} is not valid for this fault kind"),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    ProcessCrash,
    HealthcheckFailure,
    StartupStall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonitoringConfig {
    #[serde(default = "default_sample_interval")]
    pub sample_interval_s: u32,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            sample_interval_s: default_sample_interval(),
        }
    }
}

impl MonitoringConfig {
    fn validate(&self, report: &mut ValidationReport) {
        if !(1..=60).contains(&self.sample_interval_s) {
            report.error(
                "out_of_range",
                "/monitoring/sample_interval_s",
                "sample_interval_s must be between 1 and 60",
            );
        }
    }
}

const fn default_sample_interval() -> u32 {
    5
}

fn is_ascii_pattern(
    value: &str,
    min: usize,
    max: usize,
    first: impl Fn(char) -> bool,
    rest: impl Fn(char) -> bool,
) -> bool {
    let count = value.chars().count();
    if count < min || count > max {
        return false;
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(c) if first(c) => chars.all(rest),
        _ => false,
    }
}

fn is_service_name(value: &str) -> bool {
    is_ascii_pattern(
        value,
        3,
        40,
        |c| c.is_ascii_lowercase(),
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-',
    )
}

fn is_firewall_rule_id(value: &str) -> bool {
    is_ascii_pattern(
        value,
        3,
        41,
        |c| c.is_ascii_lowercase(),
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-',
    )
}

fn is_failure_code(value: &str) -> bool {
    is_ascii_pattern(
        value,
        3,
        40,
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_',
        |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_',
    )
}

fn is_env_key(value: &str) -> bool {
    is_ascii_pattern(
        value,
        1,
        128,
        |c| c.is_ascii_uppercase() || c == '_',
        |c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_',
    )
}

fn is_http_path(value: &str, max: usize) -> bool {
    value.starts_with('/')
        && value.chars().count() <= max
        && !value.contains('?')
        && !value.contains('#')
        && !value.chars().any(|c| c.is_control() || c.is_whitespace())
}

pub fn is_koncet_hostname(value: &str) -> bool {
    let Some(prefix) = value.strip_suffix(".koncet.local") else {
        return false;
    };
    if prefix.is_empty() || value.len() > 253 {
        return false;
    }
    prefix.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    })
}

/// Built-in scenarios. All presets are ordinary scenarios and remain editable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioPreset {
    HappyPath,
    BuildFailure,
    BlockedPort,
    CrashLoop,
}

impl ScenarioPreset {
    pub const ALL: [Self; 4] = [
        Self::HappyPath,
        Self::BuildFailure,
        Self::BlockedPort,
        Self::CrashLoop,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::HappyPath => "happy-path",
            Self::BuildFailure => "build-failure",
            Self::BlockedPort => "blocked-port",
            Self::CrashLoop => "crash-loop",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::HappyPath => "Happy Path",
            Self::BuildFailure => "Build Failure",
            Self::BlockedPort => "Blocked Port",
            Self::CrashLoop => "Crash Loop",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::HappyPath => "Build and expose one simulated static web service.",
            Self::BuildFailure => "A compiler failure stops the build before an image exists.",
            Self::BlockedPort => "The proxy is enabled but the firewall denies its traffic.",
            Self::CrashLoop => "The process crashes repeatedly until restarts are exhausted.",
        }
    }

    pub fn scenario(self) -> Scenario {
        let mut scenario = base_scenario(self.label(), self.description());
        match self {
            Self::HappyPath => {}
            Self::BuildFailure => {
                scenario.build.failure_injections = vec![FailureInjection {
                    stage: BuildStageName::Build,
                    at_percent: 40,
                    message: "Injected compiler failure".to_string(),
                    code: None,
                }];
            }
            Self::BlockedPort => {
                scenario.firewall.rules.clear();
            }
            Self::CrashLoop => {
                scenario.service.restart_policy = RestartPolicy {
                    policy: RestartPolicyKind::OnFailure,
                    max_retries: Some(3),
                };
                scenario.faults = vec![Fault::process_crash(5, false, 1)];
            }
        }
        scenario
    }
}

impl fmt::Display for ScenarioPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn base_scenario(name: &str, description: &str) -> Scenario {
    Scenario {
        schema_version: SCENARIO_SCHEMA_VERSION,
        name: name.to_string(),
        description: description.to_string(),
        seed_policy: SeedPolicy::random(),
        limits: Limits::default(),
        build: BuildConfig {
            profile: BuildProfile::StaticWeb,
            duration_scale: 1.0,
            failure_injections: Vec::new(),
        },
        service: ServiceConfig {
            name: "web".to_string(),
            command: "serve --port 3000".to_string(),
            port: 3000,
            cpu_millicores: 500,
            memory_mb: 256,
            env: BTreeMap::from([("APP_ENV".to_string(), "simulation".to_string())]),
            startup_delay_s: 2,
            restart_policy: RestartPolicy {
                policy: RestartPolicyKind::OnFailure,
                max_retries: Some(3),
            },
            health_check: HealthCheck {
                path: "/healthz".to_string(),
                interval_s: 5,
                timeout_s: 2,
                retries: 2,
            },
        },
        proxy: ProxyConfig {
            enabled: true,
            listener_port: 8080,
            host: "app.koncet.local".to_string(),
            path: "/".to_string(),
            target_port: 3000,
        },
        firewall: FirewallConfig {
            default_policy: FirewallDefaultPolicy::Deny,
            rules: vec![FirewallRule {
                id: "allow-proxy".to_string(),
                action: FirewallAction::Allow,
                direction: TrafficDirection::Inbound,
                protocol: TrafficProtocol::Tcp,
                port: 3000,
                source: TrafficSource::Proxy,
            }],
        },
        faults: Vec::new(),
        monitoring: MonitoringConfig::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_validates() {
        for preset in ScenarioPreset::ALL {
            let scenario = preset.scenario();
            let report = scenario.validate();
            assert!(
                report.is_valid(),
                "{} should validate: {:?}",
                preset.label(),
                report.issues
            );
        }
    }

    #[test]
    fn happy_path_matches_the_documented_example() {
        let json = ScenarioPreset::HappyPath.scenario().canonical_json();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["service"]["port"], 3000);
        assert_eq!(value["proxy"]["host"], "app.koncet.local");
        assert_eq!(value["firewall"]["rules"][0]["source"], "proxy");
    }

    #[test]
    fn canonical_json_has_sorted_keys_and_trailing_newline() {
        let text = ScenarioPreset::HappyPath.scenario().canonical_json();
        assert!(text.ends_with('\n'));
        let build = text.find("\"build\"").unwrap();
        let description = text.find("\"description\"").unwrap();
        assert!(build < description, "keys should be sorted");
        assert!(text.contains("\"duration_scale\": 1.0"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = serde_json::to_value(ScenarioPreset::HappyPath.scenario()).unwrap();
        value["unexpected"] = serde_json::json!(true);
        let error = Scenario::from_json(&value.to_string()).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Validation);
    }

    #[test]
    fn bad_health_check_timeout_is_blocking() {
        let mut scenario = ScenarioPreset::HappyPath.scenario();
        scenario.service.health_check.timeout_s = 5;
        let report = scenario.validate();
        assert!(!report.is_valid());
        assert!(report
            .errors()
            .any(|issue| issue.path == "/service/health_check/timeout_s"));
    }

    #[test]
    fn blocked_port_is_a_warning_not_an_error() {
        let scenario = ScenarioPreset::BlockedPort.scenario();
        let report = scenario.validate();
        assert!(report.is_valid());
        assert!(report
            .warnings()
            .any(|issue| issue.code == "firewall_allow_missing"));
        let decision = scenario
            .firewall
            .evaluate(TrafficSource::Proxy, TrafficProtocol::Tcp, 3000);
        assert!(!decision.allowed);
    }

    #[test]
    fn seed_policy_is_strict() {
        assert_eq!(SeedPolicy::random().resolve(42).unwrap(), 42);
        assert_eq!(
            SeedPolicy::fixed("9223372036854775807").resolve(0).unwrap(),
            MAX_SEED
        );
        for bad in ["01", "-1", "9223372036854775808", "abc", ""] {
            let report = SeedPolicy::fixed(bad).validate_report_for_test();
            assert!(!report.is_valid(), "{bad} should be rejected");
        }
    }

    #[test]
    fn required_entrypoint_depends_on_profile() {
        let scenario = ScenarioPreset::HappyPath.scenario();
        let empty = SourceTree::empty();
        assert!(!scenario.validate_for_run(&empty, None).is_valid());

        let files = ProjectTemplate::StaticWeb.seed_files();
        let tree = SourceTree::new(files).unwrap();
        assert!(scenario
            .validate_for_run(&tree, Some(ProjectTemplate::StaticWeb))
            .is_valid());
    }

    // Test-only helper to keep validation assertions concise.
    trait SeedPolicyTestExt {
        fn validate_report_for_test(&self) -> ValidationReport;
    }

    impl SeedPolicyTestExt for SeedPolicy {
        fn validate_report_for_test(&self) -> ValidationReport {
            let mut report = ValidationReport::new();
            self.validate(&mut report);
            report
        }
    }
}
