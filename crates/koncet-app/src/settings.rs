//! Preferences, storage usage, backups, and diagnostics.

use std::collections::BTreeMap;
use std::path::Path;

use koncet_storage::{BackupInfo, BackupKind, StorageUsage};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::app::{App, AppInfo};
use crate::error::AppResult;

/// Diagnostics payload for Settings → About / Support.
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticsInfo {
    pub app: AppInfo,
    pub integrity_ok: bool,
    pub backup_count: usize,
    pub usage: StorageUsage,
}

impl App {
    pub fn get_settings(&self) -> AppResult<BTreeMap<String, Value>> {
        Ok(self.database.list_settings()?)
    }

    pub fn get_setting_json(&self, key: &str) -> AppResult<Option<Value>> {
        Ok(self.database.get_setting_json(key)?)
    }

    pub fn get_setting<T: DeserializeOwned>(&self, key: &str) -> AppResult<Option<T>> {
        Ok(self.database.get_setting(key)?)
    }

    pub fn set_setting<T: Serialize>(&mut self, key: &str, value: &T) -> AppResult<()> {
        let now = self.clock.now();
        self.database.set_setting(key, value, now)?;
        Ok(())
    }

    pub fn set_setting_json(&mut self, key: &str, value: &Value) -> AppResult<()> {
        let now = self.clock.now();
        self.database.set_setting_json(key, value, now)?;
        Ok(())
    }

    pub fn delete_setting(&mut self, key: &str) -> AppResult<bool> {
        Ok(self.database.delete_setting(key)?)
    }

    pub fn storage_usage(&self) -> AppResult<StorageUsage> {
        Ok(self.database.usage()?)
    }

    /// Writes a verified online backup and applies manual retention.
    pub fn create_manual_backup(&mut self) -> AppResult<BackupInfo> {
        let now = self.clock.now();
        let path = self.database.backup(BackupKind::Manual, now)?;
        let size_bytes = path.metadata().map(|meta| meta.len()).unwrap_or(0);
        Ok(BackupInfo {
            path,
            kind: BackupKind::Manual,
            created_at: now,
            size_bytes,
        })
    }

    pub fn list_backups(&self) -> AppResult<Vec<BackupInfo>> {
        Ok(self.database.list_backups()?)
    }

    pub fn delete_backup(&mut self, path: &Path) -> AppResult<()> {
        self.database.delete_backup(path)?;
        Ok(())
    }

    pub fn check_integrity(&self) -> AppResult<()> {
        Ok(self.database.integrity_check()?)
    }

    pub fn quick_check(&self) -> AppResult<()> {
        Ok(self.database.quick_check()?)
    }

    pub fn diagnostics(&self) -> AppResult<DiagnosticsInfo> {
        let app = self.info()?;
        let backups = self.database.list_backups()?;
        let usage = self.database.usage()?;
        Ok(DiagnosticsInfo {
            app,
            integrity_ok: self.database.quick_check().is_ok(),
            backup_count: backups.len(),
            usage,
        })
    }
}
