# Scenario Schema — v1

> **Status:** Baseline · **Schema version:** 1  
> **Last updated:** 2026-11-24  
> **Related:** [Simulation engine](../architecture/simulation-engine.md) · [Domain model](../architecture/domain-model.md) · [PRD](../product/prd.md)

Scenarios are stored as canonical JSON in SQLite, snapshotted into each run, and validated by
the Rust core before a run can start. This document is normative for `schema_version: 1`.
The UI form is a friendly editor over this data; it does not define different semantics.

## 1. Invariants

- Unknown top-level or nested fields are rejected in v1 (strict deserialisation).
- Unknown `schema_version` values are rejected; migrations for v1 are not yet needed.
- All durations are integer seconds in configuration; simulation converts to milliseconds.
- Numeric limits below are inclusive.
- `seed_policy` is a policy; the actual resolved seed is stored on the run and shown in UI.
- A scenario must describe exactly one service and one proxy route in `v0.1`.
- Hostnames must end in `.koncet.local`. Ports are virtual.
- A scenario never references the host filesystem, external URLs, credentials, or real
  container/runtime identifiers.

## 2. Canonical example

```json
{
  "schema_version": 1,
  "name": "Happy Path",
  "description": "Build and expose one simulated static web service.",
  "seed_policy": {
    "mode": "random"
  },
  "limits": {
    "max_sim_seconds": 3600,
    "stop_on_terminal_failure": true
  },
  "build": {
    "profile": "static-web",
    "duration_scale": 1.0,
    "failure_injections": []
  },
  "service": {
    "name": "web",
    "command": "serve --port 3000",
    "port": 3000,
    "cpu_millicores": 500,
    "memory_mb": 256,
    "env": {
      "APP_ENV": "simulation"
    },
    "startup_delay_s": 2,
    "restart_policy": {
      "policy": "on-failure",
      "max_retries": 3
    },
    "health_check": {
      "path": "/healthz",
      "interval_s": 5,
      "timeout_s": 2,
      "retries": 2
    }
  },
  "proxy": {
    "enabled": true,
    "listener_port": 8080,
    "host": "app.koncet.local",
    "path": "/",
    "target_port": 3000
  },
  "firewall": {
    "default_policy": "deny",
    "rules": [
      {
        "id": "allow-proxy",
        "action": "allow",
        "direction": "inbound",
        "protocol": "tcp",
        "port": 3000,
        "source": "proxy"
      }
    ]
  },
  "faults": [],
  "monitoring": {
    "sample_interval_s": 5
  }
}
```

Canonical storage uses sorted object keys, two-space indentation, no comments, and `\n`
endings. Hashing and comparison always use this canonical form, not the user's editor spacing.

## 3. Field reference

### 3.1 Root

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `schema_version` | integer | yes | Exactly `1` for this version |
| `name` | string | yes | 1–80 chars after trim |
| `description` | string | no | ≤ 500 chars; default `""` |
| `seed_policy` | object | yes | See §3.2 |
| `limits` | object | yes | See §3.3 |
| `build` | object | yes | See §3.4 |
| `service` | object | yes | See §3.5 |
| `proxy` | object | yes | See §3.6 |
| `firewall` | object | yes | See §3.7 |
| `faults` | array | no | ≤ 3; see §3.8 |
| `monitoring` | object | yes | See §3.9 |

### 3.2 `seed_policy`

| Field | Type | Rules |
| --- | --- | --- |
| `mode` | `"random"` \| `"fixed"` | Required |
| `value` | decimal string | Required iff `mode = fixed`; 0 to 2^63−1, no sign or leading zero |

Random mode draws a seed once at run start. The actual seed is recorded on the run and is
copyable from the UI. Changing the policy never changes historical runs.

### 3.3 `limits`

| Field | Type | Rules |
| --- | --- | --- |
| `max_sim_seconds` | integer | 60–86,400; default 3,600 |
| `stop_on_terminal_failure` | boolean | Default `true`; if false, a failed build still ends the run in v1 because there is nothing to observe |

