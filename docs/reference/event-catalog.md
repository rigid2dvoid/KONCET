# Event Catalogue — v1

> **Status:** Baseline · **Event schema version:** 1  
> **Last updated:** 2026-11-24  
> **Related:** [Simulation engine](../architecture/simulation-engine.md) · [Domain model](../architecture/domain-model.md) · [Data model](../architecture/data-model.md)

Domain events are the authoritative, ordered history of a run. They explain every state
transition and are persisted before being published to the UI. Logs and metrics are separate
streams; they do not replace events.

## 1. Envelope

| Field | Type | Rules |
| --- | --- | --- |
| `run_id` | string (UUIDv7) | Owner |
| `seq` | integer | Starts at 1; strictly consecutive per run |
| `sim_time_ms` | integer | ≥ 0; virtual time only |
| `wall_time_ms` | integer | UTC epoch milliseconds; metadata, never used for decisions |
| `category` | string | Coarse group |
| `type` | string | Stable namespaced identifier |
| `severity` | `debug` \| `info` \| `warn` \| `error` | Display/aggregation |
| `source` | string | `engine`, `build`, `container`, `process`, `proxy`, `firewall`, `deployment`, `user` |
| `payload` | object | Versioned JSON; no secrets or host paths |
| `schema_version` | integer | `1` |

Rules:

- `type` is forward-compatible; UI renders unknown types as a generic row.
- Payload fields are additive within a schema version. Removed/retyped fields require a new
  event type or schema version.
- Domain events are never sampled or dropped. Log/metric caps do not affect them.
- User-visible timeline uses the event type plus a rendering map; raw JSON is available for
  inspection and diagnostics.
- Canonical serialisation sorts keys and fixes number formatting for golden tests.

## 2. Run lifecycle

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `run.created` | info | `project_id`, `scenario_id`, `seed`, `app_version`, `engine_version` | Durable run row created |
| `run.started` | info | `source_snapshot_id`, `file_count`, `total_bytes` | Engine session started |
| `run.paused` | info | — | Virtual clock paused |
| `run.resumed` | info | — | Clock resumed |
| `run.speed_changed` | info | `speed` | 1/2/4/8 |
| `run.command` | info | `command`, `details` | Recorded user command affecting state |
| `run.finalised` | info | `status`, `result`, `reason`, `sim_duration_ms` | Terminal state summary |
| `run.cancelled` | warn | `reason` | User cancel or crash reconciliation (`app_terminated`) |
| `run.limit_reached` | warn | `max_sim_seconds` | Scenario duration limit ended execution |

## 3. Source and validation

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `source.snapshot_created` | info | `snapshot_id`, `content_hash`, `file_count`, `total_bytes` | Immutable input captured |
| `source.validation_warning` | warn | `path`, `code`, `message` | Non-blocking issue |
| `scenario.validated` | info | `warnings` | Scenario accepted for the run |

## 4. Build

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `build.queued` | info | `build_id`, `profile` | Build created |
| `build.stage_started` | info | `stage`, `ordinal`, `planned_ms` | Stage began |
| `build.stage_finished` | info/warn/error | `stage`, `status`, `actual_ms` | Stage succeeded/failed/skipped |
| `build.fault_injected` | warn | `stage`, `at_percent`, `message`, `code` | Scenario fault fired |
| `build.succeeded` | info | `build_id`, `image_id`, `digest` | Image promoted |
| `build.failed` | error | `stage`, `code`, `message` | No image produced |
| `image.created` | info | `image_id`, `repository`, `tag`, `digest`, `size_bytes`, `layers` | Immutable image metadata |
| `image.tagged` | info | `image_id`, `tag` | Tag assigned (reserved for future retag) |

## 5. Container

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `container.created` | info | `container_id`, `name`, `image_id` | Runtime object created |
| `container.starting` | info | `container_id` | Start requested |
| `container.running` | info | `container_id`, `port` | Process started and container active |
| `container.stopping` | info | `container_id`, `grace_s` | Graceful stop began |
| `container.stopped` | info | `container_id`, `exit_code`, `forced` | Container stopped |
| `container.failed` | error | `container_id`, `reason`, `exit_code` | Runtime failure |
| `container.removed` | info | `container_id` | Object removed |
| `container.restart_count_changed` | info | `container_id`, `restart_count` | Counter increased |

