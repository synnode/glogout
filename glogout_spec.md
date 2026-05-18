# glogout — Project Specification

> Working title: `glogout` (alternative welcome). A heavily customizable, HTML/CSS-themable logout menu for Wayland compositors that supports `wlr-layer-shell`. Built as an alternative to `wlogout`, designed to bypass GTK theme inheritance issues by rendering UI entirely in a webview.

## Motivation

`wlogout` (and its forks `wleave` / `waylogout`) all use GTK widgets for the menu buttons, which inherit the system GTK theme. This makes consistent visual theming impossible when your GTK theme conflicts with your desired logout menu appearance.

By rendering the entire UI inside a webview, GTK theming becomes irrelevant — the only GTK surface is the window itself, which is invisible and only used as a vehicle for `wlr-layer-shell`.

## Goals

- **Zero GTK theme bleed.** The UI is HTML/CSS/JS; nothing inherits from the GTK theme.
- **Heavily customizable.** Users provide their own HTML template + CSS file + a config defining buttons and the commands they trigger.
- **Acceptable startup latency.** Target < 300ms cold start on modern hardware; daemon mode as a fallback for instant invocation.
- **Wayland-native.** Uses `wlr-layer-shell` for proper overlay behavior (anchored fullscreen, keyboard exclusivity, no compositor decorations).
- **Lightweight.** No Tauri runtime overhead; use `wry` + `tao` directly.

## Non-goals (for v1)

- X11 support. Wayland only.
- GNOME support. GNOME does not implement `wlr-layer-shell`.
- Full session manager features (user switching, lockscreen integration). Logout/reboot/shutdown/lock/suspend only.
- Bundled themes. Ship one reference theme; the community can do the rest.

## Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Language | Rust | Performance, single static binary, fits the rest of my tooling |
| Window + event loop | `gtk4` + glib `MainLoop` | Native GTK4 window; no tao abstraction needed since we're Linux-only |
| Webview | `webkit6` | Direct bindings to WebKitGTK 6.0 (the GTK4-based port). No wry. |
| Layer shell | `gtk4-layer-shell` | Promotes the GtkWindow to a `wlr-layer-shell` surface |
| Config | TOML | Familiar, comments, matches the rest of my ecosystem |

**Explicitly rejected:**
- **Tauri** — adds runtime, plugin system, command framework, and updater that we don't need. Adds ~100ms startup with no benefit for a single-window popup.
- **Iced / Slint** — native, but no HTML/CSS = no theming story.
- **Smithay + webkit6** — pure Wayland client without GTK, but layer-shell handshake, input, and focus management become our problem. Overkill for v1.
- **`tao` + `wry`** (original plan) — would have been nice for the abstraction, but wry still depends on `webkit2gtk` (GTK3). GTK3 + Hyprland's `wp_linux_drm_syncobj_v1` (explicit sync) crashes with a "Missing acquire timeline" Wayland protocol error. The bug is in GTK3 itself, fixed in GTK4.14+ but not backported. Wry has no committed ETA for GTK4 migration. See `.wiki/StackDecision.md` for the full debugging trail.

## Architecture

### Process model

Two modes, selectable via CLI flag:

**1. One-shot (default)**
Cold-start on every invocation. Simpler, slower (~150-400ms to first paint). Good enough for occasional use.

**2. Daemon mode (`--daemon` / `--show`)**
A long-running user service holds an initialized but hidden webview. A lightweight client process sends `show` over a Unix socket. Subsequent invocations are sub-frame because the webview is already warm.

- Daemon socket: `$XDG_RUNTIME_DIR/glogout.sock`
- Idle RSS budget: target < 150MB
- systemd user unit shipped in `contrib/`

For v1, ship one-shot first. Daemon mode can land in v1.1 once the basics are stable.

### Window setup

```
gtk4::Window::new()
  └─ gtk4-layer-shell hijacks it before present()
      ├─ Layer::Overlay
      ├─ anchored to all four edges (fullscreen)
      ├─ KeyboardMode::Exclusive
      ├─ namespace "glogout" (for compositor rules)
      └─ exclusive_zone(-1) so we cover panels (waybar et al.)
  └─ webkit6::WebView is set as the window child
      ├─ transparent background (RGBA 0,0,0,0)
      ├─ loads HTML from config dir
      └─ UserContentManager script handler routes button presses to Rust
  └─ EventControllerKey on the window catches Esc as a native fallback
  └─ glib::MainLoop drives events; quit on action complete
```

