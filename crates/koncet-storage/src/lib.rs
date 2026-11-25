//! SQLite persistence for KONCET.
//!
//! The writer owns one connection and one process lock; readers use separate
//! query-only connections. Schema changes are embedded, checksummed migrations.
//! Simulation state is stored as snapshots plus append-only event, log, and
//! metric streams so the application can reconstruct any run.

mod codec;
mod db;
mod error;
mod migrations;
mod repo;

pub use db::{
    BackupInfo, BackupKind, Database, OpenOptions, ReadConnection, StoragePaths, StorageUsage,
    DEFAULT_BACKUP_RETENTION,
};
pub use error::{StorageError, StorageResult};
pub use repo::project::{
    ProjectSummary, ProjectWithFiles, SnapshotSummary, DEFAULT_LIST_LIMIT, MAX_LIST_LIMIT,
};
pub use repo::run::{LogFilter, PersistedRun, RunHistoryFilter, RunHistoryItem, RunState};
pub use repo::scenario::ScenarioSummary;
pub use repo::settings::{validate_key as validate_setting_key, MAX_SETTING_BYTES};
