use anyhow::Result;
use gtk4::glib::{self, MainLoop};
use gtk4::prelude::*;
use gtk4::{EventControllerKey, Window, gdk};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::process::Command;
use webkit6::prelude::*;
use webkit6::{UserContentManager, WebView};

const HTML: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  html, body {
    margin: 0;
    height: 100%;
    width: 100%;
    background: rgba(0, 0, 0, 0.55);
    color: #f0f0f0;
    font-family: system-ui, sans-serif;
  }
  body {
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(12px);
  }
  .menu {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 1.5rem;
  }
  button {
    appearance: none;
    background: rgba(255, 255, 255, 0.06);
    border: 2px solid rgba(255, 255, 255, 0.15);
    color: inherit;
    font: inherit;
    font-size: 1.4rem;
    padding: 3rem 4rem;
    border-radius: 1rem;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease, transform 120ms ease;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: center;
  }
  button:hover, button:focus {
    background: rgba(255, 255, 255, 0.12);
    border-color: rgba(255, 255, 255, 0.4);
    outline: none;
  }
  button:active {
    transform: scale(0.97);
  }
  .icon {
    font-size: 2.5rem;
    line-height: 1;
  }
  .hint {
    margin-top: 2rem;
    font-size: 0.9rem;
    opacity: 0.6;
  }
  .layout {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>
</head>
<body>
  <div class="layout">
    <div class="menu">
      <button data-action="logout" autofocus>
        <span class="icon">⏻</span>
        <span>Log out</span>
      </button>
      <button data-action="reboot">
        <span class="icon">⟳</span>
        <span>Reboot</span>
      </button>
      <button data-action="cancel">
        <span class="icon">✕</span>
        <span>Cancel</span>
      </button>
    </div>
    <div class="hint">L · log out  ·  R · reboot  ·  Esc · cancel</div>
  </div>
  <script>
    const send = (action) => window.webkit.messageHandlers.ipc.postMessage(action);
    document.querySelectorAll('button[data-action]').forEach(btn => {
      btn.addEventListener('click', () => send(btn.dataset.action));
    });
    document.addEventListener('keydown', (e) => {
      switch (e.key.toLowerCase()) {
        case 'escape': send('cancel'); break;
        case 'l':      send('logout'); break;
        case 'r':      send('reboot'); break;
      }
    });
  </script>
</body>
</html>"#;

fn main() -> Result<()> {
    gtk4::init()?;

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

    let manager = UserContentManager::new();
    manager.register_script_message_handler("ipc", None);
    {
        let main_loop = main_loop.clone();
        manager.connect_script_message_received(Some("ipc"), move |_, value| {
            handle_action(value.to_str().as_str(), &main_loop);
        });
    }

    let webview = WebView::builder()
        .user_content_manager(&manager)
        .build();
    webview.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
    webview.load_html(HTML, None);
    window.set_child(Some(&webview));

    let key_controller = EventControllerKey::new();
    {
        let main_loop = main_loop.clone();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                main_loop.quit();
            }
            glib::Propagation::Proceed
        });
    }
    window.add_controller(key_controller);

    window.present();
    main_loop.run();
    Ok(())
}

fn handle_action(action: &str, main_loop: &MainLoop) {
    match action {
        "cancel" => {
            main_loop.quit();
        }
        "logout" => {
            let user = std::env::var("USER").unwrap_or_default();
            spawn_detached(Command::new("loginctl").args(["terminate-user", &user]));
            main_loop.quit();
        }
        "reboot" => {
            spawn_detached(Command::new("systemctl").arg("reboot"));
            main_loop.quit();
        }
        other => {
            eprintln!("glogout: unknown action {other:?}");
        }
    }
}

fn spawn_detached(cmd: &mut Command) {
    if let Err(e) = cmd.spawn() {
        eprintln!("glogout: failed to spawn {:?}: {e}", cmd.get_program());
    }
}
