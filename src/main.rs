mod action;
mod config;
mod daemon;
mod init;
mod ui;
mod watch;
mod window;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use gtk4::Window;
use gtk4::glib::{self, MainLoop};
use gtk4::prelude::*;
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

use crate::action::Dispatcher;
use crate::window::EscapeHook;

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
    /// Run as a long-lived background service. A subsequent `glogout show`
    /// reveals the menu without re-spawning the webview.
    Daemon,
    /// Reveal a running daemon's menu. Errors out if no daemon is reachable.
    Show,
    /// Show the menu if hidden, hide it if shown. Useful for a single keybind.
    Toggle,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Init { force }) => init::run(force),
        Some(Command::Show) => daemon::client_send("show"),
        Some(Command::Toggle) => daemon::client_send("toggle"),
        Some(Command::Daemon) => run_daemon(),
        None => run_one_shot(),
    }
}

/// Everything that survives between invocations in daemon mode and that
/// the one-shot path also wants: the GTK main loop, the constructed
/// surfaces, the webview that hosts the menu, the live dispatcher, and
/// the config dir (used by the hot-reload watcher).
struct App {
    main_loop: MainLoop,
    surfaces: Vec<Window>,
    webview: WebView,
    dispatcher: Rc<RefCell<Dispatcher>>,
    config_dir: Option<PathBuf>,
    manager: UserContentManager,
    escape_hook: EscapeHook,
    close_on_escape: bool,
}

impl App {
    fn show(&self) {
        for surface in &self.surfaces {
            surface.present();
        }
    }

    fn hide(&self) {
        for surface in &self.surfaces {
            surface.set_visible(false);
        }
    }

    /// True when the menu surface is currently mapped. Used to decide
    /// what `toggle` should do.
    fn is_visible(&self) -> bool {
        self.surfaces
            .first()
            .map(|s| s.is_visible())
            .unwrap_or(false)
    }

    fn toggle(&self) {
        if self.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Replace the Escape callback. `close_on_escape = false` in the
    /// config means we ignore the assignment so users can't accidentally
    /// re-enable it via daemon wiring.
    fn set_escape_hook<F: Fn() + 'static>(&self, hook: F) {
        if !self.close_on_escape {
            return;
        }
        *self.escape_hook.borrow_mut() = Some(Box::new(hook));
    }
}

fn build_app() -> Result<App> {
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

    let escape_hook: EscapeHook = Rc::new(RefCell::new(None));
    let (menu_window, menu_webview) =
        window::build_menu(&menu_monitor, &built.html, &manager, escape_hook.clone());

    // Dim every monitor and float the menu above on Layer::Overlay. We dim
    // the menu's own monitor too: Hyprland ignores the menu's requested
    // output and drops it on the focused screen, so we can't reliably know
    // which monitor to leave undimmed. The layer split keeps the menu on top
    // of its own dimmer regardless.
    let mut surfaces = Vec::with_capacity(monitors.len() + 1);
    surfaces.push(menu_window);
    for monitor in &monitors {
        surfaces.push(window::build_dimmer(monitor));
    }

    // TODO settings.close_on_focus_loss: focus events under
    // KeyboardMode::Exclusive don't fire predictably; needs investigation.

    Ok(App {
        main_loop,
        surfaces,
        webview: menu_webview,
        dispatcher,
        config_dir,
        manager,
        escape_hook,
        close_on_escape: loaded.config.settings.close_on_escape,
    })
}

fn run_one_shot() -> Result<()> {
    let app = build_app()?;

    let main_loop = app.main_loop.clone();
    app.set_escape_hook(move || main_loop.quit());

    {
        let dispatcher = app.dispatcher.clone();
        let main_loop = app.main_loop.clone();
        app.manager
            .connect_script_message_received(Some("ipc"), move |_, value| {
                dispatcher.borrow().dispatch(value.to_str().as_str());
                main_loop.quit();
            });
    }

    let _watch = install_watch(&app);
    app.show();
    app.main_loop.run();
    Ok(())
}

fn run_daemon() -> Result<()> {
    // Bind the socket before doing GTK work — fail fast on a duplicate
    // daemon, before we spin up a webview process we'd then have to tear
    // back down.
    let (_socket_guard, commands) = daemon::spawn_server()?;

    let app = Rc::new(build_app()?);

    {
        let app_for_closure = app.clone();
        app.set_escape_hook(move || app_for_closure.hide());
    }

    {
        let app_for_closure = app.clone();
        app.manager
            .connect_script_message_received(Some("ipc"), move |_, value| {
                app_for_closure
                    .dispatcher
                    .borrow()
                    .dispatch(value.to_str().as_str());
                app_for_closure.hide();
            });
    }

    let _watch = install_watch(&app);

    // Listen for socket commands and translate Show into surface presents.
    {
        let app = app.clone();
        glib::MainContext::default().spawn_local(async move {
            while let Ok(cmd) = commands.recv().await {
                match cmd {
                    daemon::Command::Show => app.show(),
                    daemon::Command::Toggle => app.toggle(),
                }
            }
        });
    }

    // Daemon starts with surfaces built but hidden; user invokes
    // `glogout show` to reveal them.
    app.main_loop.run();
    Ok(())
}

/// Install the hot-reload watcher on `app.config_dir` if we have one.
/// Returns the watcher handle, which the caller is expected to keep
/// alive (drop = stop watching).
fn install_watch(app: &App) -> Option<watch::Handle> {
    let dir = app.config_dir.as_deref()?;
    match watch::spawn(dir) {
        Ok((handle, rx)) => {
            let dir = dir.to_path_buf();
            let dispatcher = app.dispatcher.clone();
            let webview = app.webview.clone();
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
