use terminal_daemon_client::LocalSocketDaemonClient;

mod connection;
mod live_sessions;
mod saved_sessions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeHostClient {
    client: LocalSocketDaemonClient,
}
