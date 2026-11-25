//! Database lifecycle: XDG paths, single-instance lock, WAL configuration,
//! integrity checks, migrations, backups, and maintenance.

use std::env;
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use koncet_domain::TimestampUtc;
use rusqlite::{Connection, OpenFlags};

use crate::error::{StorageError, StorageResult};
use crate::migrations;

/// Default maximum number of manual backups retained.
pub const DEFAULT_BACKUP_RETENTION: usize = 5;

/// Files managed by the storage layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoragePaths {
    root: PathBuf,
}

impl StoragePaths {
    /// Uses `$XDG_DATA_HOME/koncet`, falling back to `~/.local/share/koncet`.
    pub fn resolve() -> StorageResult<Self> {
        let base = env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .ok_or_else(|| {
                StorageError::Io(std::io::Error::new(
                    ErrorKind::NotFound,
                    "XDG_DATA_HOME and HOME are both unset",
                ))
            })?;
        Ok(Self::at(base.join("koncet")))
    }

    /// Explicit root, primarily for tests and portable diagnostics.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("koncet.db")
    }

    pub fn backups(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn lock(&self) -> PathBuf {
        self.root.join(".lock")
    }
}

/// Behaviour for opening the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenOptions {
    /// Remove a lock whose process no longer exists. Only set after user confirmation.
    pub break_stale_lock: bool,
    /// Run `PRAGMA quick_check` before using the database.
    pub verify_integrity: bool,
    /// Back up a non-empty database before applying pending migrations.
    pub backup_before_migration: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            break_stale_lock: false,
            verify_integrity: true,
            backup_before_migration: true,
        }
    }
}

/// Backup provenance shown in storage settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupKind {
    Manual,
    PreMigration,
}

impl BackupKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::PreMigration => "pre-migration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub kind: BackupKind,
    pub created_at: TimestampUtc,
    pub size_bytes: u64,
}

/// Aggregate storage counters for the settings screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StorageUsage {
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub backup_bytes: u64,
    pub project_count: u64,
    pub run_count: u64,
    pub event_count: u64,
    pub log_count: u64,
    pub metric_count: u64,
}

/// The single writer connection plus its instance lock.
pub struct Database {
    conn: Connection,
    paths: StoragePaths,
    _lock: InstanceLock,
}

impl Database {
    /// Opens the default XDG database.
    pub fn open_default() -> StorageResult<Self> {
        Self::open(StoragePaths::resolve()?)
    }

    /// Opens with default options.
    pub fn open(paths: StoragePaths) -> StorageResult<Self> {
        Self::open_with(paths, OpenOptions::default())
    }

