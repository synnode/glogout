---
title: "SurfaceTransparency"
tags: [layer-shell, gtk4, transparency, dimmer, theming, gotcha]
related: ["MultiMonitorPlacement", "StackDecision"]
updated: 2026-05-24
---

# SurfaceTransparency

Getting a transparent menu down to the desktop requires clearing background at **three** independent layers, top to bottom: the page CSS, the webview widget, and the GTK window. Missing any one leaves an opaque fill that hides the dimmer/desktop. Only the menu monitor is affected by the window layer, because only it carries the menu window on top of its dimmer.

## The three layers

1. **Page CSS** (`config.toml`/`style.css` via `ui::build`). The built-in default sets `html, body { background: var(--bg) }`. A user theme that wants transparency must set `background: transparent` on `html, body` (not just `body` — the default rule also paints `html`). Gotcha: if the user redefines `--bg` to an opaque color in `:root`, the still-active default rule paints that opaque color even after the user comments out their own `body` background.
2. **Webview widget** (`src/window.rs::build_menu`). `webview.set_background_color(rgba(0,0,0,0))` — already transparent.
3. **GTK window** (`src/window.rs`). GTK4 paints an opaque theme background on the `window` node. The menu window now carries the `glogout-menu` css class, and `install_surface_css` sets `window.glogout-menu { background: transparent }`. Without this, a fully transparent page revealed the opaque window fill on the menu monitor only, while other monitors (dimmer-only) showed the semi-transparent dimmer correctly.

## Dimmer

Dimmers are separate windows on `Layer::Top` with `window.glogout-dimmer { background: rgba(r,g,b,a) }` (semi-transparent — proves layer-shell alpha compositing works). The menu floats above on `Layer::Overlay`.

The dimmer fill is configurable via `[settings]`: `dimmer_color` (`#RRGGBB`/`#RGB`, default `#121216`) and `dimmer_opacity` (0.0–1.0, default 0.6). `window.rs::dimmer_fill` parses the hex, clamps opacity, and builds the rgba; bad color falls back to the default dark color while keeping the requested opacity. `dimmer_opacity` deserializes from int or float (`config.rs::de_opacity`) so a bare `0`/`1` doesn't fail the whole parse. Set `dimmer_opacity = 0.0` for a fully transparent overlay (desktop shows through).

## Related
- [[MultiMonitorPlacement]] — why every monitor is dimmed and the menu/dimmer layer split
- [[StackDecision]]