## 6. Process and health

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `process.started` | info | `attempt`, `pid` | Supervised process started |
| `process.exited` | warn/info | `exit_code`, `reason` | Process exited |
| `process.restart_scheduled` | warn | `attempt`, `delay_ms`, `policy` | Supervisor scheduled retry |
| `process.restart_exhausted` | error | `attempts`, `max_retries` | Retry limit reached |
| `process.fault_injected` | warn | `kind`, `after_s`, `duration_s` | Deterministic fault fired |
| `healthcheck.passed` | debug | `consecutive`, `latency_ms` | Health probe passed |
| `healthcheck.failed` | warn | `consecutive`, `reason` | Probe failed |
| `healthcheck.state_changed` | info | `from`, `to` | Container health transition |

## 7. Proxy and firewall

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `proxy.route_validated` | info/warn | `host`, `path`, `target_port`, `warnings` | Route accepted |
| `proxy.request_simulated` | debug | `host`, `path`, `result`, `status` | Probe evaluated; low volume |
| `proxy.probe_succeeded` | debug | `status`, `latency_ms` | End-to-end success |
| `proxy.probe_failed` | warn | `reason`, `status` | Route/connectivity failure |
| `firewall.rule_evaluated` | debug | `rule_id`, `action`, `source`, `port` | Match decision for a probe |
| `firewall.denied` | warn | `source`, `port`, `default_policy` | Denied by no-match/default or explicit deny |
| `firewall.config_warning` | warn | `rule_id`, `code`, `message` | Unreachable/contradictory rule warning |

Firewall evaluation is only logged for simulated traffic (health/connectivity probes), not for
every engine operation.

## 8. Deployment

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `deployment.created` | info | `deployment_id`, `service_name`, `container_id` | Deployment record created |
| `deployment.deploying` | info | `deployment_id` | Container starting |
| `deployment.status_changed` | info/warn/error | `from`, `to`, `primary_reason` | Derived status changed |
| `deployment.health_changed` | info/warn | `from`, `to` | Health aggregate changed |
| `deployment.degraded` | warn | `reason` | Running but not end-to-end healthy |
| `deployment.failed` | error | `reason` | Terminal failure |
| `deployment.stopped` | info | `reason` | Run/container stopped |
| `deployment.connectivity_checked` | info | `result`, `reason`, `at_sim_ms` | Explicit operator check |

## 8.1 Engine limits

| Type | Severity | Payload | Meaning |
| --- | --- | --- | --- |
| `engine.logs_truncated` | warn | `cap` | The per-run structured log cap was reached; further log lines are dropped once |
| `engine.metrics_truncated` | warn | `cap` | The per-run metric sample cap was reached; further samples are dropped once |

Domain events are never dropped by these caps; the truncation events make the display gap
explicit.

## 9. Rendering contract

- Timeline groups adjacent `debug` events; `info+` remain individually visible.
- Every error/warn event shows an icon plus text and links to its tab when a target exists.
- Payload display uses a whitelist of human fields; raw JSON is progressive disclosure.
- `run.command` is never used to imply real execution; commands are simulation controls.
- A missing renderer for an unknown event type displays `type`, severity, time, and a short
  payload summary without failing the timeline.

## 10. Examples

```json
{
  "run_id": "0192f0c8-7e5a-7d61-9a31-2b3f0b6d9a10",
  "seq": 37,
  "sim_time_ms": 24500,
  "wall_time_ms": 1790263200123,
  "category": "deployment",
  "type": "deployment.status_changed",
  "severity": "warn",
  "source": "deployment",
  "schema_version": 1,
  "payload": {
    "from": "deploying",
    "to": "degraded",
    "primary_reason": "firewall_denied"
  }
}
```

```json
{
  "type": "firewall.denied",
  "severity": "warn",
  "source": "firewall",
  "payload": {
    "source_alias": "proxy",
    "port": 3000,
    "protocol": "tcp",
    "default_policy": "deny",
    "rule_id": null
  }
}
```

Payloads above omit the envelope fields for brevity; persisted events always contain the full
envelope.

## 11. Change policy

- New optional payload fields: additive, same schema version.
- New event type: additive, no version bump required for readers that degrade gracefully.
- Renamed/removed/retyped fields: new event type or schema version + migration/renderer tests.
- Golden fixtures updated intentionally when event output changes.
- Historical runs keep their original events and render with the current catalogue plus generic
  fallback.
