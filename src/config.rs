use std::path::{Path, PathBuf};

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
    pub default_sort: String,
    pub show_detail_panel: bool,
    pub show_kernel_threads: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            refresh_rate_ms: 2000,
            default_color_mode: "memory".to_string(),
            default_sort: "memory".to_string(),
            show_detail_panel: false,
            show_kernel_threads: false,
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
}

impl Default for TreemapConfig {
    fn default() -> Self {
        TreemapConfig {
            min_rect_width: 6,
            min_rect_height: 2,
            group_threshold: 0.01,
            max_visible_procs: 25,
            border_style: "thin".to_string(),
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
            theme: "dark".to_string(),
            heat_low: "#2d5a27".to_string(),
            heat_mid: "#b5890a".to_string(),
            heat_high: "#a12e2e".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct KeybindsConfig {
    pub quit: String,
    pub kill: String,
    pub force_kill: String,
    pub filter: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub cycle_color: String,
    pub toggle_detail: String,
    pub help: String,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        KeybindsConfig {
            quit: "q".to_string(),
            kill: "k".to_string(),
            force_kill: "K".to_string(),
            filter: "/".to_string(),
            zoom_in: "Enter".to_string(),
            zoom_out: "Escape".to_string(),
            cycle_color: "c".to_string(),
            toggle_detail: "d".to_string(),
            help: "?".to_string(),
        }
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
        assert_eq!(config.general.default_color_mode, "memory");
        assert!(!config.general.show_detail_panel);
        assert_eq!(config.treemap.min_rect_width, 6);
        assert_eq!(config.colors.theme, "dark");
        assert_eq!(config.keybinds.quit, "q");
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
        assert_eq!(config.general.default_color_mode, "memory");
        assert_eq!(config.treemap.min_rect_width, 6);
    }

    #[test]
    fn parse_full_toml() {
        let toml_str = r#"
[general]
refresh_rate_ms = 1000
default_color_mode = "cpu"
show_detail_panel = true

[treemap]
min_rect_width = 6
group_threshold = 0.05

[colors]
theme = "light"

[keybinds]
quit = "x"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.refresh_rate_ms, 1000);
        assert_eq!(config.general.default_color_mode, "cpu");
        assert!(config.general.show_detail_panel);
        assert_eq!(config.treemap.min_rect_width, 6);
        assert!((config.treemap.group_threshold - 0.05).abs() < f64::EPSILON);
        assert_eq!(config.colors.theme, "light");
        assert_eq!(config.keybinds.quit, "x");
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
}
