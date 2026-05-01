use super::super::*;

impl TerminalPersistenceV2 {
    pub fn restore_plan(
        &self,
        session_id: &str,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let latest_topology =
            load_latest_valid_topology_snapshot(&mut connection, session_id, now, "restore_plan")?;
        let topology_pane_high_water = latest_topology
            .as_ref()
            .map(|topology| parse_pane_high_water_json(&topology.pane_high_water_json))
            .transpose()?;
        let latest_screen = load_latest_valid_screen_snapshot(
            &mut connection,
            session_id,
            None,
            topology_pane_high_water.as_ref(),
            now,
            "restore_plan",
        )?;
        let segment_count: i64 = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .count()
            .get_result(&mut connection)?;
        let raw_segment_count: i64 = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .filter(terminal_stream_segments::capture_semantics.eq("raw_vt_stream"))
            .count()
            .get_result(&mut connection)?;
        let rendered_segment_count = segment_count - raw_segment_count;
        let stream_event_range = terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .select((
                diesel::dsl::min(terminal_stream_segments::event_seq_low),
                diesel::dsl::max(terminal_stream_segments::event_seq_high),
            ))
            .first::<(Option<i64>, Option<i64>)>(&mut connection)?;
        let persisted_gap_count: i64 = terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(session_id))
            .count()
            .get_result(&mut connection)?;
        let journal_gap_count: i64 = terminal_journal_events::table
            .filter(terminal_journal_events::session_id.eq(session_id))
            .filter(terminal_journal_events::event_type.eq("history_gap"))
            .count()
            .get_result(&mut connection)?;
        let gap_count = persisted_gap_count.max(journal_gap_count);
        let high_water_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(session_id))
            .select(diesel::dsl::max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(&mut connection)?
            .unwrap_or(0);
        let latest_restore_drill = terminal_restore_drills::table
            .filter(terminal_restore_drills::session_id.eq(session_id))
            .order(terminal_restore_drills::checked_at_ms.desc())
            .select((terminal_restore_drills::id, terminal_restore_drills::result))
            .first::<(String, String)>(&mut connection)
            .optional()?;
        let latest_restore_drill_status =
            latest_restore_drill.as_ref().map(|(_, result)| result.clone());
        let authoritative_reads_gate = terminal_feature_gates::table
            .filter(
                terminal_feature_gates::feature_name
                    .eq(FeatureGateName::TerminalPersistenceV2AuthoritativeReads.as_str()),
            )
            .select(terminal_feature_gates::state)
            .first::<String>(&mut connection)
            .optional()?
            .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
        let latest_capability_report =
            latest_backend_capability_report(&mut connection, session_id)?;
        let capability_stale = latest_capability_report.as_ref().map(|report| {
            report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed"
        });
        let has_fresh_raw_capability = latest_capability_report.as_ref().is_some_and(|report| {
            capability_stale == Some(false) && report.capture_semantics == "raw_vt_stream"
        });
        let critical_health_record_count: i64 = terminal_data_health_records::table
            .filter(
                terminal_data_health_records::session_id
                    .eq(Some(session_id.to_string()))
                    .or(terminal_data_health_records::session_id.is_null()),
            )
            .filter(terminal_data_health_records::severity.eq("critical"))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .count()
            .get_result(&mut connection)?;

        let mut guarantee_level = match (
            segment_count > 0,
            raw_segment_count > 0,
            latest_screen.is_some(),
            latest_topology.is_some(),
            gap_count > 0,
        ) {
            (_, _, _, _, true) => RestoreGuaranteeLevel::DegradedHistory,
            (true, true, true, true, false)
                if latest_restore_drill_status.as_deref() == Some("passed")
                    && has_fresh_raw_capability =>
            {
                RestoreGuaranteeLevel::RawStreamReplay
            }
            (true, _, true, _, false) => RestoreGuaranteeLevel::BasicHistory,
            (false, _, true, _, false) => RestoreGuaranteeLevel::VisualSnapshotOnly,
            _ => RestoreGuaranteeLevel::None,
        };
        if matches!(latest_restore_drill_status.as_deref(), Some("failed" | "degraded")) {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if authoritative_reads_gate == FeatureGateState::ForceDisabled.as_str() {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if critical_health_record_count > 0 {
            guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if let Some(report) = latest_capability_report.as_ref() {
            let stale = report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed";
            if stale {
                guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
            }
            if report.capture_semantics != "raw_vt_stream"
                && matches!(guarantee_level, RestoreGuaranteeLevel::RawStreamReplay)
            {
                guarantee_level = RestoreGuaranteeLevel::BasicHistory;
            }
        }

        let mut evidence = vec![
            RestoreEvidence {
                kind: "stream_segment_count".to_string(),
                value: segment_count.to_string(),
            },
            RestoreEvidence {
                kind: "raw_stream_segment_count".to_string(),
                value: raw_segment_count.to_string(),
            },
            RestoreEvidence {
                kind: "rendered_stream_segment_count".to_string(),
                value: rendered_segment_count.to_string(),
            },
            RestoreEvidence { kind: "history_gap_count".to_string(), value: gap_count.to_string() },
            RestoreEvidence {
                kind: "authoritative_reads_gate_state".to_string(),
                value: authoritative_reads_gate,
            },
            RestoreEvidence {
                kind: "critical_data_health_record_count".to_string(),
                value: critical_health_record_count.to_string(),
            },
        ];
        if let (Some(event_seq_low), Some(event_seq_high)) = stream_event_range {
            evidence.push(RestoreEvidence {
                kind: "journal_event_range".to_string(),
                value: format!("{session_id}:{event_seq_low}:{event_seq_high}"),
            });
        }
        if let Some(screen) = latest_screen.as_ref() {
            evidence.push(RestoreEvidence {
                kind: "screen_snapshot".to_string(),
                value: screen.id.clone(),
            });
        }
        if let Some(topology) = latest_topology.as_ref() {
            evidence.push(RestoreEvidence {
                kind: "topology_snapshot".to_string(),
                value: topology.id.clone(),
            });
        }
        if let Some(status) = &latest_restore_drill_status {
            evidence.push(RestoreEvidence {
                kind: "latest_restore_drill_status".to_string(),
                value: status.clone(),
            });
        }
        if let Some((drill_id, _)) = &latest_restore_drill {
            evidence.push(RestoreEvidence {
                kind: "restore_drill".to_string(),
                value: drill_id.clone(),
            });
        }
        if let Some(report) = latest_capability_report {
            let stale = report.expires_at_ms <= now
                || report.stale_reason.is_some()
                || report.probe_status != "passed";
            evidence.push(RestoreEvidence {
                kind: "backend_capability_report".to_string(),
                value: report.id.clone(),
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capability_probe_status".to_string(),
                value: report.probe_status,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capture_strategy".to_string(),
                value: report.capture_strategy,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capture_semantics".to_string(),
                value: report.capture_semantics,
            });
            evidence.push(RestoreEvidence {
                kind: "backend_capability_stale".to_string(),
                value: stale.to_string(),
            });
            if let Some(reason) = report.stale_reason {
                evidence.push(RestoreEvidence {
                    kind: "backend_capability_stale_reason".to_string(),
                    value: reason,
                });
            }
        }

        Ok(RestorePlan {
            session_id: session_id.to_string(),
            guarantee_level,
            latest_screen_snapshot_id: latest_screen.as_ref().map(|row| row.id.clone()),
            latest_topology_snapshot_id: latest_topology.as_ref().map(|row| row.id.clone()),
            high_water_commit_seq,
            latest_restore_drill_status,
            evidence,
        })
    }
}
