---
title: "SurfaceTransparency"
tags: [layer-shell, gtk4, transparency, dimmer, theming, gotcha, blur, hyprland]
related: ["MultiMonitorPlacement", "StackDecision", "HotReloadScope"]
updated: 2026-05-24
---

# SurfaceTransparency

Getting a transparent menu down to the desktop requires clearing background at **three** independent layers, top to bottom: the page CSS, the webview widget, and the GTK window. Missing any one leaves an opaque fill that hides the dimmer/desktop. Only the menu monitor is affected by the window layer, because only it carries the menu window on top of its dimmer.

## The three layers

1. **Page CSS** (`config.toml`/`style.css` via `ui::build`). The built-in default sets `html, body { background: var(--bg) }`. A user theme that wants transparency must set `background: transparent` on `html, body` (not just `body` — the default rule also paints `html`). Gotcha: if the user redefines `--bg` to an opaque color in `:root`, the still-active default rule paints that opaque color even after the user comments out their own `body` background.
2. **Webview widget** (`src/window.rs::build_menu`). `webview.set_background_color(rgba(0,0,0,0))` — already transparent.
3. **GTK window** (`src/window.rs`). GTK4 paints an opaque theme background on the `window` node. The menu window now carries the `glogout-menu` css class, and the surface CSS sets `window.glogout-menu { background: transparent }`. Without this, a fully transparent page revealed the opaque window fill on the menu monitor only, while other monitors (dimmer-only) showed the semi-transparent dimmer correctly.

## Dimmer

Dimmers are separate windows on `Layer::Top` with `window.glogout-dimmer { background: rgba(r,g,b,a) }` (semi-transparent — proves layer-shell alpha compositing works). The menu floats above on `Layer::Overlay`.

Configurable via `[settings]`: `dimmer_color` (`#RRGGBB`/`#RGB`, default `#121216`) and `dimmer_opacity` (0.0–1.0, default 0.6). `window.rs::dimmer_fill` parses the hex, clamps opacity, and builds the rgba; bad color falls back to the default dark color while keeping the requested opacity. `dimmer_opacity` deserializes from int or float (`config.rs::de_opacity`) so a bare `0`/`1` doesn't fail the whole parse. Set `dimmer_opacity = 0.0` for a fully transparent overlay (desktop shows through).

**Hot-reloadable.** `window::install_surface_css` returns the `CssProvider`, which `App` keeps. `reload()` re-feeds it via `window::surface_css(...)` with the freshly parsed settings, so dimmer changes apply on save without a daemon restart — GTK restyles the existing dimmer surfaces in place. Key constraint: reuse the same provider (update its data), never add a new provider per reload or they stack. See [[HotReloadScope]].

## Blur: CSS can't, the compositor can

CSS `backdrop-filter: blur()` does **not** blur the desktop behind the overlay. A web engine can only sample its own surface, so the filter's backdrop is content *within the webview's document* — not the compositor's content underneath a transparent surface. With a transparent page there is nothing behind `body` in-document, so it's effectively a no-op for desktop show-through. It only does something when the page itself layers a translucent element over its own background.

Real frosted-glass = **compositor blur** (built into Hyprland, no plugin; needs global `decoration { blur { enabled = true } }`, the default). Every glogout surface sets the layer-shell namespace to `"glogout"` (`window.rs::anchor_to` → `set_namespace`), so blur can target it.

**Syntax is version-dependent — this bit a real user on 0.55.2:**

- Pre-0.53 (classic): `layerrule = blur, glogout` / `layerrule = ignorezero, glogout`.
- 0.53–0.55 (hyprlang structured rule blocks; matches the `windowrule { match:class = ... }` style): the classic comma form errors (`invalid field blur: missing a value`). Use a block, and note `ignorezero` became `ignore_alpha` (a 0.0–1.0 number; `0.0` ≈ old ignorezero):
  ```
  layerrule {
      name = glogout-blur
      match:namespace = ^glogout$
      blur = true
      ignore_alpha = 0.0
  }
  ```
- 0.55+ also supports a Lua config form: `hl.layer_rule({ match = { namespace = "^glogout$" }, blur = true, ignore_alpha = 0.0 })`.

Effects available on a layer rule: `blur`, `blur_popups`, `ignore_alpha`, `xray`, `dim_around`, `no_anim`, `order`, `above_lock`, `no_screen_share`. Pairs well with a low `dimmer_opacity`.

## Related
- [[MultiMonitorPlacement]] — why every monitor is dimmed and the menu/dimmer layer split
- [[StackDecision]]
- [[HotReloadScope]]
