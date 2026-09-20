//! UniFFI interface exposing `nv-core` to Kotlin (Android).
//!
//! FFI design decision: proc-macros (`#[uniffi::export]`, `#[derive(uniffi::Record
//! / Error / Object)]`) instead of a separate UDL file. Rationale: a UDL file
//! duplicates every signature by hand and drifts from the Rust code silently;
//! proc-macros keep a single source of truth checked by the compiler, and
//! `uniffi-bindgen --library` extracts the same metadata from the built cdylib.
//! See `android-bindings/README.md` for the exact bindings-generation command.
//!
//! Parity: this module adds NO storage behavior. It adapts the existing
//! `StorageManager` / `Note` / `search_notes` behind FFI-safe types, per
//! `docs/storage-contract.md`:
//! - paths cross FFI as `String` (never `PathBuf`),
//! - dates cross FFI as `i64` unix millis (never `DateTime<Local>`),
//! - notes cross FFI as the `NoteSnapshot` record (never `Vec<Note>`),
//! - failures surface as the `NvError` enum.
//!
//! `open` mirrors the desktop default (`Config::default`: extension `"md"`, and
//! a missing directory is created, never an error).

use crate::config::Config;
use crate::note::Note;
use crate::search::search_notes;
use crate::storage::StorageManager;
use chrono::{DateTime, Local};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

/// FFI-safe view of a note: filename stem identity, raw content, derived tags,
/// filesystem dates as unix millis. See storage-contract rules 3–6.
#[derive(Debug, Clone, uniffi::Record)]
pub struct NoteSnapshot {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub modified_ms: i64,
    pub created_ms: i64,
}

/// Failures a Kotlin caller can observe. I/O details are flattened to strings
/// because UniFFI error variants must be FFI-representable.
#[derive(Debug, uniffi::Error)]
pub enum NvError {
    NotFound(String),
    InvalidPath(String),
    Io(String),
}

impl std::fmt::Display for NvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "note not found: {id}"),
            Self::InvalidPath(dir) => write!(f, "invalid notes dir: {dir}"),
            Self::Io(msg) => write!(f, "i/o error: {msg}"),
        }
    }
}

impl std::error::Error for NvError {}

impl From<std::io::Error> for NvError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Pure adapter: `DateTime<Local>` → unix millis for FFI.
pub fn millis(dt: &DateTime<Local>) -> i64 {
    dt.timestamp_millis()
}

/// Pure adapter: internal `Note` → FFI `NoteSnapshot`.
pub fn snapshot_of(note: &Note) -> NoteSnapshot {
    NoteSnapshot {
        id: note.id.clone(),
        title: note.title.clone(),
        content: note.content.clone(),
        tags: note.tags.clone(),
        modified_ms: millis(&note.modified_at),
        created_ms: millis(&note.created_at),
    }
}

/// FFI object wrapping `StorageManager`. Interior mutability via `Mutex` because
/// UniFFI methods take `&self`; the lock is recovered (not panicked) on
/// poisoning so one failed call can't wedge the storage.
#[derive(uniffi::Object)]
pub struct NvStorage {
    inner: Mutex<StorageManager>,
}

impl NvStorage {
    fn lock(&self) -> MutexGuard<'_, StorageManager> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[uniffi::export]
impl NvStorage {
    /// Open (creating if needed) the notes dir at `dir`, then load it exactly
    /// like the desktop `StorageManager::new` + `reload`.
    #[uniffi::constructor]
    pub fn open(dir: String) -> Result<Arc<Self>, NvError> {
        if dir.trim().is_empty() {
            return Err(NvError::InvalidPath(dir));
        }
        let path = PathBuf::from(&dir);
        fs::create_dir_all(&path).map_err(NvError::from)?;
        let config = Config {
            notes_dir: path,
            default_extension: "md".to_string(),
            auto_save_ms: 300,
        };
        Ok(Arc::new(Self {
            inner: Mutex::new(StorageManager::new(&config)),
        }))
    }

    /// All notes, newest-modified first (contract rule 7).
    pub fn list_notes(&self) -> Vec<NoteSnapshot> {
        self.lock().notes.iter().map(snapshot_of).collect()
    }

    /// Ranked search via `search_notes`; snapshots follow score order.
    pub fn search_notes(&self, query: String) -> Vec<NoteSnapshot> {
        let inner = self.lock();
        search_notes(&inner.notes, &query)
            .iter()
            .filter_map(|id| inner.get_note(id).map(snapshot_of))
            .collect()
    }

