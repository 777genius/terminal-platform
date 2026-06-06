use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_delivery_progress(
        &self,
        input: DeliveryProgressInput,
    ) -> Result<DeliveryOffsetRecord, TerminalPersistenceV2Error> {
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        validate_non_negative_seq(input.last_sent_event_seq, "last sent event seq")?;
        validate_non_negative_seq(input.last_acked_event_seq, "last acked event seq")?;

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            touch_delivery_client(connection, &input.client_id, now)?;
            let persisted = load_persisted_event_high_water(
                connection,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?;
            let existing = load_delivery_offset(
                connection,
                &input.client_id,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?;
            let existing_sent = existing.as_ref().map_or(0, |row| row.last_sent_event_seq);
            let existing_acked = existing.as_ref().map_or(0, |row| row.last_acked_event_seq);
            let last_sent = input.last_sent_event_seq.unwrap_or(existing_sent).max(existing_sent);
            let last_acked =
                input.last_acked_event_seq.unwrap_or(existing_acked).max(existing_acked);

            if last_sent > persisted {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "last sent event seq {last_sent} is above persisted high-water {persisted}"
                )));
            }
            if last_acked > last_sent {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "last acked event seq {last_acked} is above last sent event seq {last_sent}"
                )));
            }

            let replay_from_event_seq =
                (last_acked < persisted).then_some(last_acked.saturating_add(1));
            let gap_state = match replay_from_event_seq {
                Some(from)
                    if has_history_gap_in_range(
                        connection,
                        &input.session_id,
                        &input.pane_id,
                        &stream_id,
                        from,
                        persisted,
                    )? =>
                {
                    "gap"
                }
                _ => "none",
            }
            .to_string();
            let row = NewDeliveryOffsetRow {
                id: delivery_offset_id(
                    &input.client_id,
                    &input.session_id,
                    &input.pane_id,
                    &stream_id,
                ),
                client_id: input.client_id.clone(),
                session_id: input.session_id.clone(),
                pane_id: Some(input.pane_id.clone()),
                stream_id: stream_id.clone(),
                last_sent_event_seq: last_sent,
                last_acked_event_seq: last_acked,
                last_persisted_event_seq: persisted,
                replay_from_event_seq,
                gap_state,
                updated_at_ms: now,
            };

            insert_into(terminal_delivery_offsets::table)
                .values(&row)
                .on_conflict(terminal_delivery_offsets::id)
                .do_update()
                .set((
                    terminal_delivery_offsets::last_sent_event_seq.eq(row.last_sent_event_seq),
                    terminal_delivery_offsets::last_acked_event_seq.eq(row.last_acked_event_seq),
                    terminal_delivery_offsets::last_persisted_event_seq
                        .eq(row.last_persisted_event_seq),
                    terminal_delivery_offsets::replay_from_event_seq.eq(row.replay_from_event_seq),
                    terminal_delivery_offsets::gap_state.eq(row.gap_state.clone()),
                    terminal_delivery_offsets::updated_at_ms.eq(row.updated_at_ms),
                ))
                .execute(connection)?;

            load_delivery_offset(
                connection,
                &input.client_id,
                &input.session_id,
                &input.pane_id,
                &stream_id,
            )?
            .map(Into::into)
            .ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "delivery offset upsert did not return a row".to_string(),
                )
            })
        })
    }
}