### IPC: webview → Rust

The webview calls `window.webkit.messageHandlers.ipc.postMessage(action_id)`. A `UserContentManager` registered with handler name `"ipc"` receives the message via the `script-message-received::ipc` signal. The Rust handler looks up `action_id` in the loaded config, executes the associated command, and exits (or hides, in daemon mode).

Built-in actions for convenience (resolved if no command is configured):
- `logout` → `loginctl terminate-user $USER`
- `reboot` → `systemctl reboot`
- `shutdown` → `systemctl poweroff`
- `suspend` → `systemctl suspend`
- `hibernate` → `systemctl hibernate`
- `lock` → `loginctl lock-session`
- `cancel` → exit immediately

Users can override any of these or define arbitrary new actions with shell commands.

## Configuration

### Directory layout

```
~/.config/glogout/
├── config.toml      # button definitions, behavior
├── style.css        # all visual theming
└── template.html    # optional; default is generated
```

Fallback search order: `$XDG_CONFIG_HOME/glogout/` → `~/.config/glogout/` → `/etc/glogout/` → built-in defaults.

### `config.toml`

```toml
[settings]
# Close on click outside any button
close_on_focus_loss = true
# Close on Escape key
close_on_escape = true
# Optional: only show on a specific output (None = current focused output)
output = "DP-1"

[[buttons]]
id = "lock"
label = "Lock"
icon = "lock.svg"        # relative to config dir
keybind = "l"
action = "lock"          # built-in

[[buttons]]
id = "logout"
label = "Log out"
icon = "logout.svg"
keybind = "e"
action = "logout"

[[buttons]]
id = "suspend"
label = "Suspend"
icon = "suspend.svg"
keybind = "s"
action = "suspend"

[[buttons]]
id = "hibernate"
label = "Hibernate"
icon = "hibernate.svg"
keybind = "h"
command = "systemctl hibernate"   # arbitrary command instead of built-in

[[buttons]]
id = "reboot"
label = "Reboot"
icon = "reboot.svg"
keybind = "r"
action = "reboot"

[[buttons]]
id = "shutdown"
label = "Shutdown"
icon = "shutdown.svg"
keybind = "p"
action = "shutdown"
```

### Template variables

If the user provides a `template.html`, it is rendered with a tiny templating step before being passed to the webview. Available variables:

- `{{buttons}}` — expands to the button list, each rendered using an inner `<template id="button">` if present, otherwise a default `<button data-action="...">` markup
- `{{username}}`, `{{hostname}}` — convenience for greeting headers
- `{{stylesheet}}` — path to `style.css`

Keep this minimal. We are not a templating engine.

### Default theme

Ship one reference theme that demonstrates the capabilities:
- Centered grid of buttons
- Backdrop blur via `backdrop-filter`
- Hover/focus states
- Subtle fade-in animation on entry (also masks startup latency)
- Respects `prefers-reduced-motion`

## Open technical questions

These need verification during the prototyping phase:

1. ~~**Transparency on WebKitGTK + Wayland.**~~ **Resolved.** `webview.set_background_color(RGBA(0,0,0,0))` + CSS `background: transparent` works on Hyprland. `backdrop-filter: blur(...)` also works for the menu chrome.

2. ~~**Layer-shell init timing.**~~ **Resolved by switching to gtk4-layer-shell with direct `gtk4::Window`.** Call `init_layer_shell()` + all setup before `window.present()`. No tao to fight with.

3. **Compositor compatibility.** Target list for v1 testing: KWin (Plasma 6), Hyprland, Sway. Hyprland confirmed working with explicit sync enabled. KWin and Sway still need a smoke test. Document failure mode on GNOME (it should fail to launch with a clear error rather than silently misbehave).

4. **Keybind routing.** With `KeyboardMode::Exclusive`, all keyboard input goes to our surface. JS handles `keydown` to match against configured `keybind` values, fires the matching button's action. Single-key only for v1; modifier combos can wait. Native `EventControllerKey` on the window catches Esc as a fallback.

5. **Multi-monitor behavior.** With layer-shell, we can choose which output to anchor to. Default: focused output (requires querying compositor). Fallback: primary output. Config can pin to a specific output by name.

## Implementation phases

### Phase 1: Proof of concept (target: one evening)

