//! Projects, virtual source trees, and immutable source snapshots.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{DomainError, ValidationReport};
use crate::id::{ProjectId, SourceSnapshotId};
use crate::time::TimestampUtc;

/// Maximum number of files in a source tree.
pub const MAX_SOURCE_FILES: usize = 200;
/// Maximum bytes per source file.
pub const MAX_SOURCE_FILE_BYTES: usize = 256 * 1024;
/// Maximum total bytes across a source tree.
pub const MAX_SOURCE_TOTAL_BYTES: usize = 5 * 1024 * 1024;
/// Maximum directory depth of a source path.
pub const MAX_SOURCE_DEPTH: usize = 16;
/// Maximum characters in a project name.
pub const MAX_PROJECT_NAME_CHARS: usize = 80;
/// Maximum characters in a project description.
pub const MAX_PROJECT_DESCRIPTION_CHARS: usize = 500;

/// Built-in starter templates. Informational after project creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectTemplate {
    StaticWeb,
    HttpService,
}

impl ProjectTemplate {
    pub const ALL: [Self; 2] = [Self::StaticWeb, Self::HttpService];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaticWeb => "static-web",
            Self::HttpService => "http-service",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::StaticWeb => "Static web",
            Self::HttpService => "HTTP service",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::StaticWeb => "A small static site served by one simulated process.",
            Self::HttpService => "A Node-style HTTP service with a health endpoint.",
        }
    }

    /// Starter files. These are inert examples; KONCET never executes them.
    pub fn seed_files(self) -> Vec<SourceFile> {
        match self {
            Self::StaticWeb => vec![
                SourceFile::new(
                    "index.html",
                    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>KONCET static web</title>
    <link rel="stylesheet" href="styles.css" />
  </head>
  <body>
    <main>
      <p class="eyebrow">Simulated deployment</p>
      <h1>Hello from KONCET</h1>
      <p>This page is part of a virtual source tree. Nothing is served on a real network.</p>
    </main>
  </body>
</html>
"#,
                ),
                SourceFile::new(
                    "styles.css",
                    r#":root {
  color-scheme: light dark;
  font-family: system-ui, sans-serif;
}

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: #101418;
  color: #e8eef4;
}

main {
  max-width: 36rem;
  padding: 2rem;
}

.eyebrow {
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #7cc7a4;
  font-size: 0.75rem;
}

h1 {
  font-size: clamp(2rem, 6vw, 3.5rem);
  margin: 0.5rem 0 1rem;
}
"#,
                ),
                SourceFile::new(
                    "README.md",
                    "# Static web example\n\nThis project exists only inside KONCET. Build and deployment are simulated.\n",
                ),
            ],
            Self::HttpService => vec![
                SourceFile::new(
                    "src/service.js",
                    r#"// Inert example source. KONCET does not execute this file.
const http = require("node:http");

const port = Number(process.env.PORT || 3000);

const server = http.createServer((request, response) => {
  if (request.url === "/healthz") {
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ status: "ok" }));
    return;
  }

  response.writeHead(200, { "content-type": "text/plain" });
  response.end("Hello from the KONCET simulated service\n");
});

server.listen(port, "127.0.0.1");
"#,
                ),
                SourceFile::new(
                    "README.md",
                    "# HTTP service example\n\nThe command and port are simulated. No socket is opened.\n",
                ),
            ],
        }
    }
}

impl fmt::Display for ProjectTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A project is the root of authoring data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    pub template_id: ProjectTemplate,
    pub created_at: TimestampUtc,
    pub updated_at: TimestampUtc,
    pub archived_at: Option<TimestampUtc>,
}

