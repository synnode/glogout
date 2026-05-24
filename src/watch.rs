//! Debounced filesystem watcher for the config directory.
//!
//! Emits a unit event on the returned channel whenever `config.toml`,
//! `style.css`, or `template.html` is touched inside the watched dir.
//! Editor save patterns (rename-into-place, multi-step writes) are
//! collapsed into one event via `notify-debouncer-full`.
//!
//! The returned [`Handle`] owns the watcher thread; dropping it stops
//! the watcher. Caller is expected to keep it alive for the lifetime
//! of the program.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use async_channel::Receiver;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

const DEBOUNCE: Duration = Duration::from_millis(150);

const RELEVANT_FILES: &[&str] = &["config.toml", "style.css", "template.html"];

pub struct Handle {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

pub fn spawn(dir: &Path) -> Result<(Handle, Receiver<()>)> {
    let (tx, rx) = async_channel::unbounded::<()>();

    let mut debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| match result {
        Ok(events) => {
            // Skip `Access` events (open/read/close). Our own reload() reads
            // all three watched files, which emits inotify access events on
            // them — and since the filter below matches on filename alone,
            // those reads would re-trigger the watcher and spin a reload loop.
            // Only content-changing events (create/modify/remove) reload.
            let relevant = events.iter().filter(|e| !e.kind.is_access()).any(|e| {
                e.paths.iter().any(|p| {
                    p.file_name()
                        .and_then(|s| s.to_str())
                        .map(|n| RELEVANT_FILES.contains(&n))
                        .unwrap_or(false)
                })
            });
            if relevant {
                let _ = tx.send_blocking(());
            }
        }
        Err(errors) => {
            for e in errors {
                eprintln!("glogout: watcher error: {e}");
            }
        }
    })
    .context("failed to create config watcher")?;

    debouncer
        .watch(dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("failed to watch {}", dir.display()))?;

    Ok((Handle { _debouncer: debouncer }, rx))
}
