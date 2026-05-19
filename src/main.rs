mod action;
mod config;
mod init;
mod ui;
mod watch;
mod window;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use gtk4::glib::{self, MainLoop};
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

use crate::action::Dispatcher;

#[derive(Parser)]
#[command(version, about = "Themable Wayland logout menu", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Write default config files to $XDG_CONFIG_HOME/glogout/
    Init {
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Command::Init { force }) = cli.command {
        return init::run(force);
    }

    gtk4::init().context("GTK4 initialization failed (is DISPLAY/WAYLAND_DISPLAY set?)")?;

    if !gtk4_layer_shell::is_supported() {
        bail!(
            "this compositor does not implement wlr-layer-shell; glogout needs it to anchor as a fullscreen overlay.\n\
             Known working: Hyprland, KWin (Plasma 6), Sway. GNOME does not implement layer-shell."
        );
    }

    let loaded = config::load();
    let config_dir = loaded.dir.clone();
    let dispatcher = Rc::new(RefCell::new(Dispatcher::new(&loaded.config.buttons)));
    let built = ui::build(&loaded.config, loaded.dir.as_deref());

    let monitors = window::enumerate_monitors();
    if monitors.is_empty() {
        bail!("no monitors found via GDK; cannot anchor any layer-shell surface");
    }
    let menu_monitor = window::pick_menu_monitor(&monitors, loaded.config.settings.output.as_deref())
        .context("could not pick a menu monitor")?
        .clone();

    window::install_dimmer_css();

    let main_loop = MainLoop::new(None, false);

    let manager = UserContentManager::new();
    manager.register_script_message_handler("ipc", None);
    {
        let dispatcher = dispatcher.clone();
        let main_loop = main_loop.clone();
        manager.connect_script_message_received(Some("ipc"), move |_, value| {
            dispatcher.borrow().dispatch(value.to_str().as_str(), &main_loop);
        });
    }

    // Keep all surfaces alive for the lifetime of the main loop. They're
    // GObject ref-counted, so dropping the Vec at exit lets gtk clean up.
    let mut _surfaces = Vec::with_capacity(monitors.len());
    let (menu_window, menu_webview) = window::build_menu(
        &menu_monitor,
        &built.html,
        &manager,
        &main_loop,
        loaded.config.settings.close_on_escape,
    );
    _surfaces.push(menu_window);
    for monitor in &monitors {
        if monitor != &menu_monitor {
            _surfaces.push(window::build_dimmer(monitor));
        }
    }

    // Hot reload watcher. Only meaningful when we resolved a real config
    // dir — defaults are baked into the binary so there's nothing to
    // watch. Kept alive in a binding so the watcher thread isn't dropped.
    let _watch_handle = config_dir.as_deref().and_then(install_watch(dispatcher.clone(), menu_webview));

    // TODO settings.close_on_focus_loss: focus events under
    // KeyboardMode::Exclusive don't fire predictably; needs investigation.

    main_loop.run();
    Ok(())
}

/// Returns a closure that installs a watcher on a given dir and spawns
/// the reload listener on the GTK main context. Curried so the caller
/// can use `Option::and_then` for the no-config-dir case.
fn install_watch(
    dispatcher: Rc<RefCell<Dispatcher>>,
    webview: WebView,
) -> impl FnOnce(&Path) -> Option<watch::Handle> {
    move |dir: &Path| match watch::spawn(dir) {
        Ok((handle, rx)) => {
            let dir = dir.to_path_buf();
            glib::MainContext::default().spawn_local(async move {
                while rx.recv().await.is_ok() {
                    reload(&dir, &dispatcher, &webview);
                }
            });
            Some(handle)
        }
        Err(e) => {
            eprintln!("glogout: hot reload disabled: {e:#}");
            None
        }
    }
}

/// Re-read the config from `dir`, rebuild the menu HTML, swap the
/// dispatcher in place, and tell the webview to reload. Parse and read
/// errors are logged and the previous state is kept — a half-edited
/// config never bricks the open overlay.
fn reload(dir: &Path, dispatcher: &Rc<RefCell<Dispatcher>>, webview: &WebView) {
    let cfg = match config::load_from(dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("glogout: reload skipped — {e}");
            return;
        }
    };
    let built = ui::build(&cfg, Some(dir));
    *dispatcher.borrow_mut() = Dispatcher::new(&cfg.buttons);
    webview.load_html(&built.html, None);
    eprintln!("glogout: config reloaded");
}
