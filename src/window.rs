use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{CssProvider, EventControllerKey, Window, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

/// Apply the layer-shell properties shared by both menu and dimmer surfaces.
///
/// The menu sits on `Layer::Overlay` and the dimmers on `Layer::Top`. That
/// layer split is load-bearing: Hyprland places keyboard-interactive layer
/// surfaces (the menu) on the *focused* output and ignores our requested
/// monitor, so the menu can land on a monitor that also carries a dimmer.
/// Overlay is globally above Top in the wlr-layer-shell stack, so the menu
/// always renders on top of any dimmer regardless of which monitor it lands
/// on or the order surfaces were mapped in.
fn anchor_to(window: &Window, monitor: &gdk::Monitor, layer: Layer, keyboard: KeyboardMode) {
    window.init_layer_shell();
    window.set_layer(layer);
    window.set_monitor(Some(monitor));
    for edge in [Edge::Top, Edge::Right, Edge::Bottom, Edge::Left] {
        window.set_anchor(edge, true);
    }
    window.set_keyboard_mode(keyboard);
    window.set_namespace(Some("glogout"));
    window.set_exclusive_zone(-1);
}

/// Late-bound Escape handler. The menu window installs a key controller
/// at construction time, but the actual behavior (quit vs. hide) is only
/// known once the runner sets it. The window holds an `Rc<RefCell<_>>`
/// so the handler can be swapped without touching GTK plumbing.
pub type EscapeHook = std::rc::Rc<std::cell::RefCell<Option<Box<dyn Fn()>>>>;

/// The window that hosts the actual menu UI. Exactly one of these per run.
/// Returns both the window and the inner webview — the webview is exposed
/// so hot reload can call `load_html` on it after the program is up.
///
/// The window is not presented here — callers `present()` it themselves
/// once they are ready to map the layer-shell surface (one-shot maps
/// immediately; daemon waits for a `show` command).
///
/// `escape_hook` is consulted on every Escape press; if it holds `Some`,
/// the callback fires. One-shot fills it with `main_loop.quit`; daemon
/// fills it with `app.hide`.
pub fn build_menu(
    monitor: &gdk::Monitor,
    html: &str,
    manager: &UserContentManager,
    escape_hook: EscapeHook,
) -> (Window, WebView) {
    let window = Window::new();
    window.set_decorated(false);
    anchor_to(&window, monitor, Layer::Overlay, KeyboardMode::Exclusive);

    let webview = WebView::builder()
        .user_content_manager(manager)
        .build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    webview.load_html(html, None);
    window.set_child(Some(&webview));

    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape
            && let Some(hook) = escape_hook.borrow().as_ref()
        {
            hook();
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key_controller);

    (window, webview)
}

/// A lightweight dimmer surface. No webview, no keyboard grab — just a flat
/// semi-transparent background so every screen darkens and the action reads
/// as modal. Built for *every* monitor (including the menu's): the menu sits
/// on a higher layer and floats above its own monitor's dimmer, which keeps
/// behavior correct even though we can't predict which output Hyprland will
/// drop the keyboard-grabbing menu onto. Not presented — caller does that.
pub fn build_dimmer(monitor: &gdk::Monitor) -> Window {
    let window = Window::new();
    window.set_decorated(false);
    window.add_css_class("glogout-dimmer");
    anchor_to(&window, monitor, Layer::Top, KeyboardMode::None);
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
///
/// Note: this is only a *hint*. Compositors that honor a layer surface's
/// requested output (KWin, sway) will place the menu here; Hyprland ignores
/// it for keyboard-interactive surfaces and uses the focused output instead.
/// Either way the menu stays visible because every monitor is dimmed and the
/// menu floats above on a higher layer.
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
