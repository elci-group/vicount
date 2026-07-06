use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// User-facing configuration loaded from `~/.config/vicount/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Vicount user configuration.
pub struct Config {
    /// URL of the ViCo Desktop gateway.
    ///
    /// The `VICO_DESKTOP_URL` environment variable takes precedence when set.
    pub vico_url: Option<String>,

    /// Theme name. Currently supported: `dark` (default) and `light`.
    pub theme: Option<String>,

    /// Maximum number of recent inputs to keep in the persistent history file.
    pub max_history: Option<usize>,
}

fn config_dir() -> Option<PathBuf> {
    let base = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        return None;
    };
    let dir = base.join("vicount");
    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("cannot create config dir {}: {e}", dir.display());
        return None;
    }
    Some(dir)
}

/// Path to the TOML configuration file.
pub fn config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Load the configuration file, falling back to defaults when it is missing or malformed.
/// Load config from the default XDG config path.
pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => {
            debug!("no config directory available; using defaults");
            return Config::default();
        }
    };

    if !path.exists() {
        debug!("no config file at {}; using defaults", path.display());
        return Config::default();
    }

    match fs::read_to_string(&path) {
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(cfg) => {
                debug!("loaded config from {}", path.display());
                cfg
            }
            Err(e) => {
                warn!(
                    "malformed config file {}: {e}; using defaults",
                    path.display()
                );
                Config::default()
            }
        },
        Err(e) => {
            warn!(
                "cannot read config file {}: {e}; using defaults",
                path.display()
            );
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn config_path_uses_xdg_config_home() {
        env::set_var("XDG_CONFIG_HOME", "/tmp/xdg-test");
        let path = config_path().unwrap();
        assert!(path.ends_with("vicount/config.toml"));
        assert!(path.starts_with("/tmp/xdg-test"));
    }

    fn temp_config_dir() -> PathBuf {
        env::temp_dir().join(format!(
            "vicount-cfg-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn load_missing_config_returns_defaults() {
        let tmp = temp_config_dir();
        env::set_var("XDG_CONFIG_HOME", &tmp);
        let _ = fs::remove_dir_all(&tmp);
        let cfg = load();
        assert!(cfg.vico_url.is_none());
        assert!(cfg.theme.is_none());
        assert!(cfg.max_history.is_none());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn load_valid_config_file() {
        let tmp = temp_config_dir();
        env::set_var("XDG_CONFIG_HOME", &tmp);
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp.join("vicount")).unwrap();
        fs::write(
            tmp.join("vicount/config.toml"),
            r#"vico_url = "http://127.0.0.1:9876"
theme = "light"
max_history = 200
"#,
        )
        .unwrap();
        let cfg = load();
        assert_eq!(cfg.vico_url, Some("http://127.0.0.1:9876".to_string()));
        assert_eq!(cfg.theme, Some("light".to_string()));
        assert_eq!(cfg.max_history, Some(200));
        let _ = fs::remove_dir_all(&tmp);
    }
}