### 3.4 `build`

| Field | Type | Rules |
| --- | --- | --- |
| `profile` | string | One of `static-web`, `http-service` |
| `duration_scale` | number | 0.25–4.0, max 2 decimals; scales all stage durations |
| `failure_injections` | array | 0–2 entries; one per stage |

Failure injection:

| Field | Type | Rules |
| --- | --- | --- |
| `stage` | string | `validate`, `prepare`, `build`, or `package`; unique within array |
| `at_percent` | integer | 0–100; progress point within the stage |
| `message` | string | 1–200 chars; shown as the failure reason |
| `code` | string | Optional stable code, `[a-z0-9_]{3,40}`; defaults to `injected_build_failure` |

Stage durations are fixed engine values scaled by `duration_scale`; users cannot add arbitrary
stages in v1. Failure injections are deterministic, not random.

Profile expectations:

| Profile | Required non-empty entrypoint | Illustrative template |
| --- | --- | --- |
| `static-web` | `index.html` | `index.html`, `styles.css`, `README.md` |
| `http-service` | `src/service.js` | `src/service.js`, `README.md` |

These files are inert examples. “Build” does not parse or execute the language; it validates
presence, size, and scenario rules, then simulates stages and artifact metadata.

The image name is derived, not configured: repository `koncet/<project-id[0:8]>/app`, tag
`run-<run-id[0:8]>`. Derivation uses IDs rather than mutable project names so historical image
references stay stable. The UI always labels the result as a local simulated image.

### 3.5 `service`

| Field | Type | Rules |
| --- | --- | --- |
| `name` | string | `[a-z][a-z0-9-]{2,39}`, unique within the run |
| `command` | string | 1–200 chars; inert display text, never executed |
| `port` | integer | 1–65535; must equal `proxy.target_port` if the proxy is enabled |
| `cpu_millicores` | integer | 50–4,000 |
| `memory_mb` | integer | 32–4,096 |
| `env` | object | ≤ 50 entries; keys `[A-Z_][A-Z0-9_]*`; values ≤ 4 KiB; total ≤ 64 KiB |
| `startup_delay_s` | integer | 0–300; simulated time before health checks can pass |
| `restart_policy` | object | See below |
| `health_check` | object | See below |

Restart policy:

| Field | Type | Rules |
| --- | --- | --- |
| `policy` | `"no"` \| `"on-failure"` \| `"always"` | Required |
| `max_retries` | integer | 1–10; required for `on-failure`, ignored for `no`; `always` retries until the run ends |

Health check:

| Field | Type | Rules |
| --- | --- | --- |
| `path` | string | Starts with `/`, ≤ 200 chars; display/simulation only |
| `interval_s` | integer | 1–60 |
| `timeout_s` | integer | 1–30 and `< interval_s` |
| `retries` | integer | 1–10; consecutive failures before `unhealthy` |

### 3.6 `proxy`

| Field | Type | Rules |
| --- | --- | --- |
| `enabled` | boolean | If false, no route exists and the deployment cannot be end-to-end HEALTHY |
| `listener_port` | integer | 1–65535; unique among simulated sockets |
| `host` | string | lowercase DNS label(s) + `.koncet.local`; ≤ 253 chars; no scheme, port, or path |
| `path` | string | Starts with `/`; 1–200 chars; prefix match; no query/fragment |
| `target_port` | integer | Must equal `service.port` |

v1 supports exactly one listener and one route. No TLS, redirects, headers, rewrites, upstreams,
or load balancing. The URL shown in UI is always `http://<host><path>` and marked simulated.

### 3.7 `firewall`

| Field | Type | Rules |
| --- | --- | --- |
| `default_policy` | `"allow"` \| `"deny"` | Default `deny` |
| `rules` | array | ≤ 10 inbound rules; order is evaluation order |

Rule:

