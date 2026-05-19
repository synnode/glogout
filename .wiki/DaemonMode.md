---
title: "DaemonMode"
tags: [architecture, daemon, ipc, layer-shell]
related: ["HotReloadScope", "StackDecision"]
updated: 2026-05-19
---

# DaemonMode

`glogout daemon` is a long-lived background service that keeps a webview warm; `glogout show` is a thin client that pokes the daemon's Unix socket so the menu can appear sub-frame. Implemented in `src/daemon.rs` with an `App` abstraction in `main.rs` that one-shot and daemon both use.

## CLI surface

- `glogout` — one-shot (unchanged, back-compat).
- `glogout daemon` — server. Refuses to start if another daemon is reachable on the socket.
- `glogout show` — client. Connects, writes `show\n`, exits.
- `glogout init` — pre-existing; writes default config.

Subcommands were chosen over the spec's original `--daemon` / `--show` flags for consistency with `glogout init`. Mixing subcommands and flags reads worse than picking one.

## Socket

Path: `$XDG_RUNTIME_DIR/glogout.sock`. Refuses to start with no `XDG_RUNTIME_DIR` rather than guessing — no other path is private to the user across all setups.

Stale-vs-live detection: on startup, if the socket file exists we try to connect. Success → another daemon is alive, bail with an error. Failure → stale file from a crashed daemon, unlink and bind. A `SocketGuard` removes the file on clean Drop.

Protocol is line-based, one command per connection (`show\n`). No response framing — clients write and exit. Unknown commands are logged on the daemon side, dropped silently for the client.

## Surface lifecycle

The daemon builds menu + dimmer surfaces **without** calling `present()`. On `show` it presents them all; on action dispatch (or Escape) it calls `set_visible(false)` on each. WebView stays alive across hide/show — only the layer-shell mapping cycles.

This is the entire performance argument for daemon mode: WebKitGTK process spawn is the expensive bit, and we pay it once at daemon startup instead of per-invocation.

## Why Dispatcher no longer calls `main_loop.quit`

Pre-daemon, `Dispatcher::dispatch` ended every action with `main_loop.quit()`. That doesn't fit daemon semantics (action → hide, not quit). Refactored so dispatch is side-effects only; the IPC handler decides what to do after. One-shot quits the loop; daemon hides.

## Escape handling

`window::build_menu` installs a key controller that consults an `Rc<RefCell<Option<Box<dyn Fn()>>>>` — the late-bound `EscapeHook`. One-shot fills it with `main_loop.quit`; daemon fills it with `app.hide`. Avoids re-registering GTK key controllers when switching modes and lets `close_on_escape = false` simply leave the hook empty.

## systemd

`contrib/glogout.service`. `PartOf=graphical-session.target` so it stops with the session; `Restart=on-failure` so a webview crash doesn't leave the user without their menu.

## Related
- [[HotReloadScope]] — why hot reload runs in both modes
- [[StackDecision]] — why the stack is gtk4 + webkit6 + layer-shell
