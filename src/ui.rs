use crate::config::{Button, Config};
use std::path::Path;

pub const DEFAULT_CSS: &str = r#"
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
  grid-auto-flow: column;
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
button:active { transform: scale(0.97); }
.icon { font-size: 2.5rem; line-height: 1; }
.layout { display: flex; flex-direction: column; align-items: center; }
.hint { margin-top: 2rem; font-size: 0.9rem; opacity: 0.6; }
"#;

pub const DEFAULT_TEMPLATE: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
{{stylesheet}}
</head>
<body>
  <div class="layout">
    <div class="menu">{{buttons}}</div>
    <div class="hint">Click a button or press its keybind · Esc to cancel</div>
  </div>
  <script>{{script}}</script>
</body>
</html>"#;

const SCRIPT: &str = r#"
const send = (action) => window.webkit.messageHandlers.ipc.postMessage(action);
const buttons = [...document.querySelectorAll('button[data-action]')];
buttons.forEach(btn => btn.addEventListener('click', () => send(btn.dataset.action)));
document.addEventListener('keydown', (e) => {
  const key = e.key.toLowerCase();
  const btn = buttons.find(b => b.dataset.keybind && b.dataset.keybind.toLowerCase() === key);
  if (btn) send(btn.dataset.action);
});
"#;

pub struct Built {
    pub html: String,
}

pub fn build(config: &Config, config_dir: Option<&Path>) -> Built {
    let user_css = config_dir
        .and_then(|d| std::fs::read_to_string(d.join("style.css")).ok())
        .unwrap_or_default();
    let stylesheet_tag = format!("<style>{DEFAULT_CSS}\n{user_css}</style>");

    let buttons_html = render_buttons(&config.buttons);

    let template = config_dir
        .and_then(|d| std::fs::read_to_string(d.join("template.html")).ok())
        .unwrap_or_else(|| DEFAULT_TEMPLATE.into());

    let username = std::env::var("USER").unwrap_or_default();
    let hostname = std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let html = template
        .replace("{{stylesheet}}", &stylesheet_tag)
        .replace("{{buttons}}", &buttons_html)
        .replace("{{script}}", SCRIPT)
        .replace("{{username}}", &escape(&username))
        .replace("{{hostname}}", &escape(&hostname));

    Built { html }
}

fn render_buttons(buttons: &[Button]) -> String {
    buttons
        .iter()
        .enumerate()
        .map(|(i, b)| render_button(b, i == 0))
        .collect()
}

fn render_button(btn: &Button, autofocus: bool) -> String {
    let id = escape(&btn.id);
    let label = escape(&btn.label);
    let keybind = btn.keybind.as_deref().map(escape).unwrap_or_default();
    let icon = btn
        .icon
        .as_deref()
        .map(escape)
        .map(|s| format!("<span class=\"icon\">{s}</span>"))
        .unwrap_or_default();
    let autofocus = if autofocus { " autofocus" } else { "" };
    format!(
        "<button data-action=\"{id}\" data-keybind=\"{keybind}\"{autofocus}>{icon}<span>{label}</span></button>"
    )
}

fn escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            other => other.to_string(),
        })
        .collect()
}
