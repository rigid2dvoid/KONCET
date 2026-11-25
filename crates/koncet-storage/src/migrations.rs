//! Embedded, checksummed migrations. Append-only; never edit an applied migration.
//!
//! The baseline schema is documented in `docs/architecture/data-model.md`. The
//! migration below follows that contract and includes the migration bookkeeping
//! table so a fresh database becomes self-describing in one transaction.

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::error::{StorageError, StorageResult};

pub(crate) struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub(crate) const LATEST_SCHEMA_VERSION: i64 = 1;

const MIGRATION_1: &str = r#"
CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    checksum   TEXT NOT NULL,
    applied_at INTEGER NOT NULL
) STRICT;

CREATE TABLE projects (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    description   TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 500),
    template_id   TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    archived_at   INTEGER
) STRICT;

CREATE TABLE source_files (
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    path          TEXT NOT NULL,
    language      TEXT NOT NULL,
    content       TEXT NOT NULL,
    byte_size     INTEGER NOT NULL CHECK (byte_size >= 0),
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (project_id, path)
) STRICT;

CREATE TABLE source_snapshots (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    content_hash  TEXT NOT NULL,
    file_count    INTEGER NOT NULL CHECK (file_count >= 0),
    total_bytes   INTEGER NOT NULL CHECK (total_bytes >= 0),
    created_at    INTEGER NOT NULL
) STRICT;
CREATE INDEX idx_snapshots_project ON source_snapshots(project_id, created_at DESC);

CREATE TABLE source_snapshot_files (
    snapshot_id   TEXT NOT NULL REFERENCES source_snapshots(id) ON DELETE CASCADE,
    path          TEXT NOT NULL,
    language      TEXT NOT NULL,
    content       TEXT NOT NULL,
    byte_size     INTEGER NOT NULL CHECK (byte_size >= 0),
    PRIMARY KEY (snapshot_id, path)
) STRICT;

CREATE TABLE scenarios (
    id             TEXT PRIMARY KEY,
    project_id     TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name           TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    description    TEXT NOT NULL DEFAULT '' CHECK (length(description) <= 500),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    revision       INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    config_json    TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL,
    UNIQUE (project_id, name)
) STRICT;
CREATE INDEX idx_scenarios_project ON scenarios(project_id, updated_at DESC);

CREATE TABLE runs (
    id                     TEXT PRIMARY KEY,
    project_id             TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    scenario_id            TEXT NOT NULL REFERENCES scenarios(id),
    scenario_name          TEXT NOT NULL,
    scenario_schema_version INTEGER NOT NULL CHECK (scenario_schema_version > 0),
    scenario_snapshot_json TEXT NOT NULL,
    source_snapshot_id     TEXT NOT NULL REFERENCES source_snapshots(id),
    seed                   TEXT NOT NULL,
    app_version            TEXT NOT NULL,
    engine_version         TEXT NOT NULL,
    status                 TEXT NOT NULL CHECK (status IN
                             ('running','paused','completed','cancelled','failed')),
    result                 TEXT NOT NULL CHECK (result IN
                             ('healthy','degraded','build_failed','runtime_failed','unknown')),
    started_at             INTEGER NOT NULL,
    ended_at               INTEGER,
    sim_duration_ms        INTEGER NOT NULL DEFAULT 0 CHECK (sim_duration_ms >= 0),
    last_event_seq         INTEGER NOT NULL DEFAULT 0 CHECK (last_event_seq >= 0),
    end_reason             TEXT,
    summary_json           TEXT
) STRICT;
CREATE INDEX idx_runs_project ON runs(project_id, started_at DESC);
CREATE INDEX idx_runs_history ON runs(started_at DESC, id DESC);

CREATE TABLE run_events (
    run_id         TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq            INTEGER NOT NULL CHECK (seq > 0),
    sim_time_ms    INTEGER NOT NULL CHECK (sim_time_ms >= 0),
    wall_time_ms   INTEGER NOT NULL,
    category       TEXT NOT NULL,
    event_type     TEXT NOT NULL,
    severity       TEXT NOT NULL CHECK (severity IN ('debug','info','warn','error')),
    source         TEXT NOT NULL,
    payload_json   TEXT NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1 CHECK (schema_version > 0),
    PRIMARY KEY (run_id, seq)
) STRICT;
CREATE INDEX idx_events_type ON run_events(run_id, event_type, seq);

CREATE TABLE run_commands (
    run_id        TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq           INTEGER NOT NULL CHECK (seq > 0),
    sim_time_ms   INTEGER NOT NULL CHECK (sim_time_ms >= 0),
    command_json  TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
) STRICT;

CREATE TABLE builds (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    ordinal         INTEGER NOT NULL CHECK (ordinal > 0),
    profile         TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN
                      ('queued','running','succeeded','failed','cancelled')),
    image_id        TEXT,
    started_sim_ms  INTEGER,
    finished_sim_ms INTEGER,
    failure_json    TEXT,
    created_at      INTEGER NOT NULL,
    UNIQUE (run_id, ordinal)
) STRICT;
CREATE INDEX idx_builds_run ON builds(run_id, ordinal);

CREATE TABLE build_stages (
    build_id        TEXT NOT NULL REFERENCES builds(id) ON DELETE CASCADE,
    ordinal         INTEGER NOT NULL CHECK (ordinal > 0),
    name            TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN
                      ('pending','running','succeeded','failed','skipped')),
    planned_ms      INTEGER NOT NULL CHECK (planned_ms > 0),
    started_sim_ms  INTEGER,
    finished_sim_ms INTEGER,
    failure_json    TEXT,
    PRIMARY KEY (build_id, ordinal)
) STRICT;

