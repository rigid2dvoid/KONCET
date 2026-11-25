//! Strict scenario persistence. The canonical JSON is produced from the typed
//! scenario on every write, so the stored document can never drift.

use koncet_domain::{ProjectId, Scenario, ScenarioId, ScenarioRecord, TimestampUtc};
use rusqlite::{params, OptionalExtension, Row};

use crate::codec;
use crate::db::Database;
use crate::error::{StorageError, StorageResult};
use crate::repo::project::clamp_limit;

/// List row for the scenario picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSummary {
    pub id: ScenarioId,
    pub project_id: ProjectId,
    pub name: String,
    pub description: String,
    pub schema_version: u32,
    pub revision: u32,
    pub created_at: TimestampUtc,
    pub updated_at: TimestampUtc,
}

impl Database {
    /// Inserts a new scenario. Use [`Database::update_scenario`] for edits.
    pub fn create_scenario(&mut self, record: &ScenarioRecord) -> StorageResult<()> {
        let tx = self.conn_mut().transaction()?;
        insert_scenario_row(&tx, record)?;
        tx.commit()?;
        Ok(())
    }

    /// Updates a scenario only if the caller edited the current revision.
    pub fn update_scenario(
        &mut self,
        record: &ScenarioRecord,
        expected_revision: u32,
    ) -> StorageResult<()> {
        validate_record(record)?;
        if record.revision != expected_revision.saturating_add(1) {
            return Err(StorageError::Conflict(
                "scenario revision must increase by exactly one".into(),
            ));
        }
        let canonical = record.scenario.canonical_json();
        let changed = self.conn().execute(
            "UPDATE scenarios
             SET name = ?2, description = ?3, schema_version = ?4, revision = ?5,
                 config_json = ?6, updated_at = ?7
             WHERE id = ?1 AND project_id = ?8 AND revision = ?9",
            params![
                record.id.to_string(),
                record.scenario.name.trim(),
                record.scenario.description,
                record.schema_version as i64,
                record.revision as i64,
                canonical,
                record.updated_at.as_millis(),
                record.project_id.to_string(),
                expected_revision as i64,
            ],
        )?;
        if changed == 0 {
            return Err(StorageError::Conflict(
                "scenario changed since it was loaded".into(),
            ));
        }
        Ok(())
    }

    /// Inserts or bumps the revision in one call. Intended for simple authoring flows.
    pub fn save_scenario(&mut self, record: &mut ScenarioRecord) -> StorageResult<()> {
        let existing: Option<(u32, String)> = self
            .conn()
            .query_row(
                "SELECT revision, project_id FROM scenarios WHERE id = ?1",
                [record.id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match existing {
            None => {
                self.create_scenario(record)?;
                Ok(())
            }
            Some((revision, project_text)) => {
                let project_id: ProjectId = codec::parse_id(&project_text, "project id")?;
                if project_id != record.project_id {
                    return Err(StorageError::Conflict(
                        "scenario belongs to a different project".into(),
                    ));
                }
                record.revision = revision.saturating_add(1);
                self.update_scenario(record, revision)
            }
        }
    }

    pub fn load_scenario(&self, scenario_id: ScenarioId) -> StorageResult<Option<ScenarioRecord>> {
        let record = self
            .conn()
            .query_row(
                "SELECT id, project_id, revision, schema_version, config_json, created_at, updated_at
                 FROM scenarios WHERE id = ?1",
                [scenario_id.to_string()],
                map_scenario,
            )
            .optional()?;
        record.transpose()
    }

    pub fn list_scenarios(
        &self,
        project_id: ProjectId,
        limit: u32,
    ) -> StorageResult<Vec<ScenarioSummary>> {
        let mut statement = self.conn().prepare(
            "SELECT id, project_id, name, description, schema_version, revision, created_at, updated_at
             FROM scenarios WHERE project_id = ?1
             ORDER BY updated_at DESC, id DESC LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![project_id.to_string(), clamp_limit(limit)], |row| {
                Ok(ScenarioSummary {
                    id: codec::parse_id(&row.get::<_, String>(0)?, "scenario id")?,
                    project_id: codec::parse_id(&row.get::<_, String>(1)?, "project id")?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    schema_version: codec::positive(row.get(4)?, "schema version")?,
                    revision: codec::positive(row.get(5)?, "revision")?,
                    created_at: TimestampUtc::from_millis(row.get(6)?),
                    updated_at: TimestampUtc::from_millis(row.get(7)?),
                })
            })?;
        let mut scenarios = Vec::new();
        for row in rows {
            scenarios.push(row?);
        }
        Ok(scenarios)
    }

    /// Deletes an unreferenced scenario. A scenario used by history is kept.
    pub fn delete_scenario(&mut self, scenario_id: ScenarioId) -> StorageResult<()> {
        match self.conn().execute(
            "DELETE FROM scenarios WHERE id = ?1",
            [scenario_id.to_string()],
        ) {
            Ok(0) => Err(StorageError::NotFound("scenario")),
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(failure, _))
                if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(StorageError::Conflict(
                    "the scenario is referenced by run history and cannot be deleted".into(),
                ))
            }
            Err(error) => Err(StorageError::Sqlite(error)),
        }
    }
}

pub(crate) fn insert_scenario_row(
    conn: &rusqlite::Connection,
    record: &ScenarioRecord,
) -> StorageResult<()> {
    validate_record(record)?;
    conn.execute(
        "INSERT INTO scenarios(id, project_id, name, description, schema_version, revision,
                               config_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record.id.to_string(),
            record.project_id.to_string(),
            record.scenario.name.trim(),
            record.scenario.description,
            record.schema_version as i64,
            record.revision as i64,
            record.scenario.canonical_json(),
            record.created_at.as_millis(),
            record.updated_at.as_millis(),
        ],
    )?;
    Ok(())
}

fn validate_record(record: &ScenarioRecord) -> StorageResult<()> {
    if record.revision == 0 {
        return Err(StorageError::Validation(
            "scenario revision must be at least 1".into(),
        ));
    }
    if record.schema_version != record.scenario.schema_version {
        return Err(StorageError::Validation(
            "scenario schema version does not match its document".into(),
        ));
    }
    let report = record.scenario.validate();
    if let Some(error) = report.to_error() {
        return Err(StorageError::Validation(error.to_string()));
    }
    Ok(())
}

fn map_scenario(row: &Row<'_>) -> rusqlite::Result<StorageResult<ScenarioRecord>> {
    let id_text: String = row.get(0)?;
    let project_text: String = row.get(1)?;
    let revision: i64 = row.get(2)?;
    let schema_version: i64 = row.get(3)?;
    let config_json: String = row.get(4)?;
    let created_at: i64 = row.get(5)?;
    let updated_at: i64 = row.get(6)?;

    Ok((|| {
        let scenario = Scenario::from_json(&config_json)
            .map_err(|error| StorageError::Corrupt(format!("invalid scenario JSON: {error}")))?;
        let schema_version = codec::positive(schema_version, "schema version")?;
        if scenario.schema_version != schema_version {
            return Err(StorageError::Corrupt(
                "scenario schema version disagrees with its stored row".into(),
            ));
        }
        Ok(ScenarioRecord {
            id: codec::parse_id(&id_text, "scenario id")?,
            project_id: codec::parse_id(&project_text, "project id")?,
            revision: codec::positive(revision, "revision")?,
            schema_version,
            canonical_json: config_json,
            scenario,
            created_at: TimestampUtc::from_millis(created_at),
            updated_at: TimestampUtc::from_millis(updated_at),
        })
    })())
}
