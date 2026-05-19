# glogout

A heavily themable logout menu for `wlr-layer-shell` Wayland compositors. Renders its UI entirely in a webview so the system GTK theme never bleeds through — the only GTK surface is the invisible window that hosts the layer-shell handshake.

Built as a drop-in alternative to [`wlogout`](https://github.com/ArtsyMacaw/wlogout) / `wleave` / `waylogout`, all of which use GTK widgets and therefore inherit your GTK theme.

## Why

Themable logout menus on Linux are unreasonably hard. The existing tools render with GTK widgets, which pick up the system GTK theme by default. If your system theme is at odds with what you want the logout menu to look like, you lose. There is no good escape hatch.

`glogout` solves this by rendering the menu in a `webkit6` WebView mounted inside a `gtk4-layer-shell` overlay window. The WebView is unaffected by GTK theming. You bring HTML, CSS, and a button definition; the menu inherits zero styling from anything else on your system.

## Features

- **Zero GTK theme bleed.** Your CSS is the only CSS.
- **Hot reload.** Edit `config.toml`, `style.css`, or `template.html` and the menu re-renders in place. Debounced so editor save patterns don't spam reloads.
- **Daemon mode.** A long-running `glogout daemon` keeps the webview warm; `glogout show` reveals the menu sub-frame. One-shot still works if you don't care about latency.
- **Multi-monitor aware.** Menu on the chosen output, dimmer surfaces on every other monitor so the modal feel covers the whole session.
- **Sensible built-ins.** `logout`, `reboot`, `shutdown`, `suspend`, `hibernate`, `lock`, `cancel`. Everything else is an arbitrary shell command.
- **Single static binary.** Rust. No runtime, no plugin host, no electron.

## Requirements

- A Wayland compositor that implements `wlr-layer-shell`. Validated on **Hyprland**; **KWin (Plasma 6)** and **Sway** are expected to work. **GNOME does not implement layer-shell** and is not supported.
- WebKitGTK 6.0 (`webkitgtk-6.0` package on Arch; `libwebkitgtk-6.0` on Debian/Ubuntu).
- GTK 4.14 or newer.

## Install

```bash
git clone https://github.com/synnode/glogout
cd glogout
cargo build --release
install -Dm755 target/release/glogout ~/.cargo/bin/glogout
```

Or — once published — `cargo install glogout`.

Then write the default config:

```bash
glogout init
```

This creates `~/.config/glogout/{config.toml,style.css,template.html}`.

## Configuration

`~/.config/glogout/config.toml`:

```toml
[settings]
close_on_escape = true
close_on_focus_loss = true     # reserved; currently a no-op
# output = "DP-1"              # pin to a specific monitor (connector name)

[[buttons]]
id = "logout"
label = "Log out"
icon = "⏻"
keybind = "l"
action = "logout"              # built-in

[[buttons]]
id = "screenshot"
label = "Screenshot"
icon = "📸"
keybind = "s"
command = "grim ~/shot.png"    # arbitrary shell command
```

**Built-in actions** map to standard system commands:

| `action`     | runs                                |
|--------------|-------------------------------------|
| `logout`     | `loginctl terminate-user $USER`     |
| `reboot`     | `systemctl reboot`                  |
| `shutdown`   | `systemctl poweroff`                |
| `suspend`    | `systemctl suspend`                 |
| `hibernate`  | `systemctl hibernate`               |
| `lock`       | `loginctl lock-session`             |
| `cancel`     | (closes the menu)                   |

Anything else? Use `command = "..."` instead of `action = "..."`. Commands run via `sh -c`, so quoting, pipes, env vars, and `&` background-spawning all work.

## Theming

Edit `~/.config/glogout/style.css` directly. The defaults expose CSS variables for the common knobs:

```css
:root {
  --bg: rgba(18, 18, 22, 0.6);
  --fg: #f2f2f2;
  --accent: #7aa7ff;
  --button-bg: rgba(255, 255, 255, 0.05);
  --button-bg-hover: rgba(255, 255, 255, 0.1);
}
```

For full layout control, edit `template.html`. Available placeholders:

- `{{stylesheet}}` — your CSS, injected as `<style>...</style>`
- `{{buttons}}` — the rendered button list, in config order
- `{{script}}` — the click/keybind dispatcher (don't omit this)
- `{{username}}` / `{{hostname}}` — current `$USER` and `/etc/hostname`

## One-shot vs daemon

Two modes, two tradeoffs.

### One-shot (default)

```bash
glogout
```

Cold-starts the webview every time. Simplest setup — bind a key in your compositor to `glogout` and you're done. Expect **150–400ms** to first paint depending on hardware, dominated by WebKitGTK process spawn. Fine for occasional use.

### Daemon

```bash
glogout daemon &     # long-running; usually in a systemd user unit
glogout show         # client; binds to your hotkey
```

The daemon builds the webview at startup and keeps it warm. `glogout show` opens a Unix socket connection at `$XDG_RUNTIME_DIR/glogout.sock` and the menu appears sub-frame. After dispatch (or Escape), the surfaces hide and the daemon goes back to standby — no restart needed.

**When to prefer daemon:**

- You invoke the menu often enough that 200ms+ feels annoying.
- You're scripting around it and want predictable latency.
- You're already running other warm-webview tools (e.g. eww) and don't mind one more.

**When one-shot is fine:**

- You log out once a day or less.
- You're tight on idle RSS budget (the daemon parks at ~50–150MB depending on WebKit version).
- You don't want a systemd unit in your dotfiles.

The daemon refuses to start if another instance is reachable on the socket, and cleans up the socket file on clean exit. A stale socket from a crashed daemon is auto-detected and unlinked at startup.

### systemd user unit

`contrib/glogout.service` ships a ready-to-go unit:

```bash
mkdir -p ~/.config/systemd/user
cp contrib/glogout.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now glogout.service
```

`PartOf=graphical-session.target` so it stops when your session ends; `Restart=on-failure` so a webview crash doesn't leave you without a menu.

## Hot reload

The watcher is always on (in both one-shot and daemon mode) and triggers on changes to `config.toml`, `style.css`, or `template.html` in the resolved config directory. Reload is in-place: the webview's HTML is re-rendered, the button dispatcher is rebuilt, no process restart. Parse errors are logged and the previous state is kept, so a half-edited file never bricks an open overlay.

In one-shot mode the practical value is limited (the overlay covers all monitors, so you can't reach an editor while it's open). The feature exists mostly as scaffolding for daemon mode, where it actually matters: edit your config while the daemon is hidden, then `glogout show` to see the result.

## CLI

```
glogout              one-shot menu
glogout daemon       run as a background service
glogout show         tell a running daemon to show
glogout toggle       show if hidden, hide if shown (one keybind, both ways)
glogout init         write default config files to ~/.config/glogout/
```

Bind `glogout toggle` rather than `glogout show` if you want a single key to both summon and dismiss the menu.

`glogout init --force` overwrites existing files.

## Architecture

For the full picture see [`glogout_spec.md`](glogout_spec.md) and the `.wiki/` pages. The short version:

```
gtk4::Window  ── promoted via gtk4-layer-shell to a wlr-layer-shell overlay
  └─ webkit6::WebView  (transparent background, loads HTML from config)
       └─ JS posts action_id via UserContentManager
            └─ Rust dispatcher resolves action_id → built-in or `sh -c`
```

The stack is Linux-only on purpose. No `tao`, no `wry`. Earlier prototypes used `wry`, which still pulls in `webkit2gtk` (GTK 3) and crashes on Hyprland's explicit-sync surfaces (GTK 3 bug, fixed in GTK 4, not backported). See `.wiki/StackDecision.md` for the full debugging trail.

## License

TBD.

## Status

Pre-release. Phase 1–4 complete; the spec checklist is the authoritative roadmap. Bug reports and theming PRs welcome.
