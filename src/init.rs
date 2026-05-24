use anyhow::{Context, Result};
use std::path::PathBuf;
use std::{env, fs};

use crate::ui::{DEFAULT_CSS, DEFAULT_TEMPLATE};

/// Hand-written so it carries comments and a sensible structure that the
/// auto-generated serde output would not produce.
const DEFAULT_CONFIG_TOML: &str = r##"# glogout config — see https://github.com/synnode/glogout

[settings]
# Close the menu when Escape is pressed.
close_on_escape = true

# Close the menu when it loses focus. Currently a no-op because the menu
# grabs the keyboard exclusively, but reserved for future work.
close_on_focus_loss = true

# Pin the menu to a specific output by connector name (e.g. "DP-1").
# Leave commented out to use the focused output.
# output = "DP-1"

# Dimmer overlay drawn behind the menu on every monitor. dimmer_color is
# #RRGGBB; dimmer_opacity is 0.0 (fully see-through, shows your desktop) to
# 1.0 (opaque). Lower the opacity to let the desktop shine through.
# dimmer_color = "#121216"
# dimmer_opacity = 0.6

[[buttons]]
id = "logout"
label = "Log out"
icon = "⏻"
keybind = "l"
action = "logout"   # built-in: loginctl terminate-user $USER

[[buttons]]
id = "reboot"
label = "Reboot"
icon = "⟳"
keybind = "r"
action = "reboot"   # built-in: systemctl reboot

[[buttons]]
id = "cancel"
label = "Cancel"
icon = "✕"
action = "cancel"   # built-in: just close the menu

# Other available built-ins: shutdown, suspend, hibernate, lock.
# For anything else, use `command = "..."` instead of `action = "..."`.
# Commands run via `sh -c` so quoting, pipes, and env vars work as expected.
"##;

pub fn run(force: bool) -> Result<()> {
    let dir = target_dir().context("could not determine config directory")?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let files = [
        ("config.toml", DEFAULT_CONFIG_TOML),
        ("style.css", DEFAULT_CSS),
        ("template.html", DEFAULT_TEMPLATE),
    ];

    for (name, content) in files {
        let path = dir.join(name);
        if path.exists() && !force {
            println!("skip   {} (already exists; pass --force to overwrite)", path.display());
            continue;
        }
        fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote  {}", path.display());
    }
    Ok(())
}

fn target_dir() -> Option<PathBuf> {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("glogout"));
        }
    }
    env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/glogout"))
}