| Field | Type | Rules |
| --- | --- | --- |
| `id` | string | Unique; `[a-z][a-z0-9-]{2,40}` |
| `action` | `"allow"` \| `"deny"` | Required |
| `direction` | `"inbound"` | Only inbound in v1 |
| `protocol` | `"tcp"` \| `"udp"` | Must match simulated traffic (proxy uses tcp) |
| `port` | integer | 1–65535 (single port in v1) |
| `source` | `"proxy"` \| `"monitor"` \| `"external"` | Virtual source alias |

First matching rule wins; otherwise the default policy applies. `monitor` covers connectivity
checks; `external` is reserved for explicit simulated checks. Rules are declarative only and do
not touch host firewall state.

### 3.8 `faults`

v1 supports at most three deterministic faults:

| `kind` | Fields | Behaviour |
| --- | --- | --- |
| `process_crash` | `after_s` (1–3,600), `once` (bool, default true), `exit_code` (1–255, default 1) | Crashes the supervised process after the elapsed time since process start. `once=false` repeats with the same offset after each start; use carefully with `always`. |
| `healthcheck_failure` | `after_s` (0–3,600), `duration_s` (1–3,600) | Health checks fail during the interval, causing DEGRADED; process remains running. |
| `startup_stall` | `at_s` (0–300), `duration_s` (1–600) | Delays the simulated startup before the process becomes healthy. |

Rules:

- Fault times are relative to their documented origin and evaluated on ticks.
- Overlapping faults are allowed but may be rejected by validation if they produce impossible
  states; the validator reports the conflict.
- Faults must appear in logs/timeline with a stable code.
- Arbitrary network delay/loss, CPU stress, disk faults, and random chaos are out of scope for v1.

### 3.9 `monitoring`

| Field | Type | Rules |
| --- | --- | --- |
| `sample_interval_s` | integer | 1–60; default 5 |

Metrics are generated only while components are active; sampling stops when the run ends.
The engine caps memory/CPU-derived values to declared limits and rounds to fixed precision.

## 4. Validation and errors

Validation returns a list of issues with:

- stable code (`required`, `out_of_range`, `duplicate`, `invalid_format`, `incompatible`, …);
- JSON Pointer-like path (`/service/env/APP_ENV`);
- human message and optional suggested fix;
- severity: `error` (blocks run) or `warning` (allowed).

Blocking examples:

- missing allow rule from `proxy` to `service.port` when proxy is enabled → **warning**, because
  a DEGRADED run is a valid learning outcome; the UI explains it;
- duplicate firewall rule IDs, invalid host suffix, target port mismatch, invalid profile, more
  than one failure per stage, health timeout ≥ interval;
- `stop_on_terminal_failure` false is accepted but does not extend a failed build in v1.

Surprising-but-valid configurations remain runnable so users can learn from failure. Only
schema/invariant violations block the run.

## 5. Snapshotting and migration

- On run start, the canonical scenario JSON and schema version are stored on the run.
- Editing the stored scenario never changes a historical run.
- Future versions must provide `v1 → vN` migrations and retain read support for snapshots.
- Unknown event types and future optional fields in old snapshots are handled gracefully.
- Scenario portability/import is deferred; when added, it must include the schema version and
  validate before touching the database.

### 5.1 Baseline profile

When a project is created, its baseline `Happy Path` scenario inherits the project template's
build profile (`static-web` or `http-service`). New scenarios created from a preset in an
existing project also inherit that project's profile, so the entrypoint check matches the
template's source tree. The `Build Failure`, `Blocked Port`, and `Crash Loop` presets keep
their fault configuration and differ only in the profile adaptation.

## 6. Presets

| Preset | Key differences |
| --- | --- |
| Happy Path | Example above; proxy allow rule present; no faults |
| Build Failure | `failure_injections` with stage `build`, message “Injected compiler failure” |
| Blocked Port | Firewall rules empty (default deny) while proxy enabled → DEGRADED |
| Crash Loop | `process_crash` after 5 s, `on-failure` max 3 → restart exhaustion |

Presets are normal scenarios. Users may edit them; duplication keeps the original.
