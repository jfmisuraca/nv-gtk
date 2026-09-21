use crate::config::Config;
use crate::note::Note;
use crate::util::timestamp_title_with_seconds;
use std::fs;
use std::path::PathBuf;

pub struct StorageManager {
    pub notes_dir: PathBuf,
    pub default_extension: String,
    pub notes: Vec<Note>,
}

/// App trash subdirectory: deleted notes rest here (as plain files, same
/// formats as the notes dir) until restored or purged. Hidden so file
/// managers skip it; `reload` never loads from it.
pub const TRASH_DIR_NAME: &str = ".trash";

impl StorageManager {
    pub fn new(config: &Config) -> Self {
        let mut mgr = Self {
            notes_dir: config.notes_dir.clone(),
            default_extension: config.default_extension.clone(),
            notes: Vec::new(),
        };
        mgr.reload();
        mgr
    }

    pub fn reload(&mut self) {
        self.notes.clear();
        if let Ok(entries) = fs::read_dir(&self.notes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if is_note_file(&path) {
                        if let Ok(note) = Note::from_file(&path) {
                            self.notes.push(note);
                        }
                    }
                }
            }
        }

        // Sort by modified date descending (newest first)
        self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    }

    pub fn get_note(&self, id: &str) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    /// Creates the note file on disk and inserts it at the front.
    /// Returns `Err` (and inserts nothing) when the file cannot be written,
    /// so callers can tell persistence failure apart from success.
    pub fn create_note(&mut self, title: &str) -> std::io::Result<Note> {
        let title_clean = title.trim();
        let display_title = if title_clean.is_empty() {
            "Untitled"
        } else {
            title_clean
        };

        // Colisión de nombres: dos notas creadas en el mismo minuto comparten el
        // timestamp base AAAAMMDD-HHMM. En ese caso la nueva se crea con segundos
        // (AAAAMMDD-HHMMSS), y con un contador si ese también existe. Así nunca se
        // devuelve ni se sobreescribe la nota original.
        let unique_title = if self.title_taken(display_title, None) {
            let mut candidate = timestamp_title_with_seconds();
            let mut counter = 1u32;
            while self.title_taken(&candidate, None) {
                candidate = format!("{}-{}", timestamp_title_with_seconds(), counter);
                counter += 1;
            }
            candidate
        } else {
            display_title.to_string()
        };

        let mut note = Note::new(&self.notes_dir, &unique_title, &self.default_extension);
        note.save()?;
        self.notes.insert(0, note.clone());
        Ok(note)
    }

    /// Renames a note (new filename stem): moves the file and updates
    /// id/title. Blank titles become "Untitled", `/` becomes `-`, and taken
    /// names disambiguate timestamp-style like creation (excluding the note
    /// itself, so case-only renames work).
    /// Content, tags and dates are preserved: `rename` keeps mtime, so the
    /// list order does not move. `Ok(None)` when the id is unknown. On move
    /// failure the note is kept as-is and `Err` is returned.
    pub fn rename_note(&mut self, id: &str, new_title: &str) -> std::io::Result<Option<Note>> {
        let Some(idx) = self.notes.iter().position(|n| n.id == id) else {
            return Ok(None);
        };
        let clean = new_title.trim();
        let want = if clean.is_empty() {
            "Untitled".to_string()
        } else {
            clean.replace('/', "-")
        };
        if want == self.notes[idx].title {
            return Ok(Some(self.notes[idx].clone()));
        }
        let final_title = if self.title_taken(&want, Some(id)) {
            let mut candidate = timestamp_title_with_seconds();
            let mut counter = 1u32;
            while self.title_taken(&candidate, Some(id)) {
                candidate = format!("{}-{counter}", timestamp_title_with_seconds());
                counter += 1;
            }
            candidate
        } else {
            want
        };
        let mut note = self.notes.remove(idx);
        let target = self
            .notes_dir
            .join(format!("{final_title}.{}", self.default_extension));
        if let Err(e) = fs::rename(&note.filepath, &target) {
            self.notes.push(note);
            self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
            return Err(e);
        }
        note.filepath = target;
        note.id = final_title.clone();
        note.title = final_title;
        self.notes.push(note.clone());
        self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        Ok(Some(note))
    }

    /// Whether `title` is taken by another note (memory, case-insensitive)
    /// or by a file on disk. `exclude_id` skips one note (for renames).
    fn title_taken(&self, title: &str, exclude_id: Option<&str>) -> bool {
        self.notes
            .iter()
            .any(|n| Some(n.id.as_str()) != exclude_id && n.title.eq_ignore_ascii_case(title))
            || self
                .notes_dir
                .join(format!("{}.{}", title, self.default_extension))
                .exists()
    }

    /// Moves the note file to the app trash (same filesystem `rename`, so
    /// dates are preserved) and drops it from memory.
    /// `Ok(false)` when the id is unknown. On move failure the note is kept
    /// in memory and `Err` is returned.
    pub fn delete_note(&mut self, id: &str) -> std::io::Result<bool> {
        if let Some(idx) = self.notes.iter().position(|n| n.id == id) {
            let note = self.notes.remove(idx);
            let moved = (|| -> std::io::Result<()> {
                let trash = self.ensure_trash_dir()?;
                let target = unique_path(&trash, &note_filename(&note));
                fs::rename(&note.filepath, &target)?;
                Ok(())
            })();
            if let Err(e) = moved {
                self.notes.push(note);
                self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
                return Err(e);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Trashed notes, newest-modified first. Never touches `self.notes`.
    pub fn trash_notes(&self) -> Vec<Note> {
        let mut out = Vec::new();
        if let Ok(entries) = fs::read_dir(self.trash_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_note_file(&path) {
                    if let Ok(note) = Note::from_file(&path) {
                        out.push(note);
                    }
                }
            }
        }
        out.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        out
    }

    /// Moves a trashed note back to the notes dir. When its name was retaken
    /// meanwhile, the restored note gets a disambiguated id (timestamp-based,
    /// like creation) instead of overwriting.
    /// `Ok(None)` when the id is not in the trash.
    pub fn restore_note(&mut self, id: &str) -> std::io::Result<Option<Note>> {
        let trashed = self.trash_notes();
        let Some(entry) = trashed.iter().find(|n| n.id == id) else {
            return Ok(None);
        };
        let file_name = note_filename(entry);
        let mut target = self.notes_dir.join(&file_name);
        if target.exists() || self.notes.iter().any(|n| n.id == entry.id) {
            let mut candidate = timestamp_title_with_seconds();
            let mut counter = 1u32;
            while self.title_taken(&candidate, None) {
                candidate = format!("{}-{}", timestamp_title_with_seconds(), counter);
                counter += 1;
            }
            target = self
                .notes_dir
                .join(format!("{}.{}", candidate, self.default_extension));
        }
        fs::rename(&entry.filepath, &target)?;
        let note = Note::from_file(&target)?;
        self.notes.push(note.clone());
        self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        Ok(Some(note))
    }

    /// Permanently deletes one trashed note. `Ok(false)` when unknown.
    pub fn purge_note(&mut self, id: &str) -> std::io::Result<bool> {
        let trashed = self.trash_notes();
        if let Some(entry) = trashed.iter().find(|n| n.id == id) {
            fs::remove_file(&entry.filepath)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Permanently deletes everything in the trash. Returns the purged count.
    /// Stops at the first failure, reporting `Err` (earlier purges stand).
    pub fn empty_trash(&mut self) -> std::io::Result<u64> {
        let mut count = 0u64;
        for entry in self.trash_notes() {
            fs::remove_file(&entry.filepath)?;
            count += 1;
        }
        Ok(count)
    }

    fn trash_dir(&self) -> PathBuf {
        self.notes_dir.join(TRASH_DIR_NAME)
    }

    fn ensure_trash_dir(&self) -> std::io::Result<PathBuf> {
        let dir = self.trash_dir();
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Overwrites the note file when content changed, then re-sorts newest-first.
    /// `Ok(false)` when the id is unknown. On write failure the in-memory
    /// content is kept (caller's edits are preserved) and `Err` is returned.
    pub fn save_note(&mut self, id: &str, new_content: &str) -> std::io::Result<bool> {
        let outcome = match self.notes.iter().position(|n| n.id == id) {
            Some(idx) => {
                if self.notes[idx].content != new_content {
                    self.notes[idx].content = new_content.to_string();
                    self.notes[idx].save().map(|()| true)
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        };
        self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        outcome
    }
}

/// Case-sensitive format filter shared by `reload` and `trash_notes`
/// (contract rule 2): exactly `md`, `txt` or `markdown`.
fn is_note_file(path: &std::path::Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "txt" | "markdown")
    )
}

/// File name (`<stem>.<ext>`) of a note on disk.
fn note_filename(note: &Note) -> String {
    note.filepath
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}.md", note.id))
}

/// `dir/file_name`, or `dir/<stem>-trash-<n>.<ext>` while taken, so a move
/// never overwrites an existing file.
fn unique_path(dir: &std::path::Path, file_name: &str) -> PathBuf {
    let mut target = dir.join(file_name);
    if !target.exists() {
        return target;
    }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (file_name.to_string(), String::new()),
    };
    let mut counter = 2u32;
    loop {
        target = dir.join(format!("{stem}-trash-{counter}{ext}"));
        if !target.exists() {
            return target;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_notes_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nv-gtk-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_storage(dir: &std::path::Path) -> StorageManager {
        StorageManager {
            notes_dir: dir.to_path_buf(),
            default_extension: "md".to_string(),
            notes: Vec::new(),
        }
    }

    fn write_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn backdate(path: &std::path::Path, secs_ago: u64) {
        use std::time::{Duration, SystemTime};
        let past = SystemTime::now() - Duration::from_secs(secs_ago);
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(past)
            .unwrap();
    }

    #[test]
    fn rule1_new_loads_notes_from_configured_dir() {
        let dir = temp_notes_dir();
        fs::write(dir.join("hello.md"), "hi").unwrap();
        let config = Config {
            notes_dir: dir.clone(),
            default_extension: "md".to_string(),
            auto_save_ms: 300,
        };
        let storage = StorageManager::new(&config);
        assert_eq!(storage.notes_dir, dir);
        assert_eq!(storage.notes.len(), 1);
        assert_eq!(storage.notes[0].id, "hello");

        // Missing directory: empty list, no panic.
        let ghost_cfg = Config {
            notes_dir: dir.join("does-not-exist"),
            default_extension: "md".to_string(),
            auto_save_ms: 300,
        };
        let ghost_storage = StorageManager::new(&ghost_cfg);
        assert!(ghost_storage.notes.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule2_only_md_txt_markdown_load() {
        let dir = temp_notes_dir();
        write_file(&dir, "a.md", "a");
        write_file(&dir, "b.txt", "b");
        write_file(&dir, "c.markdown", "c");
        write_file(&dir, "d.rs", "d");
        write_file(&dir, "e.MD", "e");
        write_file(&dir, "Makefile", "f");
        let mut storage = test_storage(&dir);
        storage.reload();
        let mut ids: Vec<_> = storage.notes.iter().map(|n| n.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule7_reload_orders_by_modified_desc() {
        let dir = temp_notes_dir();
        let old_path = write_file(&dir, "old.md", "old");
        write_file(&dir, "new.md", "new");
        backdate(&old_path, 120);
        let mut storage = test_storage(&dir);
        storage.reload();
        assert_eq!(storage.notes.len(), 2);
        assert_eq!(storage.notes[0].id, "new");
        assert_eq!(storage.notes[1].id, "old");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule8_save_note_overwrites_updates_tags_and_resorts() {
        let dir = temp_notes_dir();
        let first_path = write_file(&dir, "first.md", "plain #alpha");
        let second_path = write_file(&dir, "second.md", "plain");
        backdate(&second_path, 120);
        let mut storage = test_storage(&dir);
        storage.reload();
        assert_eq!(storage.notes[0].id, "first");

        // Saving the older note with new content overwrites its file,
        // re-derives tags, and moves it to the front.
        assert!(storage.save_note("second", "updated #beta #alpha #beta").unwrap());
        assert_eq!(storage.notes[0].id, "second");
        assert_eq!(
            storage.notes[0].tags,
            vec!["alpha".to_string(), "beta".to_string()]
        );
        assert_eq!(
            fs::read_to_string(&second_path).unwrap(),
            "updated #beta #alpha #beta"
        );
        assert_eq!(fs::read_to_string(&first_path).unwrap(), "plain #alpha");

        // Saving identical content keeps the note in place at the front.
        assert!(storage
            .save_note("second", "updated #beta #alpha #beta")
            .unwrap());
        assert_eq!(storage.notes[0].id, "second");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn same_title_creates_notes_without_overwriting() {
        let dir = temp_notes_dir();
        let mut storage = StorageManager {
            notes_dir: dir.clone(),
            default_extension: "md".to_string(),
            notes: Vec::new(),
        };
        storage.reload();

        // Simula dos notas creadas en el mismo minuto: mismo título base.
        let first = storage.create_note("20240919-1530").unwrap();
        let second = storage.create_note("20240919-1530").unwrap();
        let third = storage.create_note("20240919-1530").unwrap();

        assert_eq!(first.title, "20240919-1530");
        assert_ne!(
            second.title, first.title,
            "la segunda nota debe desambiguarse con segundos"
        );
        assert_ne!(third.title, first.title);
        assert_ne!(third.title, second.title);
        assert_eq!(storage.notes.len(), 3);

        // Cada nota vive en su propio archivo: guardar en una no pisa a la otra.
        assert_ne!(first.filepath, second.filepath);
        assert!(first.filepath.exists());
        assert!(second.filepath.exists());
        assert!(third.filepath.exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unknown_ids_report_false_without_touching_disk() {
        let dir = temp_notes_dir();
        write_file(&dir, "only.md", "content");
        let mut storage = test_storage(&dir);
        storage.reload();
        assert_eq!(storage.save_note("ghost", "x").unwrap(), false);
        assert_eq!(storage.delete_note("ghost").unwrap(), false);
        assert_eq!(storage.notes.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn io_failures_surface_instead_of_silent_ghosts() {
        // Create: notes_dir points at a regular file, so no note file can be written.
        let dir = temp_notes_dir();
        let blocker = write_file(&dir, "blocker.md", "x");
        let mut bad = test_storage(&blocker);
        assert!(bad.create_note("fresh").is_err());
        assert!(bad.notes.is_empty());

        // Save/delete: note file replaced by a directory, so writing or
        // removing it via file APIs fails deterministically.
        let dir2 = temp_notes_dir();
        write_file(&dir2, "victim.md", "original");
        let mut storage = test_storage(&dir2);
        storage.reload();
        fs::remove_file(dir2.join("victim.md")).unwrap();
        fs::create_dir(dir2.join("victim.md")).unwrap();

        storage.save_note("victim", "updated").unwrap_err();
        // In-memory content is kept (the caller's edits are preserved).
        assert_eq!(storage.get_note("victim").unwrap().content, "updated");

        // Delete: notes_dir points at a file, so the trash cannot be created.
        let dir3 = temp_notes_dir();
        let blocker3 = write_file(&dir3, "blocker.md", "x");
        let mut storage3 = test_storage(&blocker3);
        let mut ghost = Note::new(&blocker3, "ghost", "md");
        ghost.content = "data".to_string();
        storage3.notes.push(ghost);
        storage3.delete_note("ghost").unwrap_err();
        // The note stays listed, matching what is still on disk.
        assert!(storage3.get_note("ghost").is_some());

        fs::remove_dir(&dir2.join("victim.md")).ok();
        fs::remove_dir_all(&dir).ok();
        fs::remove_dir_all(&dir2).ok();
        fs::remove_dir_all(&dir3).ok();
    }

    #[test]
    fn delete_moves_to_trash_and_restore_roundtrips() {
        let dir = temp_notes_dir();
        write_file(&dir, "a.md", "hello #x");
        let mut storage = test_storage(&dir);
        storage.reload();

        assert!(storage.delete_note("a").unwrap());
        assert!(storage.notes.is_empty());
        assert!(!dir.join("a.md").exists());

        let trash = storage.trash_notes();
        assert_eq!(trash.len(), 1);
        assert_eq!(trash[0].id, "a");
        assert_eq!(trash[0].content, "hello #x");
        assert_eq!(trash[0].tags, vec!["x"]);

        let restored = storage.restore_note("a").unwrap().unwrap();
        assert_eq!(restored.id, "a");
        assert_eq!(storage.notes.len(), 1);
        assert!(storage.trash_notes().is_empty());
        assert!(dir.join("a.md").exists());

        assert!(storage.restore_note("ghost").unwrap().is_none());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restore_disambiguates_when_name_retaken() {
        let dir = temp_notes_dir();
        write_file(&dir, "a.md", "original");
        let mut storage = test_storage(&dir);
        storage.reload();
        assert!(storage.delete_note("a").unwrap());

        // Meanwhile the name is taken by a brand-new note.
        write_file(&dir, "a.md", "replacement");
        storage.reload();
        assert_eq!(storage.notes.len(), 1);

        let restored = storage.restore_note("a").unwrap().unwrap();
        assert_ne!(restored.id, "a");
        assert_eq!(restored.content, "original");
        assert_eq!(storage.notes.len(), 2);
        assert!(storage.trash_notes().is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn purge_and_empty_trash() {        let dir = temp_notes_dir();
        write_file(&dir, "a.md", "a");
        write_file(&dir, "b.md", "b");
        let mut storage = test_storage(&dir);
        storage.reload();
        assert!(storage.delete_note("a").unwrap());
        assert!(storage.delete_note("b").unwrap());
        assert_eq!(storage.trash_notes().len(), 2);

        assert!(storage.purge_note("a").unwrap());
        assert!(!storage.purge_note("a").unwrap());
        assert_eq!(storage.trash_notes().len(), 1);

        assert_eq!(storage.empty_trash().unwrap(), 1);
        assert!(storage.trash_notes().is_empty());
        assert_eq!(storage.empty_trash().unwrap(), 0);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_moves_file_and_preserves_content_tags_and_dates() {
        let dir = temp_notes_dir();
        let path = write_file(&dir, "old.md", "body #beta #alpha");
        backdate(&path, 120);
        let before_mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let mut storage = test_storage(&dir);
        storage.reload();

        let renamed = storage.rename_note("old", "new").unwrap().unwrap();
        assert_eq!(renamed.id, "new");
        assert_eq!(renamed.title, "new");
        assert_eq!(renamed.content, "body #beta #alpha");
        assert_eq!(renamed.tags, vec!["alpha", "beta"]);
        // Same filesystem move: mtime preserved, list order untouched.
        assert_eq!(
            fs::metadata(dir.join("new.md"))
                .unwrap()
                .modified()
                .unwrap(),
            before_mtime
        );
        assert!(!dir.join("old.md").exists());
        assert!(storage.get_note("old").is_none());
        assert_eq!(storage.get_note("new").unwrap().content, "body #beta #alpha");

        // Unknown id and same-name no-op.
        assert!(storage.rename_note("ghost", "x").unwrap().is_none());
        let same = storage.rename_note("new", "new").unwrap().unwrap();
        assert_eq!(same.id, "new");
        assert!(dir.join("new.md").exists());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rename_disambiguates_collisions_and_sanitizes() {
        let dir = temp_notes_dir();
        write_file(&dir, "a.md", "a");
        write_file(&dir, "b.md", "b");
        let mut storage = test_storage(&dir);
        storage.reload();

        // Taken name: never overwrites, gets a distinct id.
        let renamed = storage.rename_note("a", "b").unwrap().unwrap();
        assert_ne!(renamed.id, "b");
        assert_ne!(renamed.id, "a");
        assert_eq!(fs::read_to_string(dir.join("b.md")).unwrap(), "b");
        assert_eq!(renamed.content, "a");
        assert_eq!(storage.notes.len(), 2);

        // Blank becomes Untitled; slashes are sanitized.
        let untitled = storage.rename_note(&renamed.id, "   ").unwrap().unwrap();
        assert_eq!(untitled.id, "Untitled");
        let slashed = storage.rename_note("b", "x/y").unwrap().unwrap();
        assert_eq!(slashed.id, "x-y");

        // Case-only rename works (self excluded from the taken check).
        let cased = storage.rename_note("x-y", "X-Y").unwrap().unwrap();
        assert_eq!(cased.id, "X-Y");

        fs::remove_dir_all(&dir).ok();
    }
}
