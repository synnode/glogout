use serde::Deserialize;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    pub settings: Settings,
    pub buttons: Vec<Button>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub close_on_escape: bool,
    pub close_on_focus_loss: bool,
    pub output: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Button {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub keybind: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            buttons: default_buttons(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_on_escape: true,
            close_on_focus_loss: true,
            output: None,
        }
    }
}

/// Built-in three-button fallback used when no config file is found.
/// Matches phase 1 behavior so a fresh install does something sensible.
fn default_buttons() -> Vec<Button> {
    vec![
        Button {
            id: "logout".into(),
            label: "Log out".into(),
            icon: Some("⏻".into()),
            keybind: Some("l".into()),
            action: Some("logout".into()),
            command: None,
        },
        Button {
            id: "reboot".into(),
            label: "Reboot".into(),
            icon: Some("⟳".into()),
            keybind: Some("r".into()),
            action: Some("reboot".into()),
            command: None,
        },
        Button {
            id: "cancel".into(),
            label: "Cancel".into(),
            icon: Some("✕".into()),
            keybind: None,
            action: Some("cancel".into()),
            command: None,
        },
    ]
}

/// Resolved config plus the directory it was loaded from. The directory
/// is needed so the UI layer can find sibling files like `style.css` and
/// `template.html`. Returns `None` for the path when defaults are used.
pub struct Loaded {
    pub config: Config,
    pub dir: Option<PathBuf>,
}

pub fn load() -> Loaded {
    for dir in search_paths() {
        let path = dir.join("config.toml");
        if !path.exists() {
            continue;
        }
        match fs::read_to_string(&path).map(|s| toml::from_str::<Config>(&s)) {
            Ok(Ok(config)) => return Loaded { config, dir: Some(dir) },
            Ok(Err(e)) => eprintln!("glogout: {} parse error: {e}", path.display()),
            Err(e) => eprintln!("glogout: {} read error: {e}", path.display()),
        }
    }
    Loaded { config: Config::default(), dir: None }
}

/// Re-read and parse `config.toml` from a known directory. Used by hot
/// reload, which already knows which dir we resolved at startup and has
/// no reason to re-walk the search paths.
pub fn load_from(dir: &std::path::Path) -> Result<Config, String> {
    let path = dir.join("config.toml");
    let text = fs::read_to_string(&path).map_err(|e| format!("{} read error: {e}", path.display()))?;
    toml::from_str::<Config>(&text).map_err(|e| format!("{} parse error: {e}", path.display()))
}

fn search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            paths.push(PathBuf::from(xdg).join("glogout"));
        }
    }
    if let Ok(home) = env::var("HOME") {
        paths.push(PathBuf::from(home).join(".config/glogout"));
    }
    paths.push(PathBuf::from("/etc/glogout"));
    paths
}
