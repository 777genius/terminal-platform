use std::{io, time::Duration};

use tokio::{
    task::{JoinError, JoinSet},
    time::timeout,
};

const CONNECTION_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) async fn drain_or_abort_connection_tasks(
    connection_tasks: &mut JoinSet<()>,
) -> io::Result<()> {
    if connection_tasks.is_empty() {
        return Ok(());
    }

    match timeout(CONNECTION_DRAIN_TIMEOUT, drain_connection_tasks(connection_tasks)).await {
        Ok(result) => result,
        Err(_) => {
            connection_tasks.abort_all();
            drain_connection_tasks(connection_tasks).await
        }
    }
}

async fn drain_connection_tasks(connection_tasks: &mut JoinSet<()>) -> io::Result<()> {
    while let Some(join_result) = connection_tasks.join_next().await {
        if let Err(error) = join_result
            && !error.is_cancelled()
        {
            return Err(join_error_to_io(error));
        }
    }

    Ok(())
}

pub(super) fn join_error_to_io(error: JoinError) -> io::Error {
    io::Error::other(error.to_string())
}
