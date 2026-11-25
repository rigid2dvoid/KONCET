//! Project, source-tree, and immutable snapshot persistence.

use koncet_domain::{
    Project, ProjectId, ProjectTemplate, SnapshotFile, SourceFile, SourceSnapshot,
    SourceSnapshotId, SourceTree, TimestampUtc,
};
use rusqlite::{params, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::codec;
use crate::db::Database;
use crate::error::{StorageError, StorageResult};

/// Project together with its editable virtual source tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectWithFiles {
    pub project: Project,
    pub files: SourceTree,
}

/// List row for the project picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummary {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    pub template_id: ProjectTemplate,
    pub created_at: TimestampUtc,
    pub updated_at: TimestampUtc,
    pub archived_at: Option<TimestampUtc>,
    pub file_count: u64,
    pub total_bytes: u64,
}

/// Snapshot metadata without file contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSummary {
    pub id: SourceSnapshotId,
    pub project_id: ProjectId,
    pub content_hash: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub created_at: TimestampUtc,
}

pub const DEFAULT_LIST_LIMIT: u32 = 200;
pub const MAX_LIST_LIMIT: u32 = 1_000;

pub fn clamp_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_LIST_LIMIT)
}

impl Database {
    /// Inserts a project and its initial source tree in one transaction.
    pub fn create_project(&mut self, project: &Project, files: &SourceTree) -> StorageResult<()> {
        self.create_project_with_scenario(project, files, None)
    }

    /// Creates a project, its source tree, and an optional baseline scenario atomically.
    pub fn create_project_with_scenario(
        &mut self,
        project: &Project,
        files: &SourceTree,
        scenario: Option<&koncet_domain::ScenarioRecord>,
    ) -> StorageResult<()> {
        let scenarios: Vec<_> = scenario.into_iter().cloned().collect();
        self.create_project_with_scenarios(project, files, &scenarios)
    }

    /// Creates a project together with any number of scenarios in one transaction.
    pub fn create_project_with_scenarios(
        &mut self,
        project: &Project,
        files: &SourceTree,
        scenarios: &[koncet_domain::ScenarioRecord],
    ) -> StorageResult<()> {
        if scenarios
            .iter()
            .any(|record| record.project_id != project.id)
        {
            return Err(StorageError::Validation(
                "a baseline scenario belongs to a different project".into(),
            ));
        }
        let tx = self.conn_mut().transaction()?;
        insert_project_row(&tx, project)?;
        replace_files_rows(&tx, project.id, files, project.created_at)?;
        for record in scenarios {
            crate::repo::scenario::insert_scenario_row(&tx, record)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Updates mutable project metadata. Source files are replaced separately.
    pub fn update_project(&mut self, project: &Project) -> StorageResult<()> {
        let changed = self.conn().execute(
            "UPDATE projects
             SET name = ?2, description = ?3, updated_at = ?4, archived_at = ?5
             WHERE id = ?1",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.updated_at.as_millis(),
                project.archived_at.map(TimestampUtc::as_millis),
            ],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound("project"));
        }
        Ok(())
    }

