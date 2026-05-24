use std::{sync::Arc, thread};

use diesel::{RunQueryDsl, sql_query};
use rusqlite::{Connection, params};
use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{
    BackendKind, CURRENT_BINARY_VERSION, PaneId, RouteAuthority, SavedSessionManifest, SessionId,
    SessionRoute, TabId,
};
use terminal_projection::{
    ProjectionSource, ScreenLine, ScreenSnapshot, ScreenSurface, TopologySnapshot,
};

use crate::v2::{
    HistoryGapEventInput, PersistenceFaultHealthRecordInput, RestoreGuaranteeLevel,
    ScreenSnapshotEventInput, TerminalOutputEventInput, TopologySnapshotEventInput,
    UiInputEventInput,
};

use super::{PersistenceError, SavedNativeSession, SessionRouteRecord, SqliteSessionStore};

fn sample_snapshot(session_id: SessionId, title: &str, line: &str) -> SavedNativeSession {
    SavedNativeSession {
        session_id,
        route: SessionRoute {
            backend: BackendKind::Native,
            authority: RouteAuthority::LocalDaemon,
            external: None,
        },
        title: Some(title.to_string()),
        launch: Some(ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "exec cat"])),
        manifest: SavedSessionManifest::current(),
        topology: TopologySnapshot {
            session_id,
            backend_kind: BackendKind::Native,
            focused_tab: Some(TabId::new()),
            tabs: Vec::new(),
        },
        screens: vec![ScreenSnapshot {
            pane_id: PaneId::new(),
            sequence: 1,
            rows: 24,
            cols: 80,
            source: ProjectionSource::NativeEmulator,
            surface: ScreenSurface {
                title: Some(title.to_string()),
                cursor: None,
                lines: vec![ScreenLine { text: line.to_string() }],
            },
        }],
        saved_at_ms: SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve"),
    }
}

