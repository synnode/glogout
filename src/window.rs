use gtk4::glib::{self, MainLoop};
use gtk4::prelude::*;
use gtk4::{CssProvider, EventControllerKey, Window, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

/// Apply the layer-shell properties shared by both menu and dimmer surfaces.
fn anchor_to(window: &Window, monitor: &gdk::Monitor, keyboard: KeyboardMode) {
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_monitor(Some(monitor));
    for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
        window.set_anchor(edge, true);
    }
    window.set_keyboard_mode(keyboard);
    window.set_namespace(Some("glogout"));
    window.set_exclusive_zone(-1);
}

/// The window that hosts the actual menu UI. Exactly one of these per run.
/// Returns both the window and the inner webview — the webview is exposed
/// so hot reload can call `load_html` on it after the program is up.
pub fn build_menu(
    monitor: &gdk::Monitor,
    html: &str,
    manager: &UserContentManager,
    main_loop: &MainLoop,
    close_on_escape: bool,
) -> (Window, WebView) {
    let window = Window::new();
    window.set_decorated(false);
    anchor_to(&window, monitor, KeyboardMode::Exclusive);

    let webview = WebView::builder()
        .user_content_manager(manager)
        .build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    webview.load_html(html, None);
    window.set_child(Some(&webview));

    if close_on_escape {
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

    window.present();
    (window, webview)
}

/// A lightweight dimmer surface for non-menu monitors. No webview, no
/// keyboard grab — just a flat semi-transparent background so the menu
/// monitor stands out and the user understands the action is modal.
pub fn build_dimmer(monitor: &gdk::Monitor) -> Window {
    let window = Window::new();
    window.set_decorated(false);
    window.add_css_class("glogout-dimmer");
    anchor_to(&window, monitor, KeyboardMode::None);
    window.present();
    window
}

/// Install the dimmer-surface CSS at application scope. Idempotent enough
/// for our one-shot usage — should be called once at startup.
pub fn install_dimmer_css() {
    let provider = CssProvider::new();
    provider.load_from_data("window.glogout-dimmer { background: rgba(18, 18, 22, 0.6); }");
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Enumerate the connected monitors. Returns an empty Vec if no display
/// or no monitors are available.
pub fn enumerate_monitors() -> Vec<gdk::Monitor> {
    let Some(display) = gdk::Display::default() else {
        return Vec::new();
    };
    let monitors = display.monitors();
    (0..monitors.n_items())
        .filter_map(|i| monitors.item(i)?.downcast::<gdk::Monitor>().ok())
        .collect()
}

/// Pick the menu monitor: the one matching `wanted` (by connector name).
/// Without a configured output, prefer the monitor at logical (0, 0) — that
/// is conventionally the user's primary on both X11 and Wayland setups —
/// and fall back to the first listed if no monitor sits at the origin.
pub fn pick_menu_monitor<'a>(
    monitors: &'a [gdk::Monitor],
    wanted: Option<&str>,
) -> Option<&'a gdk::Monitor> {
    if let Some(name) = wanted {
        let found = monitors
            .iter()
            .find(|m| m.connector().map(|c| c.as_str() == name).unwrap_or(false));
        if found.is_none() {
            eprintln!("glogout: output {name:?} not found; falling back to primary heuristic");
        }
        if let Some(m) = found {
            return Some(m);
        }
    }
    monitors
        .iter()
        .find(|m| {
            let g = m.geometry();
            g.x() == 0 && g.y() == 0
        })
        .or_else(|| monitors.first())
}
