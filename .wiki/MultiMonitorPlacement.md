---
title: "MultiMonitorPlacement"
tags: [multi-monitor, layer-shell, hyprland, gotcha, window]
related: ["StackDecision", "DaemonMode"]
updated: 2026-06-03
---

# MultiMonitorPlacement

The menu sits on `Layer::Overlay`; the dimmers sit on `Layer::Top`; **every** monitor gets a dimmer including the menu's own. The dimmer set is also rebuilt on `gdk::Display::monitors()` items-changed, because the startup snapshot can be incomplete. This layout is a deliberate robustness measure — do not "simplify" it back to dimming only the non-menu monitors without re-reading the history below.

## The bug it fixed

Reported after a few days of real use: on a multi-monitor setup the menu opened on the monitor under the cursor, and when the cursor was on a non-primary screen the menu appeared *behind* that screen's dimmer and was unusable.

Two distinct causes were in play:

1. **Same-layer stacking (definitely fixed).** Originally both menu and dimmers were on `Layer::Overlay`, and `App::show()` presents the menu first, dimmers after. Within one layer, later-mapped surfaces stack on top — so a dimmer mapped over the menu. Fix: menu → `Layer::Overlay`, dimmers → `Layer::Top`. Overlay is globally above Top in the wlr-layer-shell stack (background < bottom < top < overlay), so the menu always renders above any dimmer regardless of output or map order. This is protocol-defined.

2. **Menu placement following the cursor (timing-dependent).** The menu (`KeyboardMode::Exclusive`) appeared to ignore `set_monitor` and land on the focused/cursor output, while the passive dimmers (`KeyboardMode::None`) honored `set_monitor`. **However**, this turned out to be build-dependent: under a debug `cargo run` the menu followed the cursor, but a `--release` binary lands on the requested output reliably (observed 0/20 on the cursor monitor across triggers from different screens, and a daemon restarted from a non-primary still opened on primary). Best current explanation: `set_monitor` races Hyprland's focus-based placement, and the release build applies it fast enough to win. Not definitively proven — treat as timing-sensitive.

## Why we still dim every monitor

Because cause #2 is timing-sensitive rather than fully understood, dimming **all** monitors (dropped the old `if monitor != &menu_monitor` skip) is kept as a safety net: if placement ever flips back to cursor-following (slower machine, heavy load, a different compositor), every screen still darkens and the menu — on the higher Overlay layer — stays visible on whichever output it lands. With the old skip, a flipped placement would leave the intended menu monitor bright and undimmed.

`pick_menu_monitor` / `settings.output` / the (0,0) primary heuristic decide where the menu is *requested*; in release builds on Hyprland that request is honored.

## Late-arriving monitor (the boot-time GPU/source race)

Even with every-monitor dimming, one failure mode persisted in daemon mode: build_app's `enumerate_monitors()` is a single snapshot of `gdk::Display::default().monitors()` taken at daemon start, and if the snapshot is missing an output that comes online seconds later, that output never gets a dimmer. Observed in the wild on a multi-GPU rig where the primary briefly attaches to a passthrough HDMI on the secondary GPU during boot before switching back to DP on the main GPU. The user saw: menu lands on cursor monitor (which had a dimmer), but the real primary stayed bright because no dimmer was ever built for it. Cause #2 above hid the underlying cause-#3 here for a while because the visible symptom (menu on cursor monitor) looked like a recurrence of the timing race.

Fix: `window::watch_monitor_changes` connects an `items-changed` handler on the GDK monitor list (`src/window.rs`). On every delta it destroys the existing dimmer windows, rebuilds one per currently-connected monitor via `build_dimmer`, and presents the new ones immediately if the menu is currently visible. The menu window is left untouched — it owns the WebKitGTK process that [[DaemonMode]] is built around keeping alive across hide/show, so rebuilding it would cost that whole optimization. The `App.dimmers` field is therefore `Rc<RefCell<Vec<Window>>>` rather than part of an immutable `surfaces` Vec.

If a monitor change arrives while glogout is open, the user sees a brief flicker as the dimmer set is swapped — accepted, because the OS-level layout switch that triggered the items-changed is itself a visible event, so an extra flicker is in line with what the user already expects in that moment.

## Known cosmetic consequence

The monitor the menu lands on carries both a dimmer (Top) and the menu body background (`rgba(18,18,22,0.6)` + blur). Because the body is only ~60% opaque, the dimmer behind it bleeds through, stacking to ~0.84 effective darkness vs 0.6 on the others. Whether it's *visible* depends on the wallpaper: near-invisible on a dark one (both approach the dim color), clear on a bright one. Accepted as a minor tradeoff for the safety net.

## Future idea: uniform dimming via transparent menu body

Because every monitor now carries a dimmer, a fully transparent menu `body` would let the dimmer show through on the menu monitor too, giving uniform 0.6 dimming everywhere (the buttons keep their own backgrounds; `backdrop-filter: blur` would still apply only on the menu monitor). Shape this as a config toggle (e.g. `settings.dim_menu_monitor`), not a default change — the menu body backdrop is part of the themeable-via-`style.css` contract while the dimmer color is hardcoded in Rust. Deferred; not urgent.

## Decision

User was offered "menu where the cursor is, always visible" vs "force primary" and picked the former; in practice the release build delivers reliable primary placement, which is what the user originally wanted anyway. Behavior is consistent across one-shot and daemon.

## Related

- [[StackDecision]] — the validated gtk4 + webkit6 + gtk4-layer-shell stack
- [[DaemonMode]] — surfaces are built once and present/hidden; same surface set
