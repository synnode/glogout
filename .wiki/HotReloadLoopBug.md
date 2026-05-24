---
title: "HotReloadLoopBug"
tags: [hot-reload, bug, notify, watcher, daemon, gotcha]
related: ["HotReloadScope", "DaemonMode"]
updated: 2026-05-24
---

# HotReloadLoopBug

A single config edit could spin the hot-reload watcher into an endless `glogout: config reloaded` loop (one reload per ~150ms debounce tick), most visible in daemon mode where the process runs forever. Latent since Phase 3 (`src/watch.rs`), surfaced as an "inconsistent" reload loop.

## Root cause

`reload()` (`src/main.rs`) reads all three watched files — `config.toml` via `config::load_from`, and `style.css` + `template.html` via `ui::build`. On Linux, inotify emits `Access` events (open/read/close) for those reads. The watcher's relevance filter in `src/watch.rs` matched on **filename only** (`RELEVANT_FILES.contains(&n)`), ignoring event kind — so a mere *read* counted as relevant. Edit → watcher fires → reload reads files → read emits Access events → watcher fires again → loop, self-sustaining at the debounce interval.

Inconsistency came from timing: the loop only sustains when the read's Access events land in a fresh debounce window vs. the tail of the current one.

## Fix

In the debouncer callback, drop access events before the filename match:
`events.iter().filter(|e| !e.kind.is_access())`. Only create/modify/remove events trigger a reload. Confirmed with a standalone repro on `notify 8.2.0` + `notify-debouncer-full 0.5.0`: pre-fix a single edit produced 60+ reloads and counting; post-fix 3 distinct edits produced exactly 3 reloads even though the callback reads all files each time.

## Gotcha for future watchers

Any watcher whose reaction *reads* a watched file must filter out `Access` events, or it will feed itself. Filename-only relevance checks are not enough.

## Related
- [[HotReloadScope]]
- [[DaemonMode]]
