//! Application services for KONCET.
//!
//! This crate owns use cases, the single active run session, persistence
//! ordering, and event fan-out. It performs no presentation work and exposes no
//! SQL or database rows; the FFI layer maps these types to bridge DTOs.

mod app;
mod clock;
mod error;
mod history;
mod projects;
mod runs;
mod scenarios;
mod session;
mod settings;

pub use app::{App, AppInfo};
pub use clock::{Clock, SystemClock};
pub use error::{AppError, AppResult};
pub use projects::{CreateProjectInput, TemplateInfo};
pub use runs::{parse_seed, RunHandle, StartRunInput};
pub use scenarios::PresetInfo;
pub use session::{RunFrame, RunUpdate, MAX_TICKS_PER_ADVANCE};
pub use settings::DiagnosticsInfo;

// Stable read models shared with the bridge. The FFI re-exports these through
// `koncet-app`, so it never needs to depend on storage directly.
pub use koncet_storage::{
    BackupInfo, BackupKind, LogFilter, OpenOptions, PersistedRun, ProjectSummary, ProjectWithFiles,
    RunHistoryFilter, RunHistoryItem, ScenarioSummary, SnapshotSummary, StorageUsage,
};
