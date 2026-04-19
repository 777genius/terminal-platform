//! Daemon client transport will live here.

use terminal_protocol::ProtocolVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}