    pub fn open_with(paths: StoragePaths, options: OpenOptions) -> StorageResult<Self> {
        prepare_directory(paths.root())?;
        prepare_directory(&paths.backups())?;

        let lock = InstanceLock::acquire(&paths.lock(), options.break_stale_lock)?;
        let database_path = paths.database();
        let mut conn = Connection::open(&database_path)?;
        restrict_file(&database_path)?;
        configure_connection(&conn)?;

        if options.verify_integrity {
            quick_check_connection(&conn)?;
        }

        let empty = migrations::database_is_empty(&conn)?;
        let version = migrations::current_version(&conn)?;
        if version > migrations::LATEST_SCHEMA_VERSION {
            return Err(StorageError::SchemaTooNew(version));
        }

        if !empty && version < migrations::LATEST_SCHEMA_VERSION && options.backup_before_migration
        {
            write_backup(
                &conn,
                &paths,
                BackupKind::PreMigration,
                TimestampUtc::from_millis(now_ms()),
            )?;
        }

        migrations::migrate(&mut conn)?;
        verify_foreign_keys(&conn)?;
        conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")?;

        Ok(Self {
            conn,
            paths,
            _lock: lock,
        })
    }

    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    pub fn schema_version(&self) -> StorageResult<i64> {
        migrations::current_version(&self.conn)
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    pub(crate) fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Flushes WAL pages to the main file. Called on pause, stop, and shutdown.
    pub fn checkpoint(&self) -> StorageResult<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |_| Ok(()))?;
        Ok(())
    }

    /// Verifies the database, then flushes and releases the lock.
    pub fn shutdown(self) -> StorageResult<()> {
        quick_check_connection(&self.conn)?;
        self.checkpoint()?;
        Ok(())
    }

    pub fn quick_check(&self) -> StorageResult<()> {
        quick_check_connection(&self.conn)
    }

    pub fn integrity_check(&self) -> StorageResult<()> {
        let result: String = self
            .conn
            .query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
        if result == "ok" {
            Ok(())
        } else {
            Err(StorageError::Corrupt(result))
        }
    }

    /// Writes a verified online backup and prunes retention.
    pub fn backup(&self, kind: BackupKind, at: TimestampUtc) -> StorageResult<PathBuf> {
        let path = write_backup(&self.conn, &self.paths, kind, at)?;
        self.prune_backups(DEFAULT_BACKUP_RETENTION)?;
        Ok(path)
    }

    pub fn list_backups(&self) -> StorageResult<Vec<BackupInfo>> {
        list_backups(&self.paths.backups())
    }

    pub fn delete_backup(&self, path: &Path) -> StorageResult<()> {
        let backups = self.paths.backups();
        let canonical_parent = path.parent();
        if canonical_parent != Some(backups.as_path()) {
            return Err(StorageError::Conflict(
                "backups can only be deleted from the managed directory".into(),
            ));
        }
        if !path.is_file() {
            return Err(StorageError::NotFound("backup"));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    /// Keeps the newest `keep` manual backups and the newest pre-migration backup.
    pub fn prune_backups(&self, keep: usize) -> StorageResult<()> {
        let backups = self.list_backups()?;
        let mut manual_seen = 0usize;
        let mut migration_seen = 0usize;
        for backup in backups {
            match backup.kind {
                BackupKind::Manual => {
                    manual_seen += 1;
                    if manual_seen > keep {
                        fs::remove_file(&backup.path)?;
                    }
                }
                BackupKind::PreMigration => {
                    migration_seen += 1;
                    if migration_seen > 1 {
                        fs::remove_file(&backup.path)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn usage(&self) -> StorageResult<StorageUsage> {
        let database = self.paths.database();
        let wal = database.with_extension("db-wal");
        let mut backup_bytes = 0u64;
        for backup in self.list_backups()? {
            backup_bytes = backup_bytes.saturating_add(backup.size_bytes);
        }
        Ok(StorageUsage {
            database_bytes: file_len(&database),
            wal_bytes: file_len(&wal),
            backup_bytes,
            project_count: count(&self.conn, "projects")?,
            run_count: count(&self.conn, "runs")?,
            event_count: count(&self.conn, "run_events")?,
            log_count: count(&self.conn, "log_lines")?,
            metric_count: count(&self.conn, "metric_samples")?,
        })
    }

    /// Opens a query-only connection for read paths outside the writer.
    pub fn reader(&self) -> StorageResult<ReadConnection> {
        ReadConnection::open(&self.paths)
    }
}

/// A query-only connection that never writes or runs migrations.
pub struct ReadConnection {
    conn: Connection,
}

impl ReadConnection {
    pub fn open(paths: &StoragePaths) -> StorageResult<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(paths.database(), flags)?;
        conn.pragma_update(None, "query_only", "ON")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn schema_version(&self) -> StorageResult<i64> {
        migrations::current_version(&self.conn)
    }
}

struct InstanceLock {
    path: PathBuf,
    token: String,
}

impl InstanceLock {
    fn acquire(path: &Path, break_stale: bool) -> StorageResult<Self> {
        let token = format!("{}-{}", std::process::id(), now_ms());
        for _ in 0..2 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
            {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    writeln!(file, "{token}")?;
                    file.sync_all()?;
                    return Ok(Self {
                        path: path.to_path_buf(),
                        token,
                    });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    let holder = read_lock(path)?;
                    match holder {
                        LockHolder::Live(pid) => return Err(StorageError::LockHeld { pid }),
                        LockHolder::Stale(pid) => {
                            if !break_stale {
                                return Err(StorageError::LockStale { pid });
                            }
                            fs::remove_file(path)?;
                        }
                        LockHolder::Unreadable => {
                            if !break_stale {
                                return Err(StorageError::Conflict(
                                    "the instance lock is unreadable; confirm removal and retry"
                                        .into(),
                                ));
                            }
                            fs::remove_file(path)?;
                        }
                    }
                }
                Err(error) => return Err(StorageError::Io(error)),
            }
        }
        Err(StorageError::Conflict(
            "could not acquire the instance lock".into(),
        ))
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return;
        };
        if contents.lines().nth(1) == Some(self.token.as_str()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

enum LockHolder {
    Live(u32),
    Stale(u32),
    Unreadable,
}

fn read_lock(path: &Path) -> StorageResult<LockHolder> {
    let contents = fs::read_to_string(path)?;
    let Some(pid) = contents
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
    else {
        return Ok(LockHolder::Unreadable);
    };
    if Path::new("/proc").join(pid.to_string()).exists() {
        Ok(LockHolder::Live(pid))
    } else {
        Ok(LockHolder::Stale(pid))
    }
}

fn configure_connection(conn: &Connection) -> StorageResult<()> {
    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::Conflict(format!(
            "SQLite refused WAL mode (got {journal_mode})"
        )));
    }
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "trusted_schema", "OFF")?;
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
    Ok(())
}

fn quick_check_connection(conn: &Connection) -> StorageResult<()> {
    let result: String = conn.query_row("PRAGMA quick_check(1);", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(StorageError::Corrupt(result))
    }
}

fn verify_foreign_keys(conn: &Connection) -> StorageResult<()> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check;")?;
    let mut rows = statement.query([])?;
    if rows.next()?.is_some() {
        return Err(StorageError::Corrupt(
            "foreign key check reported violations".into(),
        ));
    }
    Ok(())
}

fn prepare_directory(path: &Path) -> StorageResult<()> {
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn restrict_file(path: &Path) -> StorageResult<()> {
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn write_backup(
    conn: &Connection,
    paths: &StoragePaths,
    kind: BackupKind,
    at: TimestampUtc,
) -> StorageResult<PathBuf> {
    prepare_directory(&paths.backups())?;
    let mut target =
        paths
            .backups()
            .join(format!("koncet-{}-{}.db", kind.as_str(), at.as_millis()));
    let mut counter = 1u32;
    while target.exists() {
        target = paths.backups().join(format!(
            "koncet-{}-{}-{counter}.db",
            kind.as_str(),
            at.as_millis()
        ));
        counter += 1;
    }

    let target_text = target.to_string_lossy().to_string();
    if let Err(error) = conn.execute("VACUUM INTO ?1;", [target_text.as_str()]) {
        let _ = fs::remove_file(&target);
        return Err(StorageError::Sqlite(error));
    }
    restrict_file(&target)?;

    let verified = verify_backup_file(&target);
    if let Err(error) = verified {
        let _ = fs::remove_file(&target);
        return Err(error);
    }
    Ok(target)
}

fn verify_backup_file(path: &Path) -> StorageResult<()> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
    let result: String = conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(StorageError::Corrupt(format!(
            "backup failed integrity: {result}"
        )))
    }
}

fn list_backups(directory: &Path) -> StorageResult<Vec<BackupInfo>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(stem) = name
            .strip_prefix("koncet-")
            .and_then(|n| n.strip_suffix(".db"))
        else {
            continue;
        };
        let (kind, rest) = if let Some(rest) = stem.strip_prefix("pre-migration-") {
            (BackupKind::PreMigration, rest)
        } else if let Some(rest) = stem.strip_prefix("manual-") {
            (BackupKind::Manual, rest)
        } else {
            continue;
        };
        let Some(timestamp) = rest
            .split('-')
            .next()
            .and_then(|value| value.parse::<i64>().ok())
        else {
            continue;
        };
        let size_bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        backups.push(BackupInfo {
            path,
            kind,
            created_at: TimestampUtc::from_millis(timestamp),
            size_bytes,
        });
    }
    backups.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.path.cmp(&a.path))
    });
    Ok(backups)
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn count(conn: &Connection, table: &str) -> StorageResult<u64> {
    let sql = format!("SELECT COUNT(*) FROM {table};");
    let value: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(value.max(0) as u64)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}
