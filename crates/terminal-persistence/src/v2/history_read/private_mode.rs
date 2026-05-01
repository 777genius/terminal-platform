use super::super::*;

impl TerminalPersistenceV2 {
    pub(in crate::v2) fn is_session_private(
        &self,
        session_id: &str,
    ) -> Result<bool, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        session_private_mode(&mut connection, session_id)
    }
}