    pub fn get_note(&self, id: String) -> Result<NoteSnapshot, NvError> {
        self.lock()
            .get_note(&id)
            .map(snapshot_of)
            .ok_or(NvError::NotFound(id))
    }

    /// Create a note (collision-proof like the desktop: never overwrites).
    pub fn create_note(&self, title: String) -> NoteSnapshot {
        snapshot_of(&self.lock().create_note(&title))
    }

    /// Overwrite content, refresh mtime/tags, re-sort; returns the updated note.
    pub fn save_note(&self, id: String, content: String) -> Result<NoteSnapshot, NvError> {
        let mut inner = self.lock();
        inner.save_note(&id, &content);
        inner
            .get_note(&id)
            .map(snapshot_of)
            .ok_or(NvError::NotFound(id))
    }

    /// Delete by id; `false` when the id is unknown (mirrors desktop).
    pub fn delete_note(&self, id: String) -> bool {
        self.lock().delete_note(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nv-ffi-test-{}-{}-{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn snapshot_maps_all_fields_with_millis_dates() {
        let dir = temp_dir("fields");
        let path = dir.join("hello.md");
        fs::write(&path, "hi #zeta #alpha").unwrap();
        let note = Note::from_file(&path).unwrap();
        let snap = snapshot_of(&note);
        assert_eq!(snap.id, "hello");
        assert_eq!(snap.title, "hello");
        assert_eq!(snap.content, "hi #zeta #alpha");
        assert_eq!(snap.tags, vec!["alpha", "zeta"]);
        assert_eq!(snap.modified_ms, note.modified_at.timestamp_millis());
        assert_eq!(snap.created_ms, note.created_at.timestamp_millis());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn error_display_mentions_context() {
        assert!(NvError::NotFound("x".into()).to_string().contains('x'));
        assert!(NvError::InvalidPath(String::new()).to_string().contains("invalid"));
        let io: NvError = std::io::Error::new(std::io::ErrorKind::Other, "boom").into();
        assert!(matches!(io, NvError::Io(_)));
    }

    #[test]
    fn open_creates_missing_dir_and_rejects_blank() {
        let dir = temp_dir("open");
        let missing = dir.join("does-not-exist");
        let storage = NvStorage::open(missing.to_string_lossy().into()).unwrap();
        assert!(missing.is_dir());
        assert!(storage.list_notes().is_empty());
        assert!(matches!(
            NvStorage::open("   ".to_string()),
            Err(NvError::InvalidPath(_))
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn crud_roundtrip_through_ffi_object() {
        let dir = temp_dir("crud");
        fs::write(dir.join("old.md"), "plain").unwrap();
        let storage = NvStorage::open(dir.to_string_lossy().into()).unwrap();

        let created = storage.create_note("fresh".to_string());
        assert_eq!(created.id, "fresh");
        assert_eq!(storage.list_notes().len(), 2);

        let got = storage.get_note("fresh".to_string()).unwrap();
        assert_eq!(got.title, "fresh");

        let saved = storage
            .save_note("fresh".to_string(), "updated #beta #alpha".to_string())
            .unwrap();
        assert_eq!(saved.tags, vec!["alpha", "beta"]);
        assert_eq!(storage.list_notes()[0].id, "fresh");

        assert!(storage.delete_note("fresh".to_string()));
        assert!(!storage.delete_note("fresh".to_string()));
        assert!(matches!(
            storage.get_note("fresh".to_string()),
            Err(NvError::NotFound(_))
        ));
        assert!(matches!(
            storage.save_note("ghost".to_string(), "x".to_string()),
            Err(NvError::NotFound(_))
        ));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn search_returns_ranked_snapshots() {
        let dir = temp_dir("search");
        fs::write(dir.join("shopping.md"), "buy milk eggs").unwrap();
        fs::write(dir.join("ideas.md"), "unrelated text").unwrap();
        let storage = NvStorage::open(dir.to_string_lossy().into()).unwrap();
        let hits = storage.search_notes("milk".to_string());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "shopping");
        assert_eq!(hits[0].content, "buy milk eggs");
        assert_eq!(storage.search_notes(String::new()).len(), 2);
        fs::remove_dir_all(&dir).ok();
    }
}