#[test]
fn saves_and_loads_native_session_snapshot() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path = std::env::temp_dir().join(format!("terminal-platform-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "shell", "ready");

    store.save_native_session(&snapshot).expect("save should succeed");
    let loaded = store
        .load_native_session(session_id)
        .expect("load should succeed")
        .expect("saved session should exist");

    assert_eq!(loaded, snapshot);
    assert_eq!(loaded.manifest.format_version, 1);

    let _ = std::fs::remove_file(path);
}

#[test]
fn upserts_existing_native_session_snapshot() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path = std::env::temp_dir().join(format!("terminal-platform-upsert-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let session_id = SessionId::new();
    let first = sample_snapshot(session_id, "shell", "ready");
    let second = sample_snapshot(session_id, "shell-renamed", "ready again");

    store.save_native_session(&first).expect("first save should succeed");
    store.save_native_session(&second).expect("second save should succeed");

    let loaded = store
        .load_native_session(session_id)
        .expect("load should succeed")
        .expect("saved session should exist");

    assert_eq!(loaded.title.as_deref(), Some("shell-renamed"));
    assert_eq!(
        loaded.screens[0].surface.lines.first().map(|line| line.text.as_str()),
        Some("ready again")
    );
    assert!(loaded.saved_at_ms >= first.saved_at_ms);

    let _ = std::fs::remove_file(path);
}

#[test]
fn deletes_saved_native_session_snapshot() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path = std::env::temp_dir().join(format!("terminal-platform-delete-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "shell", "ready");

    store.save_native_session(&snapshot).expect("save should succeed");

    assert!(store.delete_native_session(session_id).expect("delete should succeed"));
    assert!(store.load_native_session(session_id).expect("load should succeed").is_none());
    assert!(!store.delete_native_session(session_id).expect("delete should succeed"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn prunes_saved_native_sessions_to_latest_count() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path = std::env::temp_dir().join(format!("terminal-platform-prune-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let oldest_session = SessionId::new();
    let middle_session = SessionId::new();
    let newest_session = SessionId::new();
    let oldest = sample_snapshot(oldest_session, "older", "first");
    let mut middle = sample_snapshot(middle_session, "middle", "second");
    middle.saved_at_ms = oldest.saved_at_ms + 1;
    let mut newest = sample_snapshot(newest_session, "newest", "third");
    newest.saved_at_ms = oldest.saved_at_ms + 2;

    store.save_native_session(&oldest).expect("oldest save should succeed");
    store.save_native_session(&middle).expect("middle save should succeed");
    store.save_native_session(&newest).expect("newest save should succeed");

    let pruned = store.prune_native_sessions(1).expect("prune should succeed");
    let listed = store.list_native_sessions().expect("list should succeed");

    assert_eq!(pruned.deleted_count, 2);
    assert_eq!(pruned.kept_count, 1);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].session_id, newest_session);

    let _ = std::fs::remove_file(path);
}

#[test]
fn lists_saved_native_sessions_in_descending_timestamp_order() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path = std::env::temp_dir().join(format!("terminal-platform-list-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let older_session = SessionId::new();
    let newer_session = SessionId::new();
    let older = sample_snapshot(older_session, "older", "first");
    let mut newer = sample_snapshot(newer_session, "newer", "second");
    newer.saved_at_ms = older.saved_at_ms + 1;

    store.save_native_session(&older).expect("older save should succeed");
    store.save_native_session(&newer).expect("newer save should succeed");

    let listed = store.list_native_sessions().expect("list should succeed");

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].session_id, newer_session);
    assert_eq!(listed[0].title.as_deref(), Some("newer"));
    assert_eq!(listed[0].tab_count, 0);
    assert_eq!(listed[0].pane_count, 0);
    assert!(listed[0].has_launch);
    assert_eq!(listed[0].manifest.format_version, 1);
    assert_eq!(listed[1].session_id, older_session);
    assert_eq!(listed[1].manifest.format_version, 1);

    let _ = std::fs::remove_file(path);
}

#[test]
fn lists_saved_native_sessions_ignores_corrupted_rows() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-corrupt-list-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let valid_session_id = SessionId::new();
    let corrupt_session_id = SessionId::new();
    let snapshot = sample_snapshot(valid_session_id, "shell", "ready");

    store.save_native_session(&snapshot).expect("valid save should succeed");

    let connection = Connection::open(&path).expect("raw sqlite should open");
    connection
        .execute(
            "
                INSERT INTO native_saved_sessions (
                    session_id,
                    route_json,
                    title,
                    launch_json,
                    manifest_json,
                    topology_json,
                    screens_json,
                    saved_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
            params![
                corrupt_session_id.0.to_string(),
                serde_json::to_string(&snapshot.route).expect("route json should serialize"),
                "corrupt",
                serde_json::to_string(&snapshot.launch).expect("launch json should serialize"),
                serde_json::to_string(&snapshot.manifest).expect("manifest json should serialize"),
                "{not-json",
                serde_json::to_string(&snapshot.screens).expect("screens json should serialize"),
                snapshot.saved_at_ms + 1,
            ],
        )
        .expect("corrupted row should insert");

    let listed = store.list_native_sessions().expect("list should succeed");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].session_id, valid_session_id);
    assert_eq!(listed[0].title.as_deref(), Some("shell"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn load_native_session_reports_corrupted_row_for_targeted_lookup() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-corrupt-load-test-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "shell", "ready");

    let connection = Connection::open(&path).expect("raw sqlite should open");
    connection
        .execute(
            "
                INSERT INTO native_saved_sessions (
                    session_id,
                    route_json,
                    title,
                    launch_json,
                    manifest_json,
                    topology_json,
                    screens_json,
                    saved_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
            params![
                session_id.0.to_string(),
                serde_json::to_string(&snapshot.route).expect("route json should serialize"),
                "corrupt",
                serde_json::to_string(&snapshot.launch).expect("launch json should serialize"),
                serde_json::to_string(&snapshot.manifest).expect("manifest json should serialize"),
                "{not-json",
                serde_json::to_string(&snapshot.screens).expect("screens json should serialize"),
                snapshot.saved_at_ms,
            ],
        )
        .expect("corrupted row should insert");

    let error = store
        .load_native_session(session_id)
        .expect_err("targeted lookup should fail for corrupted row");

    assert!(matches!(error, PersistenceError::Serde(_) | PersistenceError::InvalidData(_)));

    let _ = std::fs::remove_file(path);
}

#[test]
fn upgrades_legacy_saved_session_schema_with_manifest_column() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-legacy-schema-{nonce}.sqlite3"));
    let connection = Connection::open(&path).expect("legacy db should open");
    connection
        .execute_batch(
            "
                CREATE TABLE native_saved_sessions (
                    session_id TEXT PRIMARY KEY,
                    route_json TEXT NOT NULL,
                    title TEXT,
                    launch_json TEXT,
                    topology_json TEXT NOT NULL,
                    screens_json TEXT NOT NULL,
                    saved_at_ms INTEGER NOT NULL
                );
                ",
        )
        .expect("legacy schema should be created");
    drop(connection);

    let store = SqliteSessionStore::open(&path).expect("store should upgrade legacy schema");
    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "shell", "ready");

    store.save_native_session(&snapshot).expect("save should succeed after upgrade");
    let loaded = store
        .load_native_session(session_id)
        .expect("load should succeed")
        .expect("saved session should exist");

    assert_eq!(loaded.manifest.format_version, 1);
    assert_eq!(loaded.manifest.binary_version, CURRENT_BINARY_VERSION);

    let _ = std::fs::remove_file(path);
}

#[test]
fn saves_and_loads_session_route_registry_records() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-route-registry-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let record = SessionRouteRecord {
        session_id: SessionId::new(),
        route: SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "tmux_session".to_string(),
                value: "demo".to_string(),
            }),
        },
        route_fingerprint: "tmux/import/demo".to_string(),
    };

    store.upsert_session_route(&record).expect("route record should save");

    assert_eq!(
        store.load_session_route(record.session_id).expect("lookup by id should succeed"),
        Some(record.clone())
    );
    assert_eq!(
        store
            .load_session_route_by_fingerprint(&record.route_fingerprint)
            .expect("lookup by fingerprint should succeed"),
        Some(record)
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn saves_native_session_snapshot_into_v2_visual_history() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-v2-visual-save-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "shell", "visible history");

    let plan =
        store.save_native_session_v2_snapshot(&snapshot).expect("v2 visual snapshot should save");

    assert_eq!(plan.session_id, session_id.0.to_string());
    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::VisualSnapshotOnly);
    assert!(plan.latest_screen_snapshot_id.is_some());
    assert!(plan.latest_topology_snapshot_id.is_some());
    assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("passed"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn v2_facade_serializes_concurrent_output_capture() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-v2-serialized-{nonce}.sqlite3"));
    let store = Arc::new(SqliteSessionStore::open(&path).expect("store should open"));
    let session_id = SessionId::new();
    let pane_id = PaneId::new();
    let route = SessionRoute {
        backend: BackendKind::Native,
        authority: RouteAuthority::LocalDaemon,
        external: None,
    };

    let mut handles = Vec::new();
    for index in 0..12 {
        let store = Arc::clone(&store);
        let route = route.clone();
        let input = TerminalOutputEventInput {
            session_id: session_id.0.to_string(),
            route,
            title: Some("serialized shell".to_string()),
            launch: None,
            pane_id: pane_id.0.to_string(),
            tab_id: None,
            payload: format!("serialized-line-{index}\n").into_bytes(),
            rows: Some(24),
            cols: Some(80),
            source_sequence: Some(index),
            occurred_at_ms: None,
            capture_semantics: Some("raw_vt_stream".to_string()),
        };
        handles.push(thread::spawn(move || {
            store
                .record_v2_terminal_output(input)
                .expect("serialized v2 output capture should persist");
        }));
    }

    for handle in handles {
        handle.join().expect("capture thread should finish");
    }

    let history = store
        .hydrate_v2_pane_history(
            &session_id.0.to_string(),
            &pane_id.0.to_string(),
            Some(1),
            Some(32),
            Some(16 * 1024),
        )
        .expect("history should hydrate after concurrent capture");

    assert_eq!(history.segments.len(), 12);
    let mut expected_event_seq = 1;
    let mut payload_text = String::new();
    for segment in &history.segments {
        assert_eq!(segment.event_seq_low, expected_event_seq);
        assert_eq!(segment.event_seq_high, expected_event_seq);
        expected_event_seq += 1;
        payload_text.push_str(&String::from_utf8_lossy(&segment.payload));
    }
    for index in 0..12 {
        assert!(payload_text.contains(&format!("serialized-line-{index}")));
    }

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[derive(diesel::QueryableByName)]
struct WorkerProbeCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[test]
fn v2_facade_fault_health_write_uses_worker_connection() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-v2-worker-facade-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let executor = store.v2_executor().expect("v2 executor should start");
    executor
        .execute_with_connection(|connection| {
            sql_query("CREATE TEMP TABLE worker_fault_probe (value TEXT)").execute(connection)?;
            sql_query(
                "
                CREATE TEMP TRIGGER worker_fault_probe_insert
                AFTER INSERT ON terminal_data_health_records
                BEGIN
                    INSERT INTO worker_fault_probe (value) VALUES (NEW.affected_ref);
                END
                ",
            )
            .execute(connection)?;
            Ok(())
        })
        .expect("worker temp trigger should install");

    store
        .record_v2_persistence_fault_health_record(PersistenceFaultHealthRecordInput {
            session_id: None,
            pane_id: None,
            operation: "worker_connection_probe".to_string(),
            detail: "probe fault".to_string(),
            error_kind: Some("probe".to_string()),
            metadata: None,
        })
        .expect("fault health record should persist through worker connection");

    let count = executor
        .execute_with_connection(|connection| {
            let row = sql_query("SELECT COUNT(*) AS count FROM worker_fault_probe")
                .get_result::<WorkerProbeCount>(connection)?;
            Ok(row.count)
        })
        .expect("worker temp trigger count should load");

    assert_eq!(count, 1);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn v2_facade_saved_session_import_uses_worker_connection() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-v2-worker-save-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let executor = store.v2_executor().expect("v2 executor should start");
    executor
        .execute_with_connection(|connection| {
            sql_query("CREATE TEMP TABLE worker_saved_session_probe (kind TEXT)")
                .execute(connection)?;
            sql_query(
                "
                CREATE TEMP TRIGGER worker_saved_session_probe_screen
                AFTER INSERT ON terminal_screen_snapshots
                BEGIN
                    INSERT INTO worker_saved_session_probe (kind) VALUES ('screen');
                END
                ",
            )
            .execute(connection)?;
            sql_query(
                "
                CREATE TEMP TRIGGER worker_saved_session_probe_drill
                AFTER INSERT ON terminal_restore_drills
                BEGIN
                    INSERT INTO worker_saved_session_probe (kind) VALUES ('drill');
                END
                ",
            )
            .execute(connection)?;
            Ok(())
        })
        .expect("worker temp triggers should install");

    let session_id = SessionId::new();
    let snapshot = sample_snapshot(session_id, "worker-save-shell", "worker save history");
    store
        .save_native_session_v2_snapshot(&snapshot)
        .expect("v2 saved session snapshot should persist through worker connection");

    let screen_count = executor
        .execute_with_connection(|connection| {
            let row = sql_query(
                "SELECT COUNT(*) AS count FROM worker_saved_session_probe WHERE kind = 'screen'",
            )
            .get_result::<WorkerProbeCount>(connection)?;
            Ok(row.count)
        })
        .expect("worker screen temp trigger count should load");
    let drill_count = executor
        .execute_with_connection(|connection| {
            let row = sql_query(
                "SELECT COUNT(*) AS count FROM worker_saved_session_probe WHERE kind = 'drill'",
            )
            .get_result::<WorkerProbeCount>(connection)?;
            Ok(row.count)
        })
        .expect("worker drill temp trigger count should load");

    assert_eq!(screen_count, 1);
    assert_eq!(drill_count, 1);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn v2_facade_hot_capture_paths_use_worker_connection() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-v2-worker-capture-{nonce}.sqlite3"));
    let store = SqliteSessionStore::open(&path).expect("store should open");
    let executor = store.v2_executor().expect("v2 executor should start");
    executor
        .execute_with_connection(|connection| {
            sql_query("CREATE TEMP TABLE worker_capture_probe (kind TEXT)").execute(connection)?;
            for (trigger_name, table_name, kind) in [
                ("worker_capture_probe_ui", "terminal_journal_events", "ui_input"),
                ("worker_capture_probe_output", "terminal_stream_segments", "output"),
                ("worker_capture_probe_gap", "terminal_history_gaps", "gap"),
                ("worker_capture_probe_screen", "terminal_screen_snapshots", "screen"),
                ("worker_capture_probe_topology", "terminal_topology_snapshots", "topology"),
            ] {
                sql_query(format!(
                    "
                    CREATE TEMP TRIGGER {trigger_name}
                    AFTER INSERT ON {table_name}
                    BEGIN
                        INSERT INTO worker_capture_probe (kind) VALUES ('{kind}');
                    END
                    "
                ))
                .execute(connection)?;
            }
            Ok(())
        })
        .expect("worker temp capture triggers should install");

    let session_id = SessionId::new();
    let pane_id = PaneId::new();
    let route = SessionRoute {
        backend: BackendKind::Native,
        authority: RouteAuthority::LocalDaemon,
        external: None,
    };

    store
        .record_v2_ui_input(UiInputEventInput {
            session_id: session_id.0.to_string(),
            route: route.clone(),
            title: Some("worker capture shell".to_string()),
            launch: None,
            pane_id: pane_id.0.to_string(),
            data: "git status\r".to_string(),
            is_paste: false,
            source_event_id: Some("worker-capture-ui".to_string()),
            rows: Some(24),
            cols: Some(80),
            shell_kind: Some("cmd".to_string()),
        })
        .expect("ui input should persist through worker connection");
    store
        .record_v2_terminal_output(TerminalOutputEventInput {
            session_id: session_id.0.to_string(),
            route: route.clone(),
            title: Some("worker capture shell".to_string()),
            launch: None,
            pane_id: pane_id.0.to_string(),
            tab_id: None,
            payload: b"worker output\r\n".to_vec(),
            rows: Some(24),
            cols: Some(80),
            source_sequence: Some(1),
            occurred_at_ms: None,
            capture_semantics: Some("raw_vt_stream".to_string()),
        })
        .expect("terminal output should persist through worker connection");
    store
        .record_v2_history_gap(HistoryGapEventInput {
            session_id: session_id.0.to_string(),
            route: route.clone(),
            title: Some("worker capture shell".to_string()),
            launch: None,
            pane_id: pane_id.0.to_string(),
            tab_id: None,
            rows: Some(24),
            cols: Some(80),
            skipped_events: 2,
            estimated_dropped_bytes: Some(16),
            reason: "worker_capture_probe".to_string(),
            occurred_at_ms: None,
        })
        .expect("history gap should persist through worker connection");

    let screen = sample_snapshot(session_id, "worker capture shell", "worker screen")
        .screens
        .into_iter()
        .next()
        .expect("sample screen should exist");
    store
        .record_v2_screen_snapshot(ScreenSnapshotEventInput {
            session_id: session_id.0.to_string(),
            route: route.clone(),
            title: Some("worker capture shell".to_string()),
            launch: None,
            tab_id: None,
            screen,
            buffer_kind: Some("normal".to_string()),
            capture_semantics: Some("rendered_plaintext_snapshot".to_string()),
        })
        .expect("screen snapshot should persist through worker connection");
    store
        .record_v2_topology_snapshot(TopologySnapshotEventInput {
            session_id: session_id.0.to_string(),
            route,
            title: Some("worker capture shell".to_string()),
            launch: None,
            topology: TopologySnapshot {
                session_id,
                backend_kind: BackendKind::Native,
                focused_tab: None,
                tabs: Vec::new(),
            },
        })
        .expect("topology snapshot should persist through worker connection");

    let inserted_count = executor
        .execute_with_connection(|connection| {
            let row = sql_query("SELECT COUNT(*) AS count FROM worker_capture_probe")
                .get_result::<WorkerProbeCount>(connection)?;
            Ok(row.count)
        })
        .expect("worker capture temp trigger count should load");

    assert_eq!(inserted_count, 7);

    drop(store);
    let _ = std::fs::remove_file(path);
}

