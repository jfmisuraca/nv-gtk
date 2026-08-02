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
}
