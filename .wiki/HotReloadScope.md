---
title: "HotReloadScope"
tags: [design-decision, hot-reload, daemon, roadmap]
related: ["StackDecision", "SurfaceTransparency", "HotReloadLoopBug"]
updated: 2026-05-24
---

# HotReloadScope

Hot reload (config + assets via `notify`) is implemented **before** daemon mode as deliberate preparation, not as the last Phase 3 polish item. In one-shot mode the feature is essentially decorative — the overlay covers all outputs, so the user cannot edit a file while glogout is open. In daemon mode the long-running process holds the parsed config in memory across invocations, and that is where a config watcher becomes load-bearing: without it the user must restart the user service to pick up a `config.toml` change.

## Principle: settings should be hot-reloadable

A `[settings]` change should take effect on save without restarting the daemon — restarting a long-lived service to tweak a value is undesirable. Treat "needs a daemon restart" as a gap to close. When adding a setting, also wire it into `reload()`.

As of v0.3.0 everything reloads live except `output`:

- Buttons / actions / theme (css, template, html): `reload()` rebuilds the HTML and swaps the `Dispatcher`.
- `dimmer_color` / `dimmer_opacity`: `App` keeps the surface `CssProvider`; `reload()` re-feeds it via `window::surface_css`, GTK restyles the live dimmer surfaces. See [[SurfaceTransparency]].
- `close_on_escape`: held as `EscapeEnabled` (`Rc<Cell<bool>>`) and gated at **key-press time**, not install time. The menu's key controller checks the cell; `reload()` calls `.set(...)`. `set_escape_hook` now always installs the hook (the cell decides whether it fires), so the daemon's hide-on-escape can be toggled live.
- `output`: **restart-required by design.** Changing the monitor rebuilds the layer surfaces, which the in-place reload path doesn't do; and on Hyprland `set_monitor` is ignored for the keyboard-grabbing menu anyway, so a live re-anchor would have little value. Documented as restart-required in the init template and README.

## Decision

- Land hot reload now, but design it against the in-memory `Config` model so it carries over to the daemon unchanged.
- The watcher swaps the in-memory model atomically; running invocations are not affected mid-flight.
- One-shot mode keeps the watcher too, even though the practical benefit is zero, so the daemon does not need a parallel code path.

## Why this order

Originally hot reload looked like leftover Phase 3 polish and daemon mode looked like the next big step. Reversed after noticing:

1. One-shot already re-reads the config on every cold start — there is nothing to "reload."
2. Daemon mode is the consumer that actually needs `notify`.
3. Building the watcher against the one-shot code path first means the daemon inherits a tested swap mechanism instead of growing one ad-hoc.

## Related
- [[StackDecision]]
- [[SurfaceTransparency]]
- [[HotReloadLoopBug]]
