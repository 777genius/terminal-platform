//! Node host bindings will live here.

use terminal_protocol::ProtocolVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeBindingVersion {
    pub protocol: ProtocolVersion,
}
