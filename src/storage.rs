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

        // Check if note already exists
        if let Some(existing) = self.notes.iter().find(|n| n.title.eq_ignore_ascii_case(display_title)) {
            return existing.clone();
        }

        let mut note = Note::new(&self.notes_dir, display_title, &self.default_extension);
        note.save().ok();
        self.notes.insert(0, note.clone());
        note
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