    /// Replaces the entire editable source tree and bumps `updated_at`.
    pub fn replace_source_files(
        &mut self,
        project_id: ProjectId,
        files: &SourceTree,
        now: TimestampUtc,
    ) -> StorageResult<()> {
        let tx = self.conn_mut().transaction()?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1)",
            [project_id.to_string()],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(StorageError::NotFound("project"));
        }
        replace_files_rows(&tx, project_id, files, now)?;
        tx.execute(
            "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
            params![project_id.to_string(), now.as_millis()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_project(&self, project_id: ProjectId) -> StorageResult<Option<ProjectWithFiles>> {
        let project = self
            .conn()
            .query_row(
                "SELECT id, name, description, template_id, created_at, updated_at, archived_at
                 FROM projects WHERE id = ?1",
                [project_id.to_string()],
                map_project,
            )
            .optional()?;
        let Some(project) = project else {
            return Ok(None);
        };
        let mut statement = self.conn().prepare(
            "SELECT path, content FROM source_files WHERE project_id = ?1 ORDER BY path",
        )?;
        let rows = statement.query_map([project_id.to_string()], |row| {
            Ok(SourceFile::new(
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })?;
        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }
        let files = SourceTree::new(files).map_err(|report| {
            StorageError::Corrupt(format!("stored source tree is invalid: {report:?}"))
        })?;
        Ok(Some(ProjectWithFiles { project, files }))
    }

    pub fn list_projects(
        &self,
        include_archived: bool,
        limit: u32,
    ) -> StorageResult<Vec<ProjectSummary>> {
        let sql = if include_archived {
            "SELECT p.id, p.name, p.description, p.template_id, p.created_at, p.updated_at,
                    p.archived_at,
                    (SELECT COUNT(*) FROM source_files f WHERE f.project_id = p.id),
                    (SELECT COALESCE(SUM(f.byte_size), 0) FROM source_files f WHERE f.project_id = p.id)
             FROM projects p ORDER BY p.updated_at DESC, p.id DESC LIMIT ?1"
        } else {
            "SELECT p.id, p.name, p.description, p.template_id, p.created_at, p.updated_at,
                    p.archived_at,
                    (SELECT COUNT(*) FROM source_files f WHERE f.project_id = p.id),
                    (SELECT COALESCE(SUM(f.byte_size), 0) FROM source_files f WHERE f.project_id = p.id)
             FROM projects p WHERE p.archived_at IS NULL
             ORDER BY p.updated_at DESC, p.id DESC LIMIT ?1"
        };
        let mut statement = self.conn().prepare(sql)?;
        let rows = statement.query_map([clamp_limit(limit)], map_project_summary)?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    pub fn delete_project(&mut self, project_id: ProjectId) -> StorageResult<()> {
        let changed = self.conn().execute(
            "DELETE FROM projects WHERE id = ?1",
            [project_id.to_string()],
        )?;
        if changed == 0 {
            return Err(StorageError::NotFound("project"));
        }
        Ok(())
    }

    /// Persists an immutable snapshot. Call inside a run-start transaction when possible.
    pub fn insert_source_snapshot(&mut self, snapshot: &SourceSnapshot) -> StorageResult<()> {
        let tx = self.conn_mut().transaction()?;
        insert_snapshot_rows(&tx, snapshot)?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_source_snapshot(
        &self,
        snapshot_id: SourceSnapshotId,
    ) -> StorageResult<Option<SourceSnapshot>> {
        let header = self
            .conn()
            .query_row(
                "SELECT id, project_id, content_hash, file_count, total_bytes, created_at
                 FROM source_snapshots WHERE id = ?1",
                [snapshot_id.to_string()],
                |row| {
                    Ok((
                        codec::parse_id::<SourceSnapshotId>(
                            &row.get::<_, String>(0)?,
                            "snapshot id",
                        )?,
                        codec::parse_id::<ProjectId>(&row.get::<_, String>(1)?, "project id")?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                        TimestampUtc::from_millis(row.get::<_, i64>(5)?),
                    ))
                },
            )
            .optional()?;
        let Some((id, project_id, content_hash, file_count, total_bytes, created_at)) = header
        else {
            return Ok(None);
        };

        let mut statement = self.conn().prepare(
            "SELECT path, language, content, byte_size
             FROM source_snapshot_files WHERE snapshot_id = ?1 ORDER BY path",
        )?;
        let rows = statement.query_map([snapshot_id.to_string()], |row| {
            let content: String = row.get(2)?;
            let digest = Sha256::digest(content.as_bytes());
            Ok(SnapshotFile {
                path: row.get(0)?,
                language: row.get(1)?,
                bytes: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                content_hash: hex_lower(&digest),
                content,
            })
        })?;
        let mut files = Vec::new();
        for row in rows {
            files.push(row?);
        }

        let actual_bytes: u64 = files.iter().map(|file| file.bytes).sum();
        if files.len() as i64 != file_count || actual_bytes != total_bytes as u64 {
            return Err(StorageError::Corrupt(format!(
                "snapshot {snapshot_id} metadata does not match its files"
            )));
        }

        Ok(Some(SourceSnapshot {
            id,
            project_id,
            files,
            content_hash,
            total_bytes: total_bytes.max(0) as u64,
            created_at,
        }))
    }

    pub fn list_source_snapshots(
        &self,
        project_id: ProjectId,
        limit: u32,
    ) -> StorageResult<Vec<SnapshotSummary>> {
        let mut statement = self.conn().prepare(
            "SELECT id, project_id, content_hash, file_count, total_bytes, created_at
             FROM source_snapshots WHERE project_id = ?1
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )?;
        let rows =
            statement.query_map(params![project_id.to_string(), clamp_limit(limit)], |row| {
                Ok(SnapshotSummary {
                    id: codec::parse_id(&row.get::<_, String>(0)?, "snapshot id")?,
                    project_id: codec::parse_id(&row.get::<_, String>(1)?, "project id")?,
                    content_hash: row.get(2)?,
                    file_count: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                    total_bytes: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
                    created_at: TimestampUtc::from_millis(row.get::<_, i64>(5)?),
                })
            })?;
        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row?);
        }
        Ok(snapshots)
    }
}

pub(crate) fn insert_project_row(
    conn: &rusqlite::Connection,
    project: &Project,
) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO projects(id, name, description, template_id, created_at, updated_at, archived_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            project.id.to_string(),
            project.name,
            project.description,
            project.template_id.as_str(),
            project.created_at.as_millis(),
            project.updated_at.as_millis(),
            project.archived_at.map(TimestampUtc::as_millis),
        ],
    )?;
    Ok(())
}

