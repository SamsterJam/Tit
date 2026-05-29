//! On-disk storage. Layout is byte-compatible with the original Python tool:
//!
//! ```text
//! $TIT_DATA_DIR (default ~/.local/share/tit)/
//!   HEAD                                   # current project name (or empty)
//!   projects/<name>/committed_sessions.json
//!   projects/<name>/uncommitted_sessions.json
//!   projects/<name>/deleted_sessions.json
//! ```

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::TitError;
use crate::model::{Commit, Session};

pub struct Store {
    base: PathBuf,
}

impl Store {
    pub fn open() -> Store {
        let base = match env::var_os("TIT_DATA_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => {
                let home = env::var("HOME").unwrap_or_default();
                Path::new(&home).join(".local/share/tit")
            }
        };
        Store { base }
    }

    fn head_file(&self) -> PathBuf {
        self.base.join("HEAD")
    }

    fn projects_dir(&self) -> PathBuf {
        self.base.join("projects")
    }

    fn project_dir(&self, name: &str) -> PathBuf {
        self.projects_dir().join(name)
    }

    pub fn project_exists(&self, name: &str) -> bool {
        self.project_dir(name).is_dir()
    }

    pub fn current_project(&self) -> Option<String> {
        let head = fs::read_to_string(self.head_file()).ok()?;
        let name = head.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    }

    pub fn set_current_project(&self, name: &str) -> Result<(), TitError> {
        let head = self.head_file();
        if let Some(parent) = head.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(head, name)?;
        Ok(())
    }

    /// Names of all projects, sorted.
    pub fn list_projects(&self) -> Vec<String> {
        let mut names: Vec<String> = match fs::read_dir(self.projects_dir()) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect(),
            Err(_) => Vec::new(),
        };
        names.sort();
        names
    }

    pub fn init_project(&self, name: &str) -> Result<(), TitError> {
        if self.project_exists(name) {
            return Err(TitError::ProjectExists(name.to_string()));
        }
        fs::create_dir_all(self.project_dir(name))?;
        self.set_current_project(name)?;
        Ok(())
    }

    pub fn delete_project(&self, name: &str) -> Result<(), TitError> {
        if !self.project_exists(name) {
            return Err(TitError::ProjectNotFound(name.to_string()));
        }
        fs::remove_dir_all(self.project_dir(name))?;
        if self.current_project().as_deref() == Some(name) {
            self.set_current_project("")?;
        }
        Ok(())
    }

    pub fn checkout_project(&self, name: &str) -> Result<(), TitError> {
        if !self.project_exists(name) {
            return Err(TitError::ProjectNotFound(name.to_string()));
        }
        self.set_current_project(name)
    }

    /// Load the current project, or error if none is selected.
    pub fn require_project(&self) -> Result<Project, TitError> {
        let name = self.current_project().ok_or(TitError::NoProject)?;
        Project::load(self.project_dir(&name), name)
    }
}

/// A loaded project: its three session pools plus the methods to persist them.
pub struct Project {
    pub name: String,
    dir: PathBuf,
    pub committed: Vec<Commit>,
    pub uncommitted: Vec<Session>,
    pub deleted: Vec<Commit>,
}

impl Project {
    fn load(dir: PathBuf, name: String) -> Result<Project, TitError> {
        let committed = read_json(&dir.join("committed_sessions.json"))?;
        let uncommitted = read_json(&dir.join("uncommitted_sessions.json"))?;
        let deleted = read_json(&dir.join("deleted_sessions.json"))?;
        Ok(Project {
            name,
            dir,
            committed,
            uncommitted,
            deleted,
        })
    }

    pub fn save_committed(&self) -> Result<(), TitError> {
        write_json(&self.dir.join("committed_sessions.json"), &self.committed)
    }

    pub fn save_uncommitted(&self) -> Result<(), TitError> {
        write_json(
            &self.dir.join("uncommitted_sessions.json"),
            &self.uncommitted,
        )
    }

    pub fn save_deleted(&self) -> Result<(), TitError> {
        write_json(&self.dir.join("deleted_sessions.json"), &self.deleted)
    }
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<T, TitError> {
    if !path.exists() {
        return Ok(T::default());
    }
    let text = fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(T::default());
    }
    Ok(serde_json::from_str(&text)?)
}

/// Write JSON pretty-printed with a 4-space indent (matching Python's
/// `json.dump(indent=4)`).
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), TitError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut ser)?;
    fs::write(path, buf)?;
    Ok(())
}
