use crate::v2::{
    BackendCapabilityReportInput, HistoryGapEventInput, PersistenceFaultHealthRecordInput,
    ScreenSnapshotEventInput, TerminalOutputEventInput, TerminalPersistenceV2Error,
    TopologySnapshotEventInput, UiInputEventInput,
};

use super::super::{SqliteSessionStore, retry::retry_v2_write};

impl SqliteSessionStore {
    pub fn record_v2_backend_capability_report(
        &self,
        input: BackendCapabilityReportInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_backend_capability_report(input)
            })
        })
    }

    pub fn record_v2_ui_input(
        &self,
        input: UiInputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| store.record_ui_input_event(input))
        })
    }

    pub fn record_v2_terminal_output(
        &self,
        input: TerminalOutputEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_terminal_output_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_history_gap(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_history_gap_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_screen_snapshot(
        &self,
        input: ScreenSnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_screen_snapshot_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_topology_snapshot(
        &self,
        input: TopologySnapshotEventInput,
    ) -> Result<(), TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_topology_snapshot_event(input)?;
                Ok(())
            })
        })
    }

    pub fn record_v2_persistence_fault_health_record(
        &self,
        input: PersistenceFaultHealthRecordInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        retry_v2_write(|| {
            let input = input.clone();
            self.with_v2_store_serialized(move |store| {
                store.record_persistence_fault_health_record(input)
            })
        })
    }
}