impl Project {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        template: ProjectTemplate,
        now: TimestampUtc,
    ) -> Result<Self, DomainError> {
        let name = normalise_name(name.into())?;
        let description = description.into();
        validate_description(&description)?;
        Ok(Self {
            id: ProjectId::new(),
            name,
            description,
            template_id: template,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    pub fn rename(
        &mut self,
        name: impl Into<String>,
        now: TimestampUtc,
    ) -> Result<(), DomainError> {
        self.ensure_editable()?;
        self.name = normalise_name(name.into())?;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_description(
        &mut self,
        description: impl Into<String>,
        now: TimestampUtc,
    ) -> Result<(), DomainError> {
        self.ensure_editable()?;
        let description = description.into();
        validate_description(&description)?;
        self.description = description;
        self.updated_at = now;
        Ok(())
    }

    pub fn archive(&mut self, now: TimestampUtc) -> Result<(), DomainError> {
        if self.archived_at.is_some() {
            return Err(DomainError::conflict(
                "project_already_archived",
                "the project is already archived",
            ));
        }
        self.archived_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    pub fn unarchive(&mut self, now: TimestampUtc) -> Result<(), DomainError> {
        if self.archived_at.take().is_none() {
            return Err(DomainError::conflict(
                "project_not_archived",
                "the project is not archived",
            ));
        }
        self.updated_at = now;
        Ok(())
    }

    pub const fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    pub fn ensure_editable(&self) -> Result<(), DomainError> {
        if self.is_archived() {
            return Err(DomainError::conflict(
                "project_archived",
                "archived projects are read-only; unarchive the project first",
            ));
        }
        Ok(())
    }

    /// Trims and validates a candidate project name.
    pub fn validate_name(name: &str) -> Result<String, DomainError> {
        normalise_name(name.to_string())
    }
}

fn normalise_name(name: String) -> Result<String, DomainError> {
    let trimmed = name.trim();
    let chars = trimmed.chars().count();
    if trimmed.is_empty() {
        return Err(DomainError::validation(
            "required",
            "/name",
            "project name is required",
        ));
    }
    if chars > MAX_PROJECT_NAME_CHARS {
        return Err(DomainError::validation(
            "too_long",
            "/name",
            format!("project name must be at most {MAX_PROJECT_NAME_CHARS} characters"),
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DomainError::validation(
            "invalid_format",
            "/name",
            "project name must not contain control characters",
        ));
    }
    Ok(trimmed.to_string())
}

fn validate_description(description: &str) -> Result<(), DomainError> {
    if description.chars().count() > MAX_PROJECT_DESCRIPTION_CHARS {
        return Err(DomainError::validation(
            "too_long",
            "/description",
            format!(
                "project description must be at most {MAX_PROJECT_DESCRIPTION_CHARS} characters"
            ),
        ));
    }
    Ok(())
}

/// A text file in a virtual source tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }

    pub fn byte_len(&self) -> usize {
        self.content.len()
    }

    pub fn language_hint(&self) -> &'static str {
        language_hint(&self.path)
    }
}

/// A validated, ordered set of source files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTree {
    files: Vec<SourceFile>,
}

impl SourceTree {
    pub fn new(files: Vec<SourceFile>) -> Result<Self, ValidationReport> {
        let mut report = ValidationReport::new();
        let mut seen = BTreeMap::new();
        let mut total_bytes = 0usize;

        if files.len() > MAX_SOURCE_FILES {
            report.error(
                "constraint",
                "/files",
                format!("a source tree may contain at most {MAX_SOURCE_FILES} files"),
            );
        }

        for (index, file) in files.iter().enumerate() {
            let pointer = format!("/files/{index}");
            if let Err((code, message)) = validate_source_path(&file.path) {
                report.error(code, format!("{pointer}/path"), message);
            }
            if seen.insert(file.path.as_str(), index).is_some() {
                report.error(
                    "duplicate",
                    format!("{pointer}/path"),
                    format!("duplicate source path: {}", file.path),
                );
            }
            if file.content.len() > MAX_SOURCE_FILE_BYTES {
                report.error(
                    "constraint",
                    format!("{pointer}/content"),
                    format!(
                        "source file {} exceeds the {} KiB limit",
                        file.path,
                        MAX_SOURCE_FILE_BYTES / 1024
                    ),
                );
            }
            if file.content.contains('\0') {
                report.error(
                    "invalid_format",
                    format!("{pointer}/content"),
                    "binary content is not supported; remove NUL bytes",
                );
            }
            total_bytes = total_bytes.saturating_add(file.content.len());
        }

        if total_bytes > MAX_SOURCE_TOTAL_BYTES {
            report.error(
                "constraint",
                "/files",
                format!(
                    "source tree exceeds the total {} MiB limit",
                    MAX_SOURCE_TOTAL_BYTES / (1024 * 1024)
                ),
            );
        }

        if !report.is_valid() {
            return Err(report);
        }

        let mut files = files;
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self { files })
    }

    pub fn empty() -> Self {
        Self { files: Vec::new() }
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub fn get(&self, path: &str) -> Option<&SourceFile> {
        self.files.iter().find(|file| file.path == path)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    pub fn total_bytes(&self) -> usize {
        self.files.iter().map(SourceFile::byte_len).sum()
    }

    /// Content hash independent of file order and editor state.
    ///
    /// SHA-256 over canonical JSON of sorted `(path, content)` pairs.
    pub fn content_hash(&self) -> String {
        let entries: Vec<(&str, &str)> = self
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.content.as_str()))
            .collect();
        hash_entries(&entries)
    }

    pub fn validate_entrypoint(&self, entrypoint: &str) -> Result<(), DomainError> {
        if self.contains(entrypoint) {
            return Ok(());
        }
        Err(DomainError::validation(
            "missing_entrypoint",
            "/files",
            format!("the build profile requires a non-empty {entrypoint}"),
        ))
    }
}

impl Default for SourceTree {
    fn default() -> Self {
        Self::empty()
    }
}

fn hash_entries(entries: &[(&str, &str)]) -> String {
    // `serde_json` uses a BTreeMap-backed value map and preserves slice order for
    // arrays, so this encoding is stable for the same sorted entries.
    let bytes = serde_json::to_vec(entries).expect("source entries are always serialisable");
    let digest = Sha256::digest(bytes);
    hex_lower(&digest)
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Validates a relative POSIX source path. Returns a stable code and message.
pub fn validate_source_path(path: &str) -> Result<(), (&'static str, String)> {
    if path.is_empty() {
        return Err(("required", "path is required".to_string()));
    }
    if path.len() > 255 {
        return Err(("too_long", "path must be at most 255 bytes".to_string()));
    }
    if path.contains('\0') {
        return Err(("invalid_format", "path must not contain NUL".to_string()));
    }
    if path.chars().any(|c| c.is_control()) {
        return Err((
            "invalid_format",
            "path must not contain control characters".to_string(),
        ));
    }
    if path.contains('\\') {
        return Err((
            "invalid_format",
            "path must use forward slashes, not backslashes".to_string(),
        ));
    }
    if path.starts_with('/') {
        return Err(("invalid_format", "path must be relative".to_string()));
    }
    if path.ends_with('/') {
        return Err((
            "invalid_format",
            "path must name a file, not a directory".to_string(),
        ));
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() > MAX_SOURCE_DEPTH {
        return Err((
            "constraint",
            format!("path depth must be at most {MAX_SOURCE_DEPTH} segments"),
        ));
    }
    for segment in segments {
        if segment.is_empty() {
            return Err((
                "invalid_format",
                "path must not contain empty segments".to_string(),
            ));
        }
        if segment == "." || segment == ".." {
            return Err((
                "invalid_format",
                "path must not contain '.' or '..' segments".to_string(),
            ));
        }
        if segment.len() > 128 {
            return Err((
                "too_long",
                "each path segment must be at most 128 bytes".to_string(),
            ));
        }
    }
    Ok(())
}

fn language_hint(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") | Some("htm") => "html",
        Some("css") => "css",
        Some("js") | Some("mjs") | Some("cjs") => "javascript",
        Some("ts") => "typescript",
        Some("jsx") => "javascript-jsx",
        Some("tsx") => "typescript-jsx",
        Some("json") => "json",
        Some("md") | Some("markdown") => "markdown",
        Some("rs") => "rust",
        Some("py") => "python",
        Some("sh") => "shell",
        Some("yml") | Some("yaml") => "yaml",
        Some("toml") => "toml",
        Some("txt") => "text",
        _ => "text",
    }
}

/// A file captured inside an immutable snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub path: String,
    pub language: String,
    pub content: String,
    pub content_hash: String,
    pub bytes: u64,
}

impl SnapshotFile {
    fn from_source(file: &SourceFile) -> Self {
        let digest = Sha256::digest(file.content.as_bytes());
        Self {
            path: file.path.clone(),
            language: file.language_hint().to_string(),
            content: file.content.clone(),
            content_hash: hex_lower(&digest),
            bytes: file.content.len() as u64,
        }
    }
}

/// Immutable input captured at run start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub id: SourceSnapshotId,
    pub project_id: ProjectId,
    pub files: Vec<SnapshotFile>,
    pub content_hash: String,
    pub total_bytes: u64,
    pub created_at: TimestampUtc,
}