- [x] Bare Rust binary that opens a `gtk4` window
- [x] Promote to layer-shell overlay, fullscreen, keyboard-exclusive
- [x] Mount `webkit6` webview with a hardcoded inline HTML string
- [x] Three hardcoded buttons (logout / reboot / cancel) that fire the right commands
- [x] Escape key exits

Success criterion: pressing the keybind I'll wire up in KWin shows the menu, clicking logout actually logs me out.

### Phase 2: Configurability (target: a few more sessions)

- [x] Load `config.toml` from XDG paths
- [x] Load `style.css` and pass its path into the template
- [x] Optional `template.html` with `{{buttons}}` expansion
- [x] Keybind handling driven by config
- [x] Built-in action resolution + arbitrary command support

### Phase 3: Polish

- [ ] Hot reload on config file change (`notify` crate watcher)
- [x] Fade-in animation, `prefers-reduced-motion` respect
- [x] Multi-monitor output selection — menu surface on the chosen output (`settings.output` or first listed), dimmer surfaces on every other output for a modal "all-screens-dim" feel. wlogout-style.
- [x] Reference theme that's actually nice
- [x] Error messages that don't suck (clear failure when compositor lacks layer-shell)

### Phase 4: Daemon mode (deferred)

- [ ] Unix socket server in daemon mode
- [ ] `--show` client subcommand
- [ ] systemd user unit in `contrib/`
- [ ] Document tradeoffs in README

## Risks & unknowns

- **WebKitGTK process spawn cost.** If cold-start latency turns out to be worse than the ~300ms estimate, daemon mode moves up in priority.
- **`webkit6` crate maturity.** The Rust bindings are younger than `webkit2gtk`. So far the API surface we need (UserContentManager + script-message-received + set_background_color + load_html + WebView::builder) is present and works, but uncommon features may be missing or wrapped imperfectly.
- **Compositors other than Hyprland.** Validated on Hyprland with explicit sync. KWin Plasma 6 and Sway still need smoke tests before claiming v1 support.

## Cargo dependencies

```toml
[dependencies]
gtk4 = "0.11"
gtk4-layer-shell = "0.8"
webkit6 = "0.6"
serde = { version = "1", features = ["derive"] }    # phase 2
toml = "0.8"                                         # phase 2
notify = "6"                                         # phase 3
clap = { version = "4", features = ["derive"] }      # phase 2
anyhow = "1"
```

Native libraries required at build & runtime: `gtk4`, `webkitgtk-6.0`, `gtk4-layer-shell-0`.

Versions are starting points; resolve against what's current at implementation time. The crate versions listed are what spike-2 validated on 2026-05-18.

## Reference: minimal skeleton

See `src/main.rs` for the working spike. The shape:

```rust
use gtk4::glib::{self, MainLoop};
use gtk4::prelude::*;
use gtk4::{EventControllerKey, Window, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

fn main() -> anyhow::Result<()> {
    gtk4::init()?;
    let main_loop = MainLoop::new(None, false);

    let window = Window::new();
    window.set_decorated(false);

    // CRITICAL: must happen before window.present().
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
        window.set_anchor(edge, true);
    }
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_namespace(Some("glogout"));
    window.set_exclusive_zone(-1);  // cover panels (waybar etc.)

    let manager = UserContentManager::new();
    manager.register_script_message_handler("ipc", None);
    {
        let main_loop = main_loop.clone();
        manager.connect_script_message_received(Some("ipc"), move |_, value| {
            handle_action(value.to_str().as_str(), &main_loop);
        });
    }

    let webview = WebView::builder().user_content_manager(&manager).build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    webview.load_html(include_str!("../ui/index.html"), None);
    window.set_child(Some(&webview));

    // Esc fallback in case the JS handler is unreachable.
    let key_controller = EventControllerKey::new();
    {
        let main_loop = main_loop.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                main_loop.quit();
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);

    window.present();
    main_loop.run();
    Ok(())
}

fn handle_action(action_id: &str, main_loop: &MainLoop) {
    // Look up action_id in loaded config, run command, quit main loop.
    todo!()
}
```

## Naming

Working title `glogout`. Open to alternatives. Constraints:
- Short
- Doesn't collide with existing tools (`wlogout`, `wleave`, `waylogout`, `swaylock`, etc.)
- Vibes with `glamfetch` since these are siblings in the same toolchain family

Candidates: `glogout`, `webxit`, `quitwl`, `bidout`, `outlay` (anchors layer, plays on layout).

## License

MIT or Apache-2.0 dual, matching the rest of the Rust ecosystem and my other Synnode releases.