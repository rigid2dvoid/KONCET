//! Domain events, structured logs, and metric samples.
//!
//! Events are append-only and authoritative; logs and metrics are separate streams.
//! Canonical JSON sorts keys and never contains wall-clock values in payloads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::id::RunId;
use crate::time::SimTime;

pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// Display and aggregation severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Debug,
    Info,
    Warn,
    Error,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub const fn is_visible_in_timeline(self) -> bool {
        !matches!(self, Self::Debug)
    }

    pub const fn is_warning_or_error(self) -> bool {
        matches!(self, Self::Warn | Self::Error)
    }
}

/// Event producer. Source values are stable and shared with the event catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    Engine,
    Build,
    Container,
    Process,
    Proxy,
    Firewall,
    Deployment,
    User,
}

impl EventSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Build => "build",
            Self::Container => "container",
            Self::Process => "process",
            Self::Proxy => "proxy",
            Self::Firewall => "firewall",
            Self::Deployment => "deployment",
            Self::User => "user",
        }
    }
}

/// Log producer. Logs never come from the user directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    Engine,
    Build,
    Container,
    Process,
    Proxy,
    Firewall,
    Deployment,
}

impl LogSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Build => "build",
            Self::Container => "container",
            Self::Process => "process",
            Self::Proxy => "proxy",
            Self::Firewall => "firewall",
            Self::Deployment => "deployment",
        }
    }
}

/// One ordered domain event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainEvent {
    pub run_id: RunId,
    pub seq: u64,
    pub sim_time_ms: i64,
    pub wall_time_ms: i64,
    pub category: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub severity: Severity,
    pub source: EventSource,
    pub schema_version: u32,
    pub payload: Value,
}

impl DomainEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: RunId,
        seq: u64,
        sim_time: SimTime,
        wall_time_ms: i64,
        event_type: impl Into<String>,
        severity: Severity,
        source: EventSource,
        payload: Value,
    ) -> Self {
        let event_type = event_type.into();
        let category = event_type
            .split_once('.')
            .map(|(category, _)| category)
            .unwrap_or("engine")
            .to_string();
        let payload = if payload.is_object() {
            payload
        } else {
            json!({ "value": payload })
        };
        Self {
            run_id,
            seq,
            sim_time_ms: sim_time.as_millis(),
            wall_time_ms,
            category,
            event_type,
            severity,
            source,
            schema_version: EVENT_SCHEMA_VERSION,
            payload,
        }
    }

    pub fn to_canonical_json(&self) -> String {
        let mut value = serde_json::to_value(self).expect("events are serialisable");
        sort_value(&mut value);
        let mut text = serde_json::to_string_pretty(&value).expect("event JSON is valid");
        text.push('\n');
        text
    }
}

/// One structured log record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogRecord {
    pub run_id: RunId,
    pub seq: u64,
    pub sim_time_ms: i64,
    pub source: LogSource,
    pub level: Severity,
    pub message: String,
    pub context: BTreeMap<String, Value>,
}

impl LogRecord {
    pub fn new(
        run_id: RunId,
        seq: u64,
        sim_time: SimTime,
        source: LogSource,
        level: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            run_id,
            seq,
            sim_time_ms: sim_time.as_millis(),
            source,
            level,
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    pub fn to_canonical_json(&self) -> String {
        let mut value = serde_json::to_value(self).expect("logs are serialisable");
        sort_value(&mut value);
        let mut text = serde_json::to_string_pretty(&value).expect("log JSON is valid");
        text.push('\n');
        text
    }
}

/// Entity a metric belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricEntity {
    Build,
    Container,
    Deployment,
}

impl MetricEntity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Container => "container",
            Self::Deployment => "deployment",
        }
    }
}

/// One deterministic metric sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricSample {
    pub run_id: RunId,
    pub sim_time_ms: i64,
    pub entity_kind: MetricEntity,
    pub entity_id: String,
    pub metric: String,
    pub value: f64,
    pub unit: String,
}

impl MetricSample {
    pub fn new(
        run_id: RunId,
        sim_time: SimTime,
        entity_kind: MetricEntity,
        entity_id: impl Into<String>,
        metric: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
    ) -> Self {
        Self {
            run_id,
            sim_time_ms: sim_time.as_millis(),
            entity_kind,
            entity_id: entity_id.into(),
            metric: metric.into(),
            value: round_to(value, 3),
            unit: unit.into(),
        }
    }

    pub fn to_canonical_json(&self) -> String {
        let mut value = serde_json::to_value(self).expect("metrics are serialisable");
        sort_value(&mut value);
        let mut text = serde_json::to_string_pretty(&value).expect("metric JSON is valid");
        text.push('\n');
        text
    }
}

/// Rounds and guards against NaN/infinity, which would break golden fixtures.
pub fn round_to(value: f64, decimals: u32) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let factor = 10f64.powi(decimals as i32);
    (value * factor).round() / factor
}

fn sort_value(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(sort_value),
        Value::Object(map) => {
            for child in map.values_mut() {
                sort_value(child);
            }
        }
        _ => {}
    }
}

/// Stable event type strings from the catalogue. Unknown types render generically.
pub mod event_types {
    pub const RUN_CREATED: &str = "run.created";
    pub const RUN_STARTED: &str = "run.started";
    pub const RUN_PAUSED: &str = "run.paused";
    pub const RUN_RESUMED: &str = "run.resumed";
    pub const RUN_SPEED_CHANGED: &str = "run.speed_changed";
    pub const RUN_COMMAND: &str = "run.command";
    pub const RUN_FINALISED: &str = "run.finalised";
    pub const RUN_CANCELLED: &str = "run.cancelled";
    pub const RUN_LIMIT_REACHED: &str = "run.limit_reached";

