use super::super::*;

impl TerminalPersistenceV2 {
    pub(crate) fn is_session_private_with_connection(
        &self,
        connection: &mut SqliteConnection,
        session_id: &str,
    ) -> Result<bool, TerminalPersistenceV2Error> {
        session_private_mode(connection, session_id)
    }
}
