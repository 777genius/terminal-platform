use std::path::PathBuf;

use tokio::runtime::{Builder, Runtime};

use terminal_node::NodeHostClient;
use terminal_protocol::LocalSocketAddress;

#[derive(Debug)]
pub struct TerminalCapiClientHandle {
    pub(crate) runtime: Runtime,
    pub(crate) client: NodeHostClient,
}

impl TerminalCapiClientHandle {
    pub fn from_runtime_slug(slug: String) -> Result<Self, std::io::Error> {
        Self::from_address(LocalSocketAddress::from_runtime_slug(slug))
    }

    pub fn from_namespaced_address(value: String) -> Result<Self, std::io::Error> {
        Self::from_address(LocalSocketAddress::Namespaced(value))
    }

    pub fn from_filesystem_path(path: String) -> Result<Self, std::io::Error> {
        Self::from_address(LocalSocketAddress::Filesystem(PathBuf::from(path)))
    }

    fn from_address(address: LocalSocketAddress) -> Result<Self, std::io::Error> {
        Ok(Self {
            runtime: Builder::new_multi_thread().enable_all().build()?,
            client: NodeHostClient::new(address),
        })
    }
}
