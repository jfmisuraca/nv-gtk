use crate::app_state::timestamp_title_with_seconds;
use crate::config::Config;
use crate::note::Note;
use std::fs;
use std::path::PathBuf;

pub struct StorageManager {
    pub notes_dir: PathBuf,
    pub default_extension: String,
    pub notes: Vec<Note>,
}

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
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext == "md" || ext == "txt" || ext == "markdown" {
                            if let Ok(note) = Note::from_file(&path) {
                                self.notes.push(note);
                            }
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

    pub fn create_note(&mut self, title: &str) -> Note {
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
        let unique_title = if self.note_title_exists(display_title) {
            let mut candidate = timestamp_title_with_seconds();
            let mut counter = 1u32;
            while self.note_title_exists(&candidate) {
                candidate = format!("{}-{}", timestamp_title_with_seconds(), counter);
                counter += 1;
            }
            candidate
        } else {
            display_title.to_string()
        };

        let mut note = Note::new(&self.notes_dir, &unique_title, &self.default_extension);
        note.save().ok();
        self.notes.insert(0, note.clone());
        note
    }

    /// Devuelve `true` si ya existe una nota con ese nombre, en memoria
    /// (comparando sin distinguir mayúsculas, como antes) o como archivo en disco.
    fn note_title_exists(&self, title: &str) -> bool {
        self.notes
            .iter()
            .any(|n| n.title.eq_ignore_ascii_case(title))
            || self
                .notes_dir
                .join(format!("{}.{}", title, self.default_extension))
                .exists()
    }

    pub fn delete_note(&mut self, id: &str) -> bool {
        if let Some(idx) = self.notes.iter().position(|n| n.id == id) {
            let note = self.notes.remove(idx);
            fs::remove_file(&note.filepath).ok();
            true
        } else {
            false
        }
    }

    pub fn save_note(&mut self, id: &str, new_content: &str) {
        if let Some(note) = self.notes.iter_mut().find(|n| n.id == id) {
            if note.content != new_content {
                note.content = new_content.to_string();
                note.save().ok();
            }
        }
        self.notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
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
        storage.save_note("second", "updated #beta #alpha #beta");
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
        storage.save_note("second", "updated #beta #alpha #beta");
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
        let first = storage.create_note("20240919-1530");
        let second = storage.create_note("20240919-1530");
        let third = storage.create_note("20240919-1530");

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
}