impl SourceSnapshot {
    /// Captures a source tree. Callers must persist the result before starting a run.
    pub fn capture(project_id: ProjectId, tree: &SourceTree, created_at: TimestampUtc) -> Self {
        let files: Vec<SnapshotFile> = tree.files.iter().map(SnapshotFile::from_source).collect();
        let entries: Vec<(&str, &str)> = tree
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.content.as_str()))
            .collect();
        Self {
            id: SourceSnapshotId::new(),
            project_id,
            files,
            content_hash: hash_entries(&entries),
            total_bytes: tree.total_bytes() as u64,
            created_at,
        }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn to_source_tree(&self) -> SourceTree {
        SourceTree {
            files: self
                .files
                .iter()
                .map(|file| SourceFile::new(file.path.clone(), file.content.clone()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(files: &[(&str, &str)]) -> SourceTree {
        SourceTree::new(
            files
                .iter()
                .map(|(path, content)| SourceFile::new(*path, *content))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        for path in [
            "../secret",
            "/etc/passwd",
            "a//b",
            "a/./b",
            "a\\b",
            "",
            "a/",
        ] {
            assert!(
                validate_source_path(path).is_err(),
                "{path} should be rejected"
            );
        }
        assert!(validate_source_path("src/service.js").is_ok());
        assert!(validate_source_path("index.html").is_ok());
    }

    #[test]
    fn hash_is_independent_of_input_order() {
        let a = tree(&[("index.html", "hello"), ("styles.css", "body{}")]);
        let b = tree(&[("styles.css", "body{}"), ("index.html", "hello")]);
        assert_eq!(a.content_hash(), b.content_hash());
        assert_eq!(a.files()[0].path, "index.html");
    }

    #[test]
    fn limits_are_enforced_without_panicking() {
        let big = "x".repeat(MAX_SOURCE_FILE_BYTES + 1);
        let result = SourceTree::new(vec![SourceFile::new("a.txt", big)]);
        assert!(result.is_err());
    }

    #[test]
    fn snapshot_survives_source_edits() {
        let project = Project::new(
            "Demo",
            "",
            ProjectTemplate::StaticWeb,
            TimestampUtc::from_millis(1),
        )
        .unwrap();
        let original = tree(&[("index.html", "before")]);
        let snapshot = SourceSnapshot::capture(project.id, &original, TimestampUtc::from_millis(2));
        let edited = tree(&[("index.html", "after")]);
        assert_ne!(snapshot.content_hash, edited.content_hash());
        assert_eq!(snapshot.files[0].content, "before");
    }

    #[test]
    fn names_are_trimmed_and_bounded() {
        let project = Project::new(
            "  Demo  ",
            "",
            ProjectTemplate::StaticWeb,
            TimestampUtc::from_millis(0),
        )
        .unwrap();
        assert_eq!(project.name, "Demo");
        assert!(Project::validate_name("   ").is_err());
        assert!(Project::validate_name(&"x".repeat(81)).is_err());
    }
}
