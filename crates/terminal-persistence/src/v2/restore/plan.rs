mod evidence;
mod guarantee;
mod metrics;

use super::super::*;

use evidence::build_restore_evidence;
use guarantee::choose_restore_guarantee;
use metrics::load_restore_plan_inputs;

impl TerminalPersistenceV2 {
    pub fn restore_plan(
        &self,
        session_id: &str,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.restore_plan_with_connection(&mut connection, session_id)
    }

    pub(crate) fn restore_plan_with_connection(
        &self,
        connection: &mut SqliteConnection,
        session_id: &str,
    ) -> Result<RestorePlan, TerminalPersistenceV2Error> {
        let now = self.config.clock.now_ms();
        let inputs = load_restore_plan_inputs(connection, session_id, now)?;
        let guarantee_level = choose_restore_guarantee(&inputs, now);
        let evidence = build_restore_evidence(session_id, &inputs, now);

        Ok(RestorePlan {
            session_id: session_id.to_string(),
            guarantee_level,
            latest_screen_snapshot_id: inputs.latest_screen.as_ref().map(|row| row.id.clone()),
            latest_topology_snapshot_id: inputs.latest_topology.as_ref().map(|row| row.id.clone()),
            high_water_commit_seq: inputs.high_water_commit_seq,
            latest_restore_drill_status: inputs.latest_restore_drill_status,
            evidence,
        })
    }
}
