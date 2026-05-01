use super::super::*;

impl TerminalPersistenceV2 {
    pub fn delivery_replay_window(
        &self,
        input: DeliveryOffsetInput,
    ) -> Result<DeliveryReplayWindow, TerminalPersistenceV2Error> {
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let mut connection = self.connection()?;
        let persisted = load_persisted_event_high_water(
            &mut connection,
            &input.session_id,
            &input.pane_id,
            &stream_id,
        )?;
        let offset = load_delivery_offset(
            &mut connection,
            &input.client_id,
            &input.session_id,
            &input.pane_id,
            &stream_id,
        )?;
        let from_event_seq = offset
            .as_ref()
            .and_then(|row| row.replay_from_event_seq)
            .or_else(|| {
                let acked = offset.as_ref().map_or(0, |row| row.last_acked_event_seq);
                (acked < persisted).then_some(acked.saturating_add(1))
            })
            .filter(|from| *from <= persisted);
        let gap_state = match from_event_seq {
            Some(from)
                if has_history_gap_in_range(
                    &mut connection,
                    &input.session_id,
                    &input.pane_id,
                    &stream_id,
                    from,
                    persisted,
                )? =>
            {
                "gap"
            }
            Some(_) => offset.as_ref().map_or("none", |row| row.gap_state.as_str()),
            None => "none",
        }
        .to_string();

        Ok(DeliveryReplayWindow { from_event_seq, to_event_seq: persisted, gap_state })
    }
}
