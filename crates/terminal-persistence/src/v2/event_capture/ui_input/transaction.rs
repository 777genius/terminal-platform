mod command_history;
mod context;
mod journal;
mod receipts;

use super::super::super::*;
use command_history::upsert_verified_command_history;
use context::UiInputTransaction;
use journal::{advance_ui_input_cursor, insert_ui_input_journal_event};
use receipts::{insert_ui_input_capture_receipt, reuse_ui_input_receipt_if_possible};

impl TerminalPersistenceV2 {
    pub(in crate::v2) fn append_ui_input_event_and_command(
        &self,
        input: &UiInputEventInput,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let tx = UiInputTransaction::new(input, now)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            if reuse_ui_input_receipt_if_possible(connection, &tx)? {
                return Ok(());
            }

            ensure_active_writer(connection, writer_generation, now)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                "ui_input",
                writer_generation,
                now,
                now,
                None,
            )?;
            let cursor =
                load_stream_cursor(connection, &input.session_id, &input.pane_id, &tx.stream_id)?;
            let event_seq = cursor.next_event_seq;
            insert_ui_input_journal_event(connection, &tx, &commit.id, event_seq)?;
            advance_ui_input_cursor(
                connection,
                &cursor.id,
                &input.pane_id,
                cursor.next_event_seq + 1,
                cursor.next_byte_seq,
                event_seq,
                now,
            )?;
            upsert_verified_command_history(connection, &tx, &commit.id, event_seq)?;
            insert_ui_input_capture_receipt(connection, &tx, &commit.id)?;
            Ok(())
        })
    }
}
