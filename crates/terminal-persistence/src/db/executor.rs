use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread::{self, JoinHandle},
};

use diesel::sqlite::SqliteConnection;

use crate::{
    db::connection::establish_initialized_connection,
    v2::{TerminalPersistenceV2Config, TerminalPersistenceV2Error},
};

type PersistenceJob = Box<dyn FnOnce(&mut SqliteConnection) + Send + 'static>;

enum ExecutorMessage {
    Job(PersistenceJob),
    Shutdown,
}

pub struct PersistenceExecutor {
    sender: mpsc::Sender<ExecutorMessage>,
    join: Option<JoinHandle<()>>,
}

impl PersistenceExecutor {
    pub fn start(
        path: impl Into<PathBuf>,
        config: TerminalPersistenceV2Config,
    ) -> Result<Self, TerminalPersistenceV2Error> {
        let path = path.into();
        let (sender, receiver) = mpsc::channel::<ExecutorMessage>();
        let (ready_sender, ready_receiver) =
            mpsc::sync_channel::<Result<(), TerminalPersistenceV2Error>>(1);
        let worker_path = path.clone();
        let worker_config = config.clone();

        let join = thread::Builder::new().name(worker_name(&path)).spawn(move || {
            let mut connection =
                match establish_initialized_connection(&worker_path, &worker_config) {
                    Ok(connection) => {
                        let _ = ready_sender.send(Ok(()));
                        connection
                    }
                    Err(error) => {
                        let _ = ready_sender.send(Err(error));
                        return;
                    }
                };

            while let Ok(message) = receiver.recv() {
                match message {
                    ExecutorMessage::Job(job) => job(&mut connection),
                    ExecutorMessage::Shutdown => break,
                }
            }
        })?;

        match ready_receiver.recv().map_err(|_| TerminalPersistenceV2Error::ExecutorStopped)? {
            Ok(()) => Ok(Self { sender, join: Some(join) }),
            Err(error) => {
                let _ = join.join();
                Err(error)
            }
        }
    }

    pub fn execute<T>(
        &self,
        operation: impl FnOnce() -> Result<T, TerminalPersistenceV2Error> + Send + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        self.sender
            .send(ExecutorMessage::Job(Box::new(move |_connection| {
                let _ = reply_sender.send(operation());
            })))
            .map_err(|_| TerminalPersistenceV2Error::ExecutorStopped)?;
        reply_receiver.recv().map_err(|_| TerminalPersistenceV2Error::ExecutorStopped)?
    }

    pub fn execute_with_connection<T>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T, TerminalPersistenceV2Error>
        + Send
        + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        self.sender
            .send(ExecutorMessage::Job(Box::new(move |connection| {
                let _ = reply_sender.send(operation(connection));
            })))
            .map_err(|_| TerminalPersistenceV2Error::ExecutorStopped)?;
        reply_receiver.recv().map_err(|_| TerminalPersistenceV2Error::ExecutorStopped)?
    }

    pub fn immediate_transaction<T>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T, TerminalPersistenceV2Error>
        + Send
        + 'static,
    ) -> Result<T, TerminalPersistenceV2Error>
    where
        T: Send + 'static,
    {
        self.execute_with_connection(move |connection| {
            connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(operation)
        })
    }
}

impl Drop for PersistenceExecutor {
    fn drop(&mut self) {
        let _ = self.sender.send(ExecutorMessage::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn worker_name(path: &Path) -> String {
    let file = path.file_name().and_then(|value| value.to_str()).unwrap_or("history.sqlite3");
    format!("terminal-persistence-v2-writer-{file}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{RunQueryDsl, sql_query};
    use uuid::Uuid;

    #[derive(diesel::QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    #[test]
    fn executor_serializes_jobs_and_supports_transactions() {
        let path = std::env::temp_dir()
            .join(format!("terminal-persistence-executor-{}.sqlite3", Uuid::new_v4()));
        let executor =
            PersistenceExecutor::start(&path, TerminalPersistenceV2Config::test()).unwrap();

        executor
            .immediate_transaction(|connection| {
                sql_query(
                    "CREATE TABLE executor_probe (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
                )
                .execute(connection)?;
                sql_query("INSERT INTO executor_probe (value) VALUES ('ready')")
                    .execute(connection)?;
                Ok(())
            })
            .unwrap();

        let value = executor.execute(|| Ok(41_i64)).unwrap();
        let count = executor
            .immediate_transaction(|connection| {
                let row = sql_query("SELECT COUNT(*) AS count FROM executor_probe")
                    .get_result::<CountRow>(connection)?;
                Ok(row.count)
            })
            .unwrap();

        assert_eq!(value, 41);
        assert_eq!(count, 1);
        drop(executor);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn executor_reuses_worker_connection_for_connection_jobs() {
        let path = std::env::temp_dir()
            .join(format!("terminal-persistence-executor-{}.sqlite3", Uuid::new_v4()));
        let executor =
            PersistenceExecutor::start(&path, TerminalPersistenceV2Config::test()).unwrap();

        executor
            .execute_with_connection(|connection| {
                sql_query("CREATE TEMP TABLE worker_connection_probe (value TEXT NOT NULL)")
                    .execute(connection)?;
                sql_query("INSERT INTO worker_connection_probe (value) VALUES ('same')")
                    .execute(connection)?;
                Ok(())
            })
            .unwrap();

        let count = executor
            .execute_with_connection(|connection| {
                let row = sql_query("SELECT COUNT(*) AS count FROM worker_connection_probe")
                    .get_result::<CountRow>(connection)?;
                Ok(row.count)
            })
            .unwrap();

        assert_eq!(count, 1);
        drop(executor);
        let _ = std::fs::remove_file(path);
    }
}
