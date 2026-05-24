use terminal_backend_api::BackendError;
use terminal_persistence::{
    PersistenceFaultHealthRecordInput, SavedNativeSession, SqliteSessionStore,
};

pub(super) trait SavedSessionPersistencePort {
    fn persist_native_v2_evidence(&self, snapshot: &SavedNativeSession)
    -> Result<(), BackendError>;

    fn publish_native_legacy_snapshot(
        &self,
        snapshot: &SavedNativeSession,
    ) -> Result<(), BackendError>;

    fn record_native_publish_failure_after_v2_evidence(
        &self,
        snapshot: &SavedNativeSession,
        error: &BackendError,
    ) -> Result<(), BackendError>;
}

impl SavedSessionPersistencePort for SqliteSessionStore {
    fn persist_native_v2_evidence(
        &self,
        snapshot: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        self.save_native_session_v2_snapshot(snapshot).map(|_restore_plan| ()).map_err(|error| {
            BackendError::internal(format!(
                "failed to persist native session v2 snapshot - {error}"
            ))
        })
    }

    fn publish_native_legacy_snapshot(
        &self,
        snapshot: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        self.save_native_session(snapshot).map_err(|error| {
            BackendError::internal(format!("failed to publish saved native session - {error}"))
        })
    }

    fn record_native_publish_failure_after_v2_evidence(
        &self,
        snapshot: &SavedNativeSession,
        error: &BackendError,
    ) -> Result<(), BackendError> {
        self.record_v2_persistence_fault_health_record(PersistenceFaultHealthRecordInput {
            session_id: Some(snapshot.session_id.0.to_string()),
            pane_id: None,
            operation: "saved_session_legacy_publish_after_v2_evidence".to_string(),
            detail: error.message.clone(),
            error_kind: Some("legacy_publish_failed".to_string()),
            metadata: Some(serde_json::json!({
                "save_semantics": "v2_evidence_persisted_legacy_publish_failed",
                "legacy_visible": false,
                "backend": "native",
            })),
        })
        .map(|_record_id| ())
        .map_err(|record_error| {
            BackendError::internal(format!(
                "failed to record saved-session publish failure after v2 evidence - {record_error}; original publish error - {}",
                error.message
            ))
        })
    }
}

pub(super) struct SavedSessionSaveOrchestrator<'a, P: SavedSessionPersistencePort + ?Sized> {
    persistence: &'a P,
}

impl<'a, P: SavedSessionPersistencePort + ?Sized> SavedSessionSaveOrchestrator<'a, P> {
    pub(super) fn new(persistence: &'a P) -> Self {
        Self { persistence }
    }

    pub(super) fn save_native(&self, snapshot: SavedNativeSession) -> Result<(), BackendError> {
        self.persistence.persist_native_v2_evidence(&snapshot)?;
        match self.persistence.publish_native_legacy_snapshot(&snapshot) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.persistence
                    .record_native_publish_failure_after_v2_evidence(&snapshot, &error)?;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use terminal_domain::{
        BackendKind, PaneId, RouteAuthority, SavedSessionManifest, SessionId, SessionRoute, TabId,
    };
    use terminal_mux_domain::PaneTreeNode;
    use terminal_projection::{
        ProjectionSource, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
    };

    use super::*;

    #[derive(Default)]
    struct FakePersistence {
        calls: Mutex<Vec<&'static str>>,
        fail_v2: bool,
        fail_publish: bool,
        fail_publish_marker: bool,
    }

    impl SavedSessionPersistencePort for FakePersistence {
        fn persist_native_v2_evidence(
            &self,
            _snapshot: &SavedNativeSession,
        ) -> Result<(), BackendError> {
            self.calls.lock().expect("calls should lock").push("v2");
            if self.fail_v2 {
                return Err(BackendError::internal("v2 failed"));
            }
            Ok(())
        }

        fn publish_native_legacy_snapshot(
            &self,
            _snapshot: &SavedNativeSession,
        ) -> Result<(), BackendError> {
            self.calls.lock().expect("calls should lock").push("publish");
            if self.fail_publish {
                return Err(BackendError::internal("publish failed"));
            }
            Ok(())
        }

        fn record_native_publish_failure_after_v2_evidence(
            &self,
            _snapshot: &SavedNativeSession,
            error: &BackendError,
        ) -> Result<(), BackendError> {
            self.calls.lock().expect("calls should lock").push("mark_unpublished");
            if self.fail_publish_marker {
                return Err(BackendError::internal(format!(
                    "mark unpublished failed after {}",
                    error.message
                )));
            }
            Ok(())
        }
    }

    #[test]
    fn save_native_persists_v2_before_publish() {
        let persistence = FakePersistence::default();

        SavedSessionSaveOrchestrator::new(&persistence)
            .save_native(test_snapshot())
            .expect("save should succeed");

        assert_eq!(*persistence.calls.lock().expect("calls should lock"), vec!["v2", "publish"]);
    }

    #[test]
    fn save_native_does_not_publish_when_v2_fails() {
        let persistence = FakePersistence { fail_v2: true, ..FakePersistence::default() };

        let error = SavedSessionSaveOrchestrator::new(&persistence)
            .save_native(test_snapshot())
            .expect_err("v2 failure should fail save");

        assert!(error.message.contains("v2 failed"));
        assert_eq!(*persistence.calls.lock().expect("calls should lock"), vec!["v2"]);
    }

    #[test]
    fn save_native_surfaces_publish_failure_after_v2_evidence() {
        let persistence = FakePersistence { fail_publish: true, ..FakePersistence::default() };

        let error = SavedSessionSaveOrchestrator::new(&persistence)
            .save_native(test_snapshot())
            .expect_err("publish failure should fail save");

        assert!(error.message.contains("publish failed"));
        assert_eq!(
            *persistence.calls.lock().expect("calls should lock"),
            vec!["v2", "publish", "mark_unpublished"]
        );
    }

    #[test]
    fn save_native_reports_marker_failure_after_publish_failure() {
        let persistence = FakePersistence {
            fail_publish: true,
            fail_publish_marker: true,
            ..FakePersistence::default()
        };

        let error = SavedSessionSaveOrchestrator::new(&persistence)
            .save_native(test_snapshot())
            .expect_err("marker failure should fail save explicitly");

        assert!(error.message.contains("mark unpublished failed"));
        assert!(error.message.contains("publish failed"));
        assert_eq!(
            *persistence.calls.lock().expect("calls should lock"),
            vec!["v2", "publish", "mark_unpublished"]
        );
    }

    fn test_snapshot() -> SavedNativeSession {
        let session_id = SessionId::new();
        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        SavedNativeSession {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
            launch: None,
            manifest: SavedSessionManifest::current(),
            topology: TopologySnapshot {
                session_id,
                backend_kind: BackendKind::Native,
                tabs: vec![terminal_mux_domain::TabSnapshot {
                    tab_id,
                    title: Some("shell".to_string()),
                    root: PaneTreeNode::Leaf { pane_id },
                    focused_pane: Some(pane_id),
                }],
                focused_tab: Some(tab_id),
            },
            screens: vec![ScreenSnapshot {
                pane_id,
                sequence: 1,
                rows: 24,
                cols: 80,
                source: ProjectionSource::NativeEmulator,
                surface: ScreenSurface {
                    title: Some("shell".to_string()),
                    cursor: None,
                    lines: vec![ScreenLine { text: "ready".to_string() }],
                },
            }],
            saved_at_ms: 1_000,
        }
    }
}
