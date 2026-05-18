mod action;
mod config;
mod init;
mod ui;
mod window;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use gtk4::glib::MainLoop;
use webkit6::UserContentManager;

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
    let dispatcher = Dispatcher::new(&loaded.config.buttons);
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
        let main_loop = main_loop.clone();
        let dispatcher = std::rc::Rc::new(dispatcher);
        manager.connect_script_message_received(Some("ipc"), move |_, value| {
            dispatcher.dispatch(value.to_str().as_str(), &main_loop);
        });
    }

    // Keep all surfaces alive for the lifetime of the main loop. They're
    // GObject ref-counted, so dropping the Vec at exit lets gtk clean up.
    let mut _surfaces = Vec::with_capacity(monitors.len());
    _surfaces.push(window::build_menu(
        &menu_monitor,
        &built.html,
        &manager,
        &main_loop,
        loaded.config.settings.close_on_escape,
    ));
    for monitor in &monitors {
        if monitor != &menu_monitor {
            _surfaces.push(window::build_dimmer(monitor));
        }
    }

    // TODO settings.close_on_focus_loss: focus events under
    // KeyboardMode::Exclusive don't fire predictably; needs investigation.

    main_loop.run();
    Ok(())
}
