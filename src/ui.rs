use crate::config::{Button, Config};
use std::path::Path;

pub const DEFAULT_CSS: &str = r#"
:root {
  --bg: rgba(18, 18, 22, 0.6);
  --fg: #f2f2f2;
  --muted: rgba(255, 255, 255, 0.55);
  --button-bg: rgba(255, 255, 255, 0.05);
  --button-bg-hover: rgba(255, 255, 255, 0.1);
  --button-border: rgba(255, 255, 255, 0.1);
  --button-border-hover: rgba(255, 255, 255, 0.25);
  --accent: #7aa7ff;
}
html, body {
  margin: 0;
  height: 100%;
  width: 100%;
  background: var(--bg);
  color: var(--fg);
  font-family: system-ui, sans-serif;
}
body {
  display: flex;
  align-items: center;
  justify-content: center;
  backdrop-filter: blur(14px);
  animation: glogout-enter 180ms ease-out;
}
@keyframes glogout-enter {
  from { opacity: 0; transform: scale(0.98); }
  to   { opacity: 1; transform: scale(1); }
}
@media (prefers-reduced-motion: reduce) {
  body { animation: none; }
  button { transition: none; }
}
.layout {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.75rem;
}
.menu {
  display: grid;
  grid-auto-flow: column;
  gap: 1.25rem;
}
button {
  position: relative;
  appearance: none;
  background: var(--button-bg);
  border: 1px solid var(--button-border);
  color: inherit;
  font: inherit;
  font-size: 1.2rem;
  padding: 3rem 3.5rem;
  min-width: 9rem;
  border-radius: 0.85rem;
  cursor: pointer;
  transition: background 140ms ease, border-color 140ms ease,
              transform 140ms ease, box-shadow 140ms ease;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: center;
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.04) inset,
              0 8px 24px rgba(0, 0, 0, 0.25);
}
button:hover {
  background: var(--button-bg-hover);
  border-color: var(--button-border-hover);
  transform: translateY(-2px);
}
button:focus-visible {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(122, 167, 255, 0.4),
              0 8px 24px rgba(0, 0, 0, 0.3);
}
button:active { transform: translateY(0); }
.icon { font-size: 2.4rem; line-height: 1; }
.kbd {
  position: absolute;
  top: 0.7rem;
  right: 0.7rem;
  font-size: 0.65rem;
  font-weight: 600;
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 0.3rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.hint { font-size: 0.8rem; color: var(--muted); letter-spacing: 0.02em; }
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
    <div class="hint">Press a key, click a button, or hit Esc to cancel</div>
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
    let kbd_badge = if keybind.is_empty() {
        String::new()
    } else {
        format!("<span class=\"kbd\">{keybind}</span>")
    };
    let autofocus = if autofocus { " autofocus" } else { "" };
    format!(
        "<button data-action=\"{id}\" data-keybind=\"{keybind}\"{autofocus}>{kbd_badge}{icon}<span>{label}</span></button>"
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
