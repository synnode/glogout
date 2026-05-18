---
title: "SpikeFindings"
tags: [spike, wry, gtk, wayland, history]
related: ["StackDecision"]
updated: 2026-05-18
---

# SpikeFindings

Chronological notes from the proof-of-concept spike on 2026-05-18 that validated the layer-shell + webview approach on Hyprland. Outcome: see [[StackDecision]] — the spec stack does not work and we pivot to gtk4 + webkit6.

## What we tested

Goal: tao + wry + gtk-layer-shell, fullscreen overlay, keyboard-exclusive, with a magenta probe page to confirm transparency and rendering. Esc to exit via JS IPC.

## Issues hit, in order

### 1. `gtk-layer-shell::set_keyboard_mode` not found
**Cause:** Method is gated behind feature `v0_6`.
**Fix:** `gtk-layer-shell = { version = "0.8", features = ["v0_6"] }`

### 2. `HandleError::Unavailable` from wry's `WebViewBuilder::new(&window)`
**Cause:** Wry's default builder uses `raw_window_handle::HasWindowHandle`. On Wayland the wl_surface is not assigned until the window is mapped, so a window built with `with_visible(false)` (required by layer-shell) has no handle.
**Fix:** Use `WebViewBuilder::new_gtk(&gtk_container)` (the `WebViewBuilderExtUnix` trait). This bypasses raw-window-handle entirely and mounts on a GtkContainer directly. Wry's own docs explicitly recommend this for Wayland.

### 3. GTK warning: `as a GtkBin subclass a GtkApplicationWindow can only contain one widget at a time; it already contains a widget of type GtkBox`
**Cause:** tao inserts its own GtkBox as the ApplicationWindow's child. Mounting wry on `window.gtk_window()` tries to add a second child, which is rejected silently — overlay appears but is blank.
**Fix:** Walk the children of the ApplicationWindow, downcast the existing GtkBox to GtkContainer, and pass *that* to `WebViewBuilder::new_gtk(&host)`.

### 4. `Wayland Error 71 (Protocol error)` — the real blocker
**Cause** (revealed by `WAYLAND_DEBUG=1`): `wp_linux_drm_syncobj_surface_v1, "Missing acquire timeline"`. GDK3 requests an explicit-sync surface but commits buffers without acquire-timeline points. **GTK3 bug, fixed in GTK4.14+, not backported.** Hits any compositor advertising `wp_linux_drm_syncobj_v1` — Hyprland with explicit_sync enabled, KWin 6, etc.
**Not fixed** by removing tao, by using plain `gtk::Window`, by reordering the mount sequence, or by bumping wry 0.45 → 0.55.1. The bug is in webkit2gtk's underlying GTK3, untouchable from our code.

## Things we tried that did NOT fix the protocol error

- `gtk_window.realize()` before wry mount
- `WebViewBuilder::new_gtk(&fixed)` on a fresh `gtk::Fixed` child
- Dropping tao entirely, using `gtk::Window` directly
- Mounting wry before vs after `show_all()`
- Upgrading wry 0.45 → 0.55.1

## The pivot

See [[StackDecision]] for the conclusion: replace the wry/webkit2gtk/gtk3 stack with webkit6/gtk4/gtk4-layer-shell.

## Related

- [[StackDecision]]
