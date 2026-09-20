use chrono::{DateTime, Local};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub filepath: PathBuf,
    pub content: String,
    pub modified_at: DateTime<Local>,
    pub created_at: DateTime<Local>,
    pub tags: Vec<String>,
}
impl Note {
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = fs::read_to_string(path)?;
        let metadata = fs::metadata(path)?;
        let modified_sys = metadata.modified().unwrap_or_else(|_| SystemTime::now());
        let modified_at: DateTime<Local> = DateTime::from(modified_sys);
        let created_sys = metadata.created().unwrap_or(modified_sys);
        let created_at: DateTime<Local> = DateTime::from(created_sys);
        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let tags = Self::extract_tags(&content);
        let id = filename.clone();
        Ok(Self {
            id,
            title: filename,
            filepath: path.to_path_buf(),
            content,
            modified_at,
            created_at,
            tags,
        })
    }
    pub fn new(notes_dir: &Path, title: &str, extension: &str) -> Self {
        let safe_title = title.replace('/', "-");
        let filename = format!("{}.{}", safe_title, extension);
        let filepath = notes_dir.join(&filename);
        let now = Local::now();
        Self {
            id: safe_title.clone(),
            title: safe_title,
            filepath,
            content: String::new(),
            modified_at: now,
            created_at: now,
            tags: Vec::new(),
        }
    }
    pub fn save(&mut self) -> Result<(), std::io::Error> {
        fs::write(&self.filepath, &self.content)?;
        self.modified_at = Local::now();
        self.tags = Self::extract_tags(&self.content);
        Ok(())
    }
    pub fn extract_tags(content: &str) -> Vec<String> {
        let tag_regex = Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap();
        let mut tags: Vec<String> = tag_regex
            .captures_iter(content)
            .map(|cap| cap[1].to_string())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }
    pub fn formatted_date(&self) -> String {
        self.modified_at.format("%Y-%m-%d %H:%M").to_string()
    }
    pub fn formatted_created_date(&self) -> String {
        self.created_at.format("%Y-%m-%d %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_notes_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nv-gtk-note-test-{}-{}",
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
    fn rule3_id_is_filename_stem() {
        let dir = temp_notes_dir();
        let path = dir.join("my-note.md");
        fs::write(&path, "content").unwrap();
        let note = Note::from_file(&path).unwrap();
        assert_eq!(note.id, "my-note");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule4_title_is_stem_not_first_content_line() {
        let dir = temp_notes_dir();
        let path = dir.join("my-note.md");
        fs::write(&path, "\n# Hello\nbody text").unwrap();
        let note = Note::from_file(&path).unwrap();
        assert_eq!(note.title, "my-note");
        assert_eq!(note.title, note.id);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule5_content_raw_and_tags_sorted_deduped() {
        let dir = temp_notes_dir();
        let raw = "  see #beta and #alpha #beta\n\n#with-dash_1 #!";
        let path = dir.join("tags.md");
        fs::write(&path, raw).unwrap();
        let note = Note::from_file(&path).unwrap();
        assert_eq!(note.content, raw);
        assert_eq!(note.tags, vec!["alpha", "beta", "with-dash_1"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule5_extract_tags_charset() {
        assert_eq!(
            Note::extract_tags("#foo #Foo #foo-bar_1 #! #"),
            vec!["Foo", "foo", "foo-bar_1"]
        );
        assert!(Note::extract_tags("no tags here").is_empty());
    }

    #[test]
    fn rule6_dates_come_from_filesystem() {
        let dir = temp_notes_dir();
        let path = dir.join("dated.md");
        fs::write(&path, "x").unwrap();
        let mtime: DateTime<Local> =
            DateTime::from(fs::metadata(&path).unwrap().modified().unwrap());
        let note = Note::from_file(&path).unwrap();
        let skew = (note.modified_at - mtime).num_seconds().abs();
        assert!(
            skew <= 2,
            "modified_at {:?} should match file mtime {:?}",
            note.modified_at,
            mtime
        );
        match fs::metadata(&path).unwrap().created() {
            Err(_) => assert_eq!(
                (note.created_at - note.modified_at).num_seconds().abs(),
                0,
                "without birthtime, created_at falls back to mtime"
            ),
            Ok(_) => assert!(note.created_at <= note.modified_at),
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rule8_save_refreshes_mtime_and_tags() {
        let dir = temp_notes_dir();
        assert_eq!(Note::new(&dir, "a/b", "md").id, "a-b");
        let mut note = Note::new(&dir, "fresh", "md");
        note.content = "hello #gamma #alpha #gamma".to_string();
        note.save().unwrap();
        assert_eq!(
            fs::read_to_string(&note.filepath).unwrap(),
            "hello #gamma #alpha #gamma"
        );
        assert_eq!(note.tags, vec!["alpha", "gamma"]);
        assert_eq!(note.id, "fresh");

        fs::remove_dir_all(&dir).ok();
    }
}
