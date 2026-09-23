use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub notes_dir: PathBuf,
    pub default_extension: String,
    pub auto_save_ms: u64,
    /// Opaque persisted theme identifier: the stable lowercase id produced by
    /// the desktop `palettes::ThemeId` (`catppuccin`, `dracula`, `flexoki`,
    /// `wallpaper`). `nv-core` deliberately does not validate it — the UI
    /// layer parses it and falls back to Catppuccin on an unknown value.
    /// `#[serde(default)]` keeps an existing `~/.config/nv-gtk/config.json`
    /// (written before this field existed) loading unchanged.
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    String::from("catppuccin")
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let notes_dir = home.join("Notes");
        Self {
            notes_dir,
            default_extension: String::from("md"),
            auto_save_ms: 300,
            theme: default_theme(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nv-gtk");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                    // Ensure notes_dir exists
                    fs::create_dir_all(&cfg.notes_dir).ok();
                    return cfg;
                }
            }
        }

        let cfg = Self::default();
        cfg.save();
        cfg
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            fs::write(path, content).ok();
        }
        fs::create_dir_all(&self.notes_dir).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_catppuccin() {
        assert_eq!(Config::default().theme, "catppuccin");
    }

    #[test]
    fn round_trip_preserves_theme_and_other_fields() {
        let cfg = Config::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.theme, cfg.theme);
        assert_eq!(back.notes_dir, cfg.notes_dir);
        assert_eq!(back.default_extension, cfg.default_extension);
        assert_eq!(back.auto_save_ms, cfg.auto_save_ms);
    }

    #[test]
    fn legacy_file_without_theme_defaults_to_catppuccin() {
        let cfg: Config = serde_json::from_str(
            r#"{"notes_dir":"/tmp/notes","default_extension":"md","auto_save_ms":300}"#,
        )
        .unwrap();
        assert_eq!(cfg.theme, "catppuccin");
    }

    #[test]
    fn explicit_theme_value_preserved() {
        let cfg: Config = serde_json::from_str(
            r#"{"notes_dir":"/tmp/notes","default_extension":"md","auto_save_ms":300,"theme":"dracula"}"#,
        )
        .unwrap();
        assert_eq!(cfg.theme, "dracula");
    }

    #[test]
    fn unknown_theme_value_preserved_verbatim() {
        let cfg: Config = serde_json::from_str(
            r#"{"notes_dir":"/tmp/notes","default_extension":"md","auto_save_ms":300,"theme":"solarized"}"#,
        )
        .unwrap();
        assert_eq!(cfg.theme, "solarized");
    }
}