    pub const SOURCE_SNAPSHOT_CREATED: &str = "source.snapshot_created";
    pub const SOURCE_VALIDATION_WARNING: &str = "source.validation_warning";
    pub const SCENARIO_VALIDATED: &str = "scenario.validated";

    pub const BUILD_QUEUED: &str = "build.queued";
    pub const BUILD_STAGE_STARTED: &str = "build.stage_started";
    pub const BUILD_STAGE_FINISHED: &str = "build.stage_finished";
    pub const BUILD_FAULT_INJECTED: &str = "build.fault_injected";
    pub const BUILD_SUCCEEDED: &str = "build.succeeded";
    pub const BUILD_FAILED: &str = "build.failed";
    pub const IMAGE_CREATED: &str = "image.created";
    pub const IMAGE_TAGGED: &str = "image.tagged";

    pub const CONTAINER_CREATED: &str = "container.created";
    pub const CONTAINER_STARTING: &str = "container.starting";
    pub const CONTAINER_RUNNING: &str = "container.running";
    pub const CONTAINER_STOPPING: &str = "container.stopping";
    pub const CONTAINER_STOPPED: &str = "container.stopped";
    pub const CONTAINER_FAILED: &str = "container.failed";
    pub const CONTAINER_REMOVED: &str = "container.removed";
    pub const CONTAINER_RESTART_COUNT_CHANGED: &str = "container.restart_count_changed";

    pub const PROCESS_STARTED: &str = "process.started";
    pub const PROCESS_EXITED: &str = "process.exited";
    pub const PROCESS_RESTART_SCHEDULED: &str = "process.restart_scheduled";
    pub const PROCESS_RESTART_EXHAUSTED: &str = "process.restart_exhausted";
    pub const PROCESS_FAULT_INJECTED: &str = "process.fault_injected";
    pub const HEALTHCHECK_PASSED: &str = "healthcheck.passed";
    pub const HEALTHCHECK_FAILED: &str = "healthcheck.failed";
    pub const HEALTHCHECK_STATE_CHANGED: &str = "healthcheck.state_changed";

    pub const PROXY_ROUTE_VALIDATED: &str = "proxy.route_validated";
    pub const PROXY_REQUEST_SIMULATED: &str = "proxy.request_simulated";
    pub const PROXY_PROBE_SUCCEEDED: &str = "proxy.probe_succeeded";
    pub const PROXY_PROBE_FAILED: &str = "proxy.probe_failed";
    pub const FIREWALL_RULE_EVALUATED: &str = "firewall.rule_evaluated";
    pub const FIREWALL_DENIED: &str = "firewall.denied";
    pub const FIREWALL_CONFIG_WARNING: &str = "firewall.config_warning";

    pub const DEPLOYMENT_CREATED: &str = "deployment.created";
    pub const DEPLOYMENT_DEPLOYING: &str = "deployment.deploying";
    pub const DEPLOYMENT_STATUS_CHANGED: &str = "deployment.status_changed";
    pub const DEPLOYMENT_HEALTH_CHANGED: &str = "deployment.health_changed";
    pub const DEPLOYMENT_DEGRADED: &str = "deployment.degraded";
    pub const DEPLOYMENT_FAILED: &str = "deployment.failed";
    pub const DEPLOYMENT_STOPPED: &str = "deployment.stopped";
    pub const DEPLOYMENT_CONNECTIVITY_CHECKED: &str = "deployment.connectivity_checked";

    pub const ENGINE_LOGS_TRUNCATED: &str = "engine.logs_truncated";
    pub const ENGINE_METRICS_TRUNCATED: &str = "engine.metrics_truncated";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_category_is_derived_from_type() {
        let event = DomainEvent::new(
            RunId::new(),
            1,
            SimTime::from_secs(2),
            123,
            event_types::DEPLOYMENT_DEGRADED,
            Severity::Warn,
            EventSource::Deployment,
            json!({ "reason": "firewall_denied" }),
        );
        assert_eq!(event.category, "deployment");
        assert_eq!(event.event_type, "deployment.degraded");
        let json = event.to_canonical_json();
        assert!(json.contains("\"type\": \"deployment.degraded\""));
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn non_object_payloads_are_wrapped() {
        let event = DomainEvent::new(
            RunId::new(),
            1,
            SimTime::ZERO,
            0,
            "custom.event",
            Severity::Info,
            EventSource::Engine,
            json!(5),
        );
        assert_eq!(event.payload["value"], 5);
    }

    #[test]
    fn metrics_are_rounded_and_finite() {
        let sample = MetricSample::new(
            RunId::new(),
            SimTime::from_secs(1),
            MetricEntity::Container,
            "c1",
            "cpu_millicores",
            123.4567,
            "m",
        );
        assert_eq!(sample.value, 123.457);
        let nan = round_to(f64::NAN, 3);
        assert_eq!(nan, 0.0);
    }

    #[test]
    fn log_context_is_sorted() {
        let log = LogRecord::new(
            RunId::new(),
            1,
            SimTime::ZERO,
            LogSource::Build,
            Severity::Info,
            "stage started",
        )
        .with_context("stage", json!("build"))
        .with_context("attempt", json!(1));
        let json = log.to_canonical_json();
        let attempt = json.find("\"attempt\"").unwrap();
        let stage = json.find("\"stage\"").unwrap();
        assert!(attempt < stage);
    }
}
