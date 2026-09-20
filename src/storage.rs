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
