use super::{fakes::*, prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn import_session_reuses_canonical_id_for_same_route_in_one_process() {
    let store = SqliteSessionStore::open(unique_runtime_store_path("import-same-process"))
        .expect("isolated sqlite store should open");
    let backend = Arc::new(FakeImportedBackend::default());
    let runtime = TerminalRuntime::with_persistence(runtime_backends(backend.clone()), store);
    let route = foreign_route("workspace-a");

    let first = runtime
        .import_session(route.clone(), Some("workspace-a".to_string()))
        .await
        .expect("first import should succeed");
    let second = runtime
        .import_session(route, Some("workspace-a".to_string()))
        .await
        .expect("second import should reuse the existing session");

    assert_eq!(first.session_id, second.session_id);
    assert_eq!(backend.attached_session_ids(), vec![first.session_id]);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_session_reuses_persisted_canonical_id_after_restart_and_distinguishes_routes() {
    let path = unique_runtime_store_path("import-restart");
    let route_a = foreign_route("workspace-a");
    let route_b = foreign_route("workspace-b");

    let first_session_id = {
        let store = SqliteSessionStore::open(&path).expect("isolated sqlite store should open");
        let backend = Arc::new(FakeImportedBackend::default());
        let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
        runtime
            .import_session(route_a.clone(), Some("workspace-a".to_string()))
            .await
            .expect("first import should succeed")
            .session_id
    };

    let store = SqliteSessionStore::open(&path).expect("reopened sqlite store should open");
    let backend = Arc::new(FakeImportedBackend::default());
    let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
    let repeated = runtime
        .import_session(route_a, Some("workspace-a".to_string()))
        .await
        .expect("reimport after restart should succeed");
    let distinct = runtime
        .import_session(route_b, Some("workspace-b".to_string()))
        .await
        .expect("different foreign route should import separately");

    assert_eq!(repeated.session_id, first_session_id);
    assert_ne!(distinct.session_id, first_session_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn import_session_on_legacy_store_creates_route_registry_record() {
    let path = unique_runtime_store_path("legacy-route-registry");
    seed_legacy_saved_session_schema(&path);

    let store = SqliteSessionStore::open(&path).expect("legacy sqlite store should migrate");
    let backend = Arc::new(FakeImportedBackend::default());
    let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
    let route = foreign_route("legacy-import");
    let imported = runtime
        .import_session(route.clone(), Some("legacy-import".to_string()))
        .await
        .expect("legacy import should succeed");

    let reopened = SqliteSessionStore::open(&path).expect("migrated sqlite store should reopen");
    let fingerprint = format!(
        "v1/{:?}/{:?}/{}/{}",
        route.backend,
        route.authority,
        route.external.as_ref().expect("foreign route must have external ref").namespace,
        route.external.as_ref().expect("foreign route must have external ref").value,
    );
    let record = reopened
        .load_session_route_by_fingerprint(&fingerprint)
        .expect("route registry lookup should succeed")
        .expect("route registry record should exist");

    assert_eq!(record.session_id, imported.session_id);
    assert_eq!(record.route, route);
}
