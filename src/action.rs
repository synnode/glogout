use crate::config::Button;
use std::collections::HashMap;
use std::process::Command;

pub struct Dispatcher {
    by_id: HashMap<String, Resolved>,
}

enum Resolved {
    /// Just quit the main loop.
    Cancel,
    /// Spawn a command, then quit.
    Spawn(SpawnSpec),
    /// Resolution failed (unknown action, no command). Log and quit so the
    /// user is not stuck in a non-responsive overlay.
    Unknown(String),
}

struct SpawnSpec {
    program: String,
    args: Vec<String>,
}

impl Dispatcher {
    pub fn new(buttons: &[Button]) -> Self {
        let by_id = buttons
            .iter()
            .map(|b| (b.id.clone(), resolve(b)))
            .collect();
        Self { by_id }
    }

    /// Execute the side-effects for an action and return. The caller
    /// decides what to do after — one-shot mode quits the main loop,
    /// daemon mode hides the surfaces.
    pub fn dispatch(&self, action_id: &str) {
        match self.by_id.get(action_id) {
            Some(Resolved::Cancel) => {}
            Some(Resolved::Spawn(spec)) => {
                if let Err(e) = Command::new(&spec.program).args(&spec.args).spawn() {
                    eprintln!("glogout: failed to spawn {:?}: {e}", spec.program);
                }
            }
            Some(Resolved::Unknown(reason)) => {
                eprintln!("glogout: action {action_id:?} not runnable: {reason}");
            }
            None => {
                eprintln!("glogout: no button registered for action {action_id:?}");
            }
        }
    }
}

fn resolve(btn: &Button) -> Resolved {
    // An explicit `command` always wins. Run it via `sh -c` so users get
    // shell quoting, env expansion, and pipes "for free" without us
    // implementing a parser.
    if let Some(cmd) = btn.command.as_deref() {
        return Resolved::Spawn(SpawnSpec {
            program: "sh".into(),
            args: vec!["-c".into(), cmd.into()],
        });
    }

    // Otherwise resolve against built-ins by `action` field, falling back
    // to the button `id` so users can omit `action` for the common case.
    let name = btn.action.as_deref().unwrap_or(&btn.id);
    match builtin(name) {
        Some(b) => b,
        None => Resolved::Unknown(format!("unknown built-in action {name:?}")),
    }
}

fn builtin(name: &str) -> Option<Resolved> {
    let spawn = |program: &str, args: &[&str]| {
        Resolved::Spawn(SpawnSpec {
            program: program.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
        })
    };
    Some(match name {
        "cancel" => Resolved::Cancel,
        "logout" => {
            let user = std::env::var("USER").unwrap_or_default();
            Resolved::Spawn(SpawnSpec {
                program: "loginctl".into(),
                args: vec!["terminate-user".into(), user],
            })
        }
        "reboot" => spawn("systemctl", &["reboot"]),
        "shutdown" => spawn("systemctl", &["poweroff"]),
        "suspend" => spawn("systemctl", &["suspend"]),
        "hibernate" => spawn("systemctl", &["hibernate"]),
        "lock" => spawn("loginctl", &["lock-session"]),
        _ => return None,
    })
}
