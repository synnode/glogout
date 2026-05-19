//! Daemon mode plumbing: Unix socket server and matching client.
//!
//! The server listens at `$XDG_RUNTIME_DIR/glogout.sock` and translates
//! line-based commands (`show\n`) into [`Command`] values delivered on
//! an `async_channel` that the GTK main context can poll.
//!
//! The client opens the same socket, writes the command, and exits.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use async_channel::Receiver;

#[derive(Debug)]
pub enum Command {
    Show,
    Toggle,
}

/// Resolve the socket path from `$XDG_RUNTIME_DIR`. Returns an error if
/// the variable is unset — without it we have no reliable user-private
/// runtime directory to put a socket in.
pub fn socket_path() -> Result<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR is not set; cannot place daemon socket")?;
    Ok(PathBuf::from(dir).join("glogout.sock"))
}

/// Guard that removes the socket file on drop, so a clean daemon exit
/// does not leave a stale path behind for the next start.
pub struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Bind the daemon socket and start the accept loop. Returns a channel
/// of commands plus a guard that owns the socket file.
///
/// If the socket path already exists, we probe it: a successful connect
/// means another daemon is alive and we refuse to start; a failed connect
/// means a stale file we are free to unlink.
pub fn spawn_server() -> Result<(SocketGuard, Receiver<Command>)> {
    let path = socket_path()?;

    if path.exists() {
        match UnixStream::connect(&path) {
            Ok(_) => bail!(
                "another glogout daemon is already running (socket {} is live)",
                path.display()
            ),
            Err(_) => {
                std::fs::remove_file(&path)
                    .with_context(|| format!("failed to remove stale socket {}", path.display()))?;
            }
        }
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("failed to bind {}", path.display()))?;
    let guard = SocketGuard(path);

    let (tx, rx) = async_channel::unbounded::<Command>();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            match line.trim() {
                "show" => {
                    let _ = tx.send_blocking(Command::Show);
                }
                "toggle" => {
                    let _ = tx.send_blocking(Command::Toggle);
                }
                "" => {}
                other => eprintln!("glogout daemon: unknown command {other:?}"),
            }
        }
    });

    Ok((guard, rx))
}

/// Connect to a running daemon and send a single command line.
pub fn client_send(command: &str) -> Result<()> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path).with_context(|| {
        format!(
            "could not reach daemon at {} (is `glogout daemon` running?)",
            path.display()
        )
    })?;
    writeln!(stream, "{command}")?;
    Ok(())
}