#[test]
fn upgrades_legacy_saved_session_schema_without_session_routes_table() {
    let nonce = SqliteSessionStore::save_timestamp_ms().expect("timestamp should resolve");
    let path =
        std::env::temp_dir().join(format!("terminal-platform-legacy-routes-{nonce}.sqlite3"));
    let connection = Connection::open(&path).expect("legacy db should open");
    connection
        .execute_batch(
            "
                CREATE TABLE native_saved_sessions (
                    session_id TEXT PRIMARY KEY,
                    route_json TEXT NOT NULL,
                    title TEXT,
                    launch_json TEXT,
                    topology_json TEXT NOT NULL,
                    screens_json TEXT NOT NULL,
                    saved_at_ms INTEGER NOT NULL
                );
                ",
        )
        .expect("legacy schema should be created");
    drop(connection);

    let store = SqliteSessionStore::open(&path).expect("store should upgrade legacy schema");
    let record = SessionRouteRecord {
        session_id: SessionId::new(),
        route: SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "tmux_session".to_string(),
                value: "after-upgrade".to_string(),
            }),
        },
        route_fingerprint: "tmux/import/after-upgrade".to_string(),
    };

    store.upsert_session_route(&record).expect("route record should save after upgrade");
    assert_eq!(
        store
            .load_session_route_by_fingerprint(&record.route_fingerprint)
            .expect("lookup by fingerprint should succeed"),
        Some(record)
    );

    let _ = std::fs::remove_file(path);
}
