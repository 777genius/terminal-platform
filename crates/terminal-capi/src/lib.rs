//! C ABI bindings will live here.

use terminal_protocol::ProtocolVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapiVersion {
    pub protocol: ProtocolVersion,
}
