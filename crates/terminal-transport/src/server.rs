mod connection;
mod listener;
mod shutdown;

use std::{io, sync::Arc};

use interprocess::local_socket::traits::tokio::Listener as _;
use terminal_protocol::LocalSocketAddress;
use tokio::{
    sync::{oneshot, watch},
    task::{JoinHandle, JoinSet},
};

use crate::{TransportRequestHandler, TransportSubscriptionHandler};

use self::{
    connection::handle_connection,
    listener::create_listener,
    shutdown::{drain_or_abort_connection_tasks, join_error_to_io},
};

pub struct LocalSocketServerHandle {
    address: LocalSocketAddress,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<io::Result<()>>,
}

impl LocalSocketServerHandle {
    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        &self.address
    }

    pub async fn shutdown(mut self) -> io::Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        self.task.await.map_err(join_error_to_io)?
    }
}

pub fn spawn_local_socket_server<Handler>(
    handler: Handler,
    address: LocalSocketAddress,
) -> io::Result<LocalSocketServerHandle>
where
    Handler: TransportRequestHandler + TransportSubscriptionHandler + Send + Sync + 'static,
{
    let listener = create_listener(&address)?;
    let handler = Arc::new(handler);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let (connection_shutdown_tx, connection_shutdown_rx) = watch::channel(false);

    let task = tokio::spawn(async move {
        let mut connection_tasks = JoinSet::new();

        let result = loop {
            tokio::select! {
                _ = &mut shutdown_rx => break Ok(()),
                accept_result = listener.accept() => {
                    let stream = accept_result?;
                    let handler = Arc::clone(&handler);
                    let connection_shutdown_rx = connection_shutdown_rx.clone();

                    connection_tasks.spawn(async move {
                        let _ = handle_connection(handler, stream, connection_shutdown_rx).await;
                    });
                }
                Some(join_result) = connection_tasks.join_next(), if !connection_tasks.is_empty() => {
                    if let Err(error) = join_result
                        && !error.is_cancelled()
                    {
                        break Err(join_error_to_io(error));
                    }
                }
            }
        };

        let _ = connection_shutdown_tx.send(true);
        drain_or_abort_connection_tasks(&mut connection_tasks).await?;

        result
    });

    Ok(LocalSocketServerHandle { address, shutdown_tx: Some(shutdown_tx), task })
}
