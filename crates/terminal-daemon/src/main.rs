use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::LocalSocketAddress;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse(env::args().skip(1))?;
    let address = args
        .socket_path
        .map(LocalSocketAddress::Filesystem)
        .unwrap_or_else(|| LocalSocketAddress::from_runtime_slug(args.runtime_slug));
    let daemon = daemon_with_persistence(args.session_store_path.as_deref());
    let server = spawn_local_socket_server(daemon, address.clone())?;

    println!("terminal-daemon listening on {address}");

    tokio::signal::ctrl_c().await?;
    server.shutdown().await?;
    Ok(())
}

fn daemon_with_persistence(path: Option<&Path>) -> TerminalDaemon {
    let store = match path {
        Some(path) => SqliteSessionStore::open(path),
        None => SqliteSessionStore::open_default(),
    };

    match store {
        Ok(store) => TerminalDaemon::with_persistence(store),
        Err(error) => {
            eprintln!(
                "terminal-daemon persistence disabled - {error}. falling back to in-memory state"
            );
            TerminalDaemon::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliArgs {
    runtime_slug: String,
    socket_path: Option<PathBuf>,
    session_store_path: Option<PathBuf>,
}

impl CliArgs {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut runtime_slug = "default".to_string();
        let mut socket_path = None;
        let mut session_store_path = None;
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--runtime-slug" => {
                    runtime_slug = args.next().ok_or("missing value for --runtime-slug")?;
                }
                "--socket-path" => {
                    socket_path =
                        Some(PathBuf::from(args.next().ok_or("missing value for --socket-path")?));
                }
                "--session-store" => {
                    session_store_path = Some(PathBuf::from(
                        args.next().ok_or("missing value for --session-store")?,
                    ));
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => {
                    return Err(format!("unsupported argument: {other}").into());
                }
            }
        }

        Ok(Self { runtime_slug, socket_path, session_store_path })
    }
}

fn print_help() {
    println!(concat!(
        "terminal-daemon\n\n",
        "Options:\n",
        "  --runtime-slug <slug>     Local socket runtime slug. Default: default\n",
        "  --socket-path <path>      Override filesystem socket path\n",
        "  --session-store <path>    Override SQLite session store path\n",
        "  -h, --help                Show this help\n"
    ));
}
