# glogout

**A Wayland logout menu you theme with real HTML, CSS, and JavaScript — zero GTK theme inheritance, ever.**

`glogout` renders its entire UI inside a WebKitGTK webview mounted in a `wlr-layer-shell` overlay. The only GTK surface is the invisible window that carries the layer-shell handshake; everything you see is a web page you fully control. Backdrop blur, web fonts, inline SVG, keyframe animations, JS — the whole web platform, with nothing inherited from your system.

Every other Wayland logout menu — [`wlogout`](https://github.com/ArtsyMacaw/wlogout), [`wleave`](https://github.com/AMNatty/wleave), `waylogout`, [`powermenu`](https://github.com/shelepuginivan/powermenu), `nwg-bar` — is built from GTK (or Qt) widgets and styled with that toolkit's narrow CSS dialect, so it inherits and fights your system theme. `glogout` sidesteps the problem by not using widgets at all.

## Why

Themable logout menus on Linux are unreasonably hard. The existing tools render with GTK widgets that pick up the system GTK theme by default, and "style.css" there means GTK's CSS subset, not the real thing. If your system theme is at odds with what you want the menu to look like, you lose — there's no clean escape hatch.

`glogout` rendering the menu in a `webkit6` WebView inside a `gtk4-layer-shell` overlay *is* that escape hatch. The WebView is untouched by GTK theming, and you get a real browser engine: anything you can do on a web page, you can do to your logout menu.

## Features

- **Zero GTK theme bleed.** Your CSS is the only CSS.
- **Hot reload.** Edit `config.toml`, `style.css`, or `template.html` and the menu re-renders in place. Debounced so editor save patterns don't spam reloads.
- **Daemon mode.** A long-running `glogout daemon` keeps the webview warm; `glogout show` reveals the menu sub-frame. One-shot still works if you don't care about latency.
- **Multi-monitor aware.** Every output dims and the menu floats on top wherever the compositor places it, so the modal feel covers the whole session no matter which screen it opens on.
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
make install        # builds, installs to ~/.cargo/bin + the systemd user unit
```

`make install` warns if another `glogout` earlier in `PATH` would shadow the
one it just installed. For a system-wide install: `make build && sudo make
install PREFIX=/usr/local`. Run `make help` for all targets.

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
# output = "DP-1"              # pin to a specific monitor (needs daemon restart)
# dimmer_color = "#121216"     # dimmer overlay color (#RRGGBB)
# dimmer_opacity = 0.6         # 0.0 = see-through (shows desktop), 1.0 = opaque

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

### Button anatomy

`{{buttons}}` expands to one `<button>` per `[[buttons]]` entry, wrapped in a
`.menu` container inside a `.layout` column. Each button is generated like this
— the `.kbd` and `.icon` spans appear **only** when you set `keybind`/`icon`,
and the **label span has no class**:

```html
<div class="layout">
  <div class="menu">
    <button data-action="logout" data-keybind="l" autofocus>
      <span class="kbd">l</span>      <!-- only if keybind is set -->
      <span class="icon">⏻</span>     <!-- only if icon is set -->
      <span>Log out</span>            <!-- label: NO class -->
    </button>
    <!-- ...one <button> per config entry, in order... -->
  </div>
  <div class="hint">Press a key, click a button, or hit Esc to cancel</div>
</div>
```

`data-action` is the button's `id`; the first button gets `autofocus`. Selector
reference for your `style.css`:

| Selector | Targets |
|---|---|
| `.layout` | outer column wrapping the buttons and the hint |
| `.menu` | the button container (default: a horizontal grid) |
| `button` | every button |
| `button[data-action="logout"]` | one specific button, by its config `id` |
| `button:focus-visible` | the keyboard-focused button |
| `.icon` | the icon span (present only if `icon` is set) |
| `.kbd` | the keybind badge (present only if `keybind` is set) |
| `button > span:last-child` | the label text (it has no class of its own) |
| `.hint` | the footer hint line |

So to restyle just the cancel button and the labels:

```css
button[data-action="cancel"] { opacity: 0.7; }
button > span:last-child { font-weight: 600; letter-spacing: 0.02em; }
```

### Frosted-glass blur

CSS `backdrop-filter: blur()` **cannot** blur your desktop. A web engine only
samples its own document, so with a transparent page there is nothing behind
`body` to blur — it's a no-op for desktop show-through. Real frosted glass is
**compositor blur**. Every glogout surface sets its layer-shell namespace to
`glogout`, so you can target it with a Hyprland layer rule (needs the global
`decoration { blur { enabled = true } }`, which is on by default). Pair it with a
low `dimmer_opacity`.

The rule syntax is Hyprland-version-dependent:

```ini
# Hyprland < 0.53 (classic)
layerrule = blur, glogout
layerrule = ignorezero, glogout

# Hyprland 0.53–0.55 (structured rule blocks).
# The classic comma form errors here; ignorezero became ignore_alpha (0.0–1.0).
layerrule {
    name = glogout-blur
    match:namespace = ^glogout$
    blur = true
    ignore_alpha = 0.0
}
```

On 0.55+ you can also use the Lua config form:

```lua
hl.layer_rule({ match = { namespace = "^glogout$" }, blur = true, ignore_alpha = 0.0 })
```

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

The watcher is always on (in both one-shot and daemon mode) and triggers on changes to `config.toml`, `style.css`, or `template.html` in the resolved config directory. Reload is in-place: the webview's HTML is re-rendered, the button dispatcher is rebuilt, and `[settings]` are re-applied live — the dimmer (`dimmer_color`/`dimmer_opacity`) and `close_on_escape` all take effect on save, no process restart. The one exception is `output`, which rebuilds the layer surfaces and so needs a restart. Parse errors are logged and the previous state is kept, so a half-edited file never bricks an open overlay.

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

## Known limitations

- **`output` placement on Hyprland is timing-sensitive.** The menu grabs the keyboard exclusively, and Hyprland places keyboard-grabbing layer surfaces on the *focused* output, racing the `set_monitor` request. The passive dimmers honor `output` reliably; the menu itself does not in every build. In practice a `--release` binary applies `set_monitor` fast enough that the menu lands on the requested output (consistently observed across triggers from different screens), but a debug build often opens it on the cursor's monitor instead. Treat pinned `output` as reliable on release builds, not guaranteed — verify on your own setup. This is also why every monitor is dimmed rather than just the menu's.
- **`output` changes need a daemon restart.** Re-anchoring rebuilds the layer surfaces, which the in-place reload path doesn't do.

## License

MIT — see [LICENSE](LICENSE). Do whatever you want with it; just keep the copyright notice.

## Status

`v0.3.0`. Phases 1–4 complete — config + theming, hot reload, daemon mode, multi-monitor. Configurable dimmer and fully hot-reloadable `[settings]` (except `output`). Daily-driven on Hyprland; the spec checklist is the authoritative roadmap. Bug reports and theming PRs welcome.
