use std::path::PathBuf;

use tokio::runtime::{Builder, Runtime};

use terminal_node::{NodeHostClient, NodeSubscriptionHandle, NodeSubscriptionSpec};
use terminal_protocol::{LocalSocketAddress, ProtocolError};

#[derive(Debug)]
pub struct TerminalCapiClientHandle {
    pub(crate) runtime: Runtime,
    pub(crate) client: NodeHostClient,
}

#[derive(Debug)]
pub struct TerminalCapiSubscriptionHandle {
    pub(crate) subscription: NodeSubscriptionHandle,
    pub(crate) runtime: Runtime,
}

#[derive(Debug)]
pub enum TerminalCapiHandleError {
    Runtime(std::io::Error),
    Protocol(ProtocolError),
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

impl TerminalCapiSubscriptionHandle {
    pub fn open(
        client: NodeHostClient,
        session_id: String,
        spec: NodeSubscriptionSpec,
    ) -> Result<Self, TerminalCapiHandleError> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(TerminalCapiHandleError::Runtime)?;
        let subscription = runtime
            .block_on(client.open_subscription(&session_id, &spec))
            .map_err(TerminalCapiHandleError::Protocol)?;

        Ok(Self { subscription, runtime })
    }
}