CREATE TABLE images (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    build_id        TEXT NOT NULL REFERENCES builds(id) ON DELETE CASCADE,
    repository      TEXT NOT NULL,
    tag             TEXT NOT NULL,
    digest          TEXT NOT NULL CHECK (length(digest) = 71 AND substr(digest, 1, 7) = 'sha256:'),
    size_bytes      INTEGER NOT NULL CHECK (size_bytes >= 0),
    layers_json     TEXT NOT NULL,
    profile         TEXT NOT NULL,
    source_hash     TEXT NOT NULL,
    engine_version  TEXT NOT NULL,
    created_sim_ms  INTEGER NOT NULL CHECK (created_sim_ms >= 0)
) STRICT;
CREATE INDEX idx_images_run ON images(run_id);

CREATE TABLE containers (
    id                TEXT PRIMARY KEY,
    run_id            TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    image_id          TEXT NOT NULL REFERENCES images(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    state             TEXT NOT NULL CHECK (state IN
                        ('created','starting','running','stopping','stopped','failed','removed')),
    health            TEXT NOT NULL CHECK (health IN
                        ('unknown','starting','healthy','unhealthy')),
    restart_count     INTEGER NOT NULL DEFAULT 0 CHECK (restart_count >= 0),
    config_json       TEXT NOT NULL,
    process_json      TEXT NOT NULL,
    created_sim_ms    INTEGER NOT NULL,
    started_sim_ms    INTEGER,
    stopped_sim_ms    INTEGER,
    exit_code         INTEGER,
    failure_reason    TEXT,
    UNIQUE (run_id, name)
) STRICT;
CREATE INDEX idx_containers_run ON containers(run_id);

CREATE TABLE deployments (
    id                   TEXT PRIMARY KEY,
    run_id               TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    image_id             TEXT NOT NULL,
    container_id         TEXT NOT NULL REFERENCES containers(id) ON DELETE CASCADE,
    service_name         TEXT NOT NULL,
    status               TEXT NOT NULL CHECK (status IN
                           ('pending','deploying','healthy','degraded','failed','stopped')),
    health               TEXT NOT NULL CHECK (health IN
                           ('unknown','starting','healthy','unhealthy')),
    primary_reason       TEXT,
    revision             INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    uptime_ms            INTEGER NOT NULL DEFAULT 0 CHECK (uptime_ms >= 0),
    healthy_since_sim_ms INTEGER,
    created_sim_ms       INTEGER NOT NULL,
    updated_sim_ms       INTEGER NOT NULL
) STRICT;
CREATE INDEX idx_deployments_run ON deployments(run_id);

CREATE TABLE log_lines (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq          INTEGER NOT NULL CHECK (seq > 0),
    sim_time_ms  INTEGER NOT NULL CHECK (sim_time_ms >= 0),
    source       TEXT NOT NULL,
    level        TEXT NOT NULL CHECK (level IN ('debug','info','warn','error')),
    message      TEXT NOT NULL,
    context_json TEXT,
    UNIQUE (run_id, seq)
) STRICT;
CREATE INDEX idx_logs_run_time ON log_lines(run_id, id);
CREATE INDEX idx_logs_run_level ON log_lines(run_id, level, id);
CREATE INDEX idx_logs_run_source ON log_lines(run_id, source, id);

CREATE TABLE metric_samples (
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    entity_type  TEXT NOT NULL,
    entity_id    TEXT NOT NULL,
    metric       TEXT NOT NULL,
    sim_time_ms  INTEGER NOT NULL CHECK (sim_time_ms >= 0),
    value        REAL NOT NULL,
    unit         TEXT NOT NULL,
    PRIMARY KEY (run_id, entity_type, entity_id, metric, sim_time_ms)
) STRICT;

CREATE TABLE app_settings (
    key         TEXT PRIMARY KEY,
    value_json  TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
) STRICT;
"#;

pub(crate) const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: MIGRATION_1,
}];

pub(crate) fn checksum(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

pub(crate) fn applied_versions(conn: &Connection) -> StorageResult<Vec<(i64, String)>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut stmt =
        conn.prepare("SELECT version, checksum FROM schema_migrations ORDER BY version")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Applies pending migrations in order. Returns true when anything was applied.
pub(crate) fn migrate(conn: &mut Connection) -> StorageResult<bool> {
    let applied = applied_versions(conn)?;
    if let Some((version, _)) = applied.last() {
        if *version > LATEST_SCHEMA_VERSION {
            return Err(StorageError::SchemaTooNew(*version));
        }
    }

    let pending: Vec<&Migration> = MIGRATIONS
        .iter()
        .filter(|migration| {
            !applied
                .iter()
                .any(|(version, _)| *version == migration.version)
        })
        .collect();
    let applied_any = !pending.is_empty();

    for migration in pending {
        let checksum = checksum(migration.sql);
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations(version, name, checksum, applied_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![migration.version, migration.name, checksum, now_ms()],
        )?;
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;
    }

    for (version, stored) in applied_versions(conn)? {
        if let Some(migration) = MIGRATIONS.iter().find(|m| m.version == version) {
            if checksum(migration.sql) != stored {
                return Err(StorageError::MigrationChecksum { version });
            }
        } else {
            return Err(StorageError::SchemaTooNew(version));
        }
    }
    Ok(applied_any)
}

pub(crate) fn database_is_empty(conn: &Connection) -> StorageResult<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    Ok(count == 0)
}

pub(crate) fn current_version(conn: &Connection) -> StorageResult<i64> {
    let version: Option<i64> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .optional()
        .unwrap_or(None);
    Ok(version.unwrap_or(0))
}

pub(crate) fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
