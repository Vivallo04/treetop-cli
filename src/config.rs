use std::path::{Path, PathBuf};

use crossterm::event::KeyCode;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub treemap: TreemapConfig,
    pub colors: ColorsConfig,
    pub keybinds: KeybindsConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub refresh_rate_ms: u64,
    pub default_color_mode: String,
    pub show_detail_panel: bool,
    pub sparkline_length: usize,
    pub color_support: String,
    pub default_sort: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            refresh_rate_ms: 2000,
            default_color_mode: "name".to_string(),
            show_detail_panel: false,
            sparkline_length: 60,
            color_support: "auto".to_string(),
            default_sort: "memory".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct TreemapConfig {
    pub min_rect_width: u16,
    pub min_rect_height: u16,
    pub group_threshold: f64,
    pub max_visible_procs: usize,
    pub border_style: String,
    pub animation_frames: u8,
}

impl Default for TreemapConfig {
    fn default() -> Self {
        TreemapConfig {
            min_rect_width: 6,
            min_rect_height: 2,
            group_threshold: 0.01,
            max_visible_procs: 25,
            border_style: "thin".to_string(),
            animation_frames: 5,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ColorsConfig {
    pub theme: String,
    pub heat_low: String,
    pub heat_mid: String,
    pub heat_high: String,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        ColorsConfig {
            theme: "vivid".to_string(),
            heat_low: "#475569".to_string(),
            heat_mid: "#f97316".to_string(),
            heat_high: "#ec4899".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct KeybindsConfig {
    pub quit: String,
    pub filter: String,
    pub kill: String,
    pub force_kill: String,
    pub cycle_color: String,
    pub cycle_theme: String,
    pub toggle_detail: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub help: String,
    pub cycle_sort: String,
    pub refresh: String,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        KeybindsConfig {
            quit: "q".to_string(),
            filter: "/".to_string(),
            kill: "k".to_string(),
            force_kill: "K".to_string(),
            cycle_color: "c".to_string(),
            cycle_theme: "t".to_string(),
            toggle_detail: "d".to_string(),
            zoom_in: "Enter".to_string(),
            zoom_out: "Esc".to_string(),
            help: "?".to_string(),
            cycle_sort: "s".to_string(),
            refresh: "r".to_string(),
        }
    }
}

/// Parses a key string from config into a `KeyCode`.
///
/// Supports:
/// - Single characters: `"q"`, `"/"`, `"?"`, `"K"`
/// - Named keys: `"Enter"`, `"Esc"`, `"Tab"`, `"Backspace"`, `"Space"`
#[allow(dead_code)] // Used in Step 5 (keybind integration)
pub fn parse_key(s: &str) -> Option<KeyCode> {
    // Check named keys case-insensitively first
    match s.to_lowercase().as_str() {
        "enter" | "return" => return Some(KeyCode::Enter),
        "esc" | "escape" => return Some(KeyCode::Esc),
        "tab" => return Some(KeyCode::Tab),
        "backspace" => return Some(KeyCode::Backspace),
        "space" => return Some(KeyCode::Char(' ')),
        "delete" | "del" => return Some(KeyCode::Delete),
        _ => {}
    }
    // For single characters, preserve case (K ≠ k)
    let chars: Vec<char> = s.chars().collect();
    if chars.len() == 1 {
        Some(KeyCode::Char(chars[0]))
    } else {
        None
    }
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("treetop").join("config.toml"))
}

pub fn load_config() -> Config {
    match config_path() {
        Some(path) if path.exists() => load_config_from_path(&path),
        _ => Config::default(),
    }
}

pub fn load_config_from_path(path: &Path) -> Config {
    match std::fs::read_to_string(path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = Config::default();
        assert_eq!(config.general.refresh_rate_ms, 2000);
        assert_eq!(config.general.default_color_mode, "name");
        assert!(!config.general.show_detail_panel);
        assert_eq!(config.treemap.min_rect_width, 6);
        assert_eq!(config.colors.theme, "vivid");
        assert_eq!(config.general.color_support, "auto");
    }

    #[test]
    fn parse_partial_toml() {
        let toml_str = r#"
[general]
refresh_rate_ms = 500
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.refresh_rate_ms, 500);
        // Other fields should be defaults
        assert_eq!(config.general.default_color_mode, "name");
        assert_eq!(config.treemap.min_rect_width, 6);
    }

    #[test]
    fn parse_full_toml() {
        let toml_str = r#"
[general]
refresh_rate_ms = 1000
default_color_mode = "cpu"
show_detail_panel = true
color_support = "truecolor"

[treemap]
min_rect_width = 6
group_threshold = 0.05

[colors]
theme = "light"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.refresh_rate_ms, 1000);
        assert_eq!(config.general.default_color_mode, "cpu");
        assert!(config.general.show_detail_panel);
        assert_eq!(config.general.color_support, "truecolor");
        assert_eq!(config.treemap.min_rect_width, 6);
        assert!((config.treemap.group_threshold - 0.05).abs() < f64::EPSILON);
        assert_eq!(config.colors.theme, "light");
    }

    #[test]
    fn missing_file_returns_default() {
        let config = load_config_from_path(Path::new("/nonexistent/path/config.toml"));
        assert_eq!(config.general.refresh_rate_ms, 2000);
    }

    #[test]
    fn invalid_toml_returns_default() {
        let temp = std::env::temp_dir().join("treetop_test_invalid.toml");
        std::fs::write(&temp, "this is not valid toml {{{{").unwrap();
        let config = load_config_from_path(&temp);
        assert_eq!(config.general.refresh_rate_ms, 2000);
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn parse_key_valid_chars_and_names() {
        assert_eq!(parse_key("q"), Some(KeyCode::Char('q')));
        assert_eq!(parse_key("K"), Some(KeyCode::Char('K'))); // case preserved for single chars
        assert_eq!(parse_key("/"), Some(KeyCode::Char('/')));
        assert_eq!(parse_key("?"), Some(KeyCode::Char('?')));
        assert_eq!(parse_key("Enter"), Some(KeyCode::Enter));
        assert_eq!(parse_key("enter"), Some(KeyCode::Enter));
        assert_eq!(parse_key("Esc"), Some(KeyCode::Esc));
        assert_eq!(parse_key("escape"), Some(KeyCode::Esc));
        assert_eq!(parse_key("Tab"), Some(KeyCode::Tab));
        assert_eq!(parse_key("Backspace"), Some(KeyCode::Backspace));
        assert_eq!(parse_key("Space"), Some(KeyCode::Char(' ')));
        assert_eq!(parse_key("Delete"), Some(KeyCode::Delete));
    }

    #[test]
    fn parse_key_invalid_returns_none() {
        assert_eq!(parse_key(""), None);
        assert_eq!(parse_key("CtrlA"), None);
        assert_eq!(parse_key("nope"), None);
        assert_eq!(parse_key("ab"), None);
    }

    #[test]
    fn keybinds_partial_toml_uses_defaults() {
        let toml_str = r#"
[keybinds]
quit = "x"
help = "h"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.keybinds.quit, "x");
        assert_eq!(config.keybinds.help, "h");
        // Others should be defaults
        assert_eq!(config.keybinds.filter, "/");
        assert_eq!(config.keybinds.kill, "k");
        assert_eq!(config.keybinds.cycle_sort, "s");
        assert_eq!(config.keybinds.zoom_in, "Enter");
    }

    #[test]
    fn default_sort_config() {
        let config = Config::default();
        assert_eq!(config.general.default_sort, "memory");

        let toml_str = r#"
[general]
default_sort = "cpu"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.default_sort, "cpu");
    }
}