pub(crate) fn replace_files_rows(
    conn: &rusqlite::Connection,
    project_id: ProjectId,
    files: &SourceTree,
    now: TimestampUtc,
) -> StorageResult<()> {
    conn.execute(
        "DELETE FROM source_files WHERE project_id = ?1",
        [project_id.to_string()],
    )?;
    let mut statement = conn.prepare(
        "INSERT INTO source_files(project_id, path, language, content, byte_size, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    )?;
    for file in files.files() {
        statement.execute(params![
            project_id.to_string(),
            file.path,
            file.language_hint(),
            file.content,
            file.content.len() as i64,
            now.as_millis(),
        ])?;
    }
    Ok(())
}

pub(crate) fn insert_snapshot_rows(
    conn: &rusqlite::Connection,
    snapshot: &SourceSnapshot,
) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO source_snapshots(id, project_id, content_hash, file_count, total_bytes, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            snapshot.id.to_string(),
            snapshot.project_id.to_string(),
            snapshot.content_hash,
            snapshot.file_count() as i64,
            snapshot.total_bytes as i64,
            snapshot.created_at.as_millis(),
        ],
    )?;
    let mut statement = conn.prepare(
        "INSERT INTO source_snapshot_files(snapshot_id, path, language, content, byte_size)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for file in &snapshot.files {
        statement.execute(params![
            snapshot.id.to_string(),
            file.path,
            file.language,
            file.content,
            file.bytes as i64,
        ])?;
    }
    Ok(())
}

fn map_project(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: codec::parse_id(&row.get::<_, String>(0)?, "project id").map_err(to_sql_error)?,
        name: row.get(1)?,
        description: row.get(2)?,
        template_id: codec::parse_enum(&row.get::<_, String>(3)?, "project template")
            .map_err(to_sql_error)?,
        created_at: TimestampUtc::from_millis(row.get(4)?),
        updated_at: TimestampUtc::from_millis(row.get(5)?),
        archived_at: row.get::<_, Option<i64>>(6)?.map(TimestampUtc::from_millis),
    })
}

fn map_project_summary(row: &Row<'_>) -> rusqlite::Result<ProjectSummary> {
    Ok(ProjectSummary {
        id: codec::parse_id(&row.get::<_, String>(0)?, "project id").map_err(to_sql_error)?,
        name: row.get(1)?,
        description: row.get(2)?,
        template_id: codec::parse_enum(&row.get::<_, String>(3)?, "project template")
            .map_err(to_sql_error)?,
        created_at: TimestampUtc::from_millis(row.get(4)?),
        updated_at: TimestampUtc::from_millis(row.get(5)?),
        archived_at: row.get::<_, Option<i64>>(6)?.map(TimestampUtc::from_millis),
        file_count: u64::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
        total_bytes: u64::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
    })
}

fn to_sql_error(error: StorageError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
