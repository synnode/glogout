mod action;
mod config;
mod init;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use gtk4::glib::{self, MainLoop};
use gtk4::prelude::*;
use gtk4::{EventControllerKey, Window, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
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

    gtk4::init()?;

    let loaded = config::load();
    let dispatcher = Dispatcher::new(&loaded.config.buttons);
    let built = ui::build(&loaded.config, loaded.dir.as_deref());

    let main_loop = MainLoop::new(None, false);
    let window = Window::new();
    window.set_decorated(false);

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
        window.set_anchor(edge, true);
    }
    window.set_keyboard_mode(KeyboardMode::Exclusive);
    window.set_namespace(Some("glogout"));
    window.set_exclusive_zone(-1);

    if let Some(name) = loaded.config.settings.output.as_deref() {
        match pick_monitor(name) {
            Some(monitor) => window.set_monitor(Some(&monitor)),
            None => eprintln!("glogout: output {name:?} not found; using default"),
        }
    }

    let manager = UserContentManager::new();
    manager.register_script_message_handler("ipc", None);
    {
        let main_loop = main_loop.clone();
        let dispatcher = std::rc::Rc::new(dispatcher);
        manager.connect_script_message_received(Some("ipc"), move |_, value| {
            dispatcher.dispatch(value.to_str().as_str(), &main_loop);
        });
    }

    let webview = WebView::builder()
        .user_content_manager(&manager)
        .build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    webview.load_html(&built.html, None);
    window.set_child(Some(&webview));

    if loaded.config.settings.close_on_escape {
        let key_controller = EventControllerKey::new();
        let main_loop = main_loop.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                main_loop.quit();
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);
    }

    // TODO settings.close_on_focus_loss: focus events under
    // KeyboardMode::Exclusive don't fire predictably; needs investigation.

    window.present();
    main_loop.run();
    Ok(())
}

fn pick_monitor(name: &str) -> Option<gdk::Monitor> {
    let display = gdk::Display::default()?;
    let monitors = display.monitors();
    (0..monitors.n_items()).find_map(|i| {
        let monitor = monitors.item(i)?.downcast::<gdk::Monitor>().ok()?;
        let connector = monitor.connector()?;
        (connector.as_str() == name).then_some(monitor)
    })
}
