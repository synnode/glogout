---
title: "HotReloadScope"
tags: [design-decision, hot-reload, daemon, roadmap]
related: ["StackDecision"]
updated: 2026-05-19
---

# HotReloadScope

Hot reload (config + assets via `notify`) is implemented **before** daemon mode as deliberate preparation, not as the last Phase 3 polish item. In one-shot mode the feature is essentially decorative — the overlay covers all outputs, so the user cannot edit a file while glogout is open. In daemon mode the long-running process holds the parsed config in memory across invocations, and that is where a config watcher becomes load-bearing: without it the user must restart the user service to pick up a `config.toml` change.

## Decision

- Land hot reload now, but design it against the in-memory `Config` model so it carries over to the daemon unchanged.
- The watcher swaps the in-memory model atomically; running invocations are not affected mid-flight.
- One-shot mode keeps the watcher too, even though the practical benefit is zero, so the daemon does not need a parallel code path.

## Why this order

Originally hot reload looked like leftover Phase 3 polish and daemon mode looked like the next big step. Reversed after noticing:

1. One-shot already re-reads the config on every cold start — there is nothing to "reload."
2. Daemon mode is the consumer that actually needs `notify`.
3. Building the watcher against the one-shot code path first means the daemon inherits a tested swap mechanism instead of growing one ad-hoc.

## Out of scope here

- Live re-theming of an *open* overlay. Theoretically possible (reload HTML/CSS in the webview on file change), but irrelevant for the same reason: the user can't reach an editor while it's open. Revisit only if a real use case appears.

## Related
- [[StackDecision]]
