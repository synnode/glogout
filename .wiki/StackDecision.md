---
title: "StackDecision"
tags: [architecture, gtk4, webkit6, wayland, validated]
related: ["SpikeFindings"]
updated: 2026-05-18
---

# StackDecision

**Decision (validated 2026-05-18):** the stack is `gtk4` + `webkit6` + `gtk4-layer-shell`, driven by a glib `MainLoop`. No tao, no wry. Spike-2 confirmed this works on Hyprland with explicit-sync enabled — the original spec stack (tao + wry + gtk-layer-shell on GTK3) hit a GTK3 bug that hits the entire target userbase.

## Why we pivoted away from the spec stack

**Symptom:** `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`
**Root cause (via `WAYLAND_DEBUG=1`):** `wp_linux_drm_syncobj_surface_v1, "Missing acquire timeline"`. GDK3 requests an explicit-sync surface but commits buffers without acquire-timeline points. **GTK3 bug, fixed in GTK4.14+, not backported.** Bumping wry 0.45 → 0.55.1 didn't help — wry still depends on `webkit2gtk` (gtk3).

Hits any compositor advertising `wp_linux_drm_syncobj_v1` — Hyprland (default), KWin 6, soon Sway. The target userbase for a CSS-themable logout menu (Arch + Hyprland tilers) is precisely the group affected.

## The validated stack

Cargo deps:
```toml
gtk4 = "0.11"
gtk4-layer-shell = "0.8"
webkit6 = "0.6"
anyhow = "1"
```

Native libs required: `gtk4`, `webkitgtk-6.0`, `gtk4-layer-shell-0`. All shipped on current Arch.

Wiring summary (see `src/main.rs` for the spike):
- `gtk4::init()` + a `glib::MainLoop` (no `gtk::Application` needed).
- `gtk4::Window::new()`, then `init_layer_shell()` + `set_layer(Overlay)` + four-edge anchors + `set_keyboard_mode(Exclusive)` + `set_namespace("glogout")` + `set_exclusive_zone(-1)`. The last one is what makes the overlay cover panels like waybar — without it, panels' reserved zones are subtracted from our anchored area.
- `webkit6::WebView` mounted as the window's child via `set_child`. Transparent via `set_background_color(RGBA::new(0,0,0,0))`.
- IPC: `UserContentManager::register_script_message_handler("ipc", None)` + `connect_script_message_received(Some("ipc"), ...)`. JS calls `window.webkit.messageHandlers.ipc.postMessage(...)`. Replaces wry's `with_ipc_handler` / `window.ipc.postMessage` pattern.
- Esc fallback: `EventControllerKey` on the window, added with `window.add_controller(...)`.

## Tradeoffs vs the original spec

| | spec stack | validated stack |
|--|--|--|
| Works on Hyprland w/ explicit sync | No | Yes |
| GTK3 themability bleed | N/A (webview) | N/A (webview) |
| LOC for IPC | ~3 (wry's handler) | ~6 (UserContentManager + signal) |
| Abstraction layers | wry over webkit2gtk | direct webkit6 |
| Wait on upstream | yes (wry GTK4 migration, no ETA) | no |

## Related

- [[SpikeFindings]] — chronological notes from both spikes
