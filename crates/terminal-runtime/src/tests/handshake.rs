use super::{prelude::*, support::*};

#[test]
fn runtime_handshake_reflects_available_backends() {
    let store = SqliteSessionStore::open(unique_runtime_store_path("handshake"))
        .expect("isolated sqlite store should open");
    let runtime = TerminalRuntime::with_persistence(
        BackendCatalog::new([Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>]),
        store,
    );
    let handshake = runtime.handshake();

    assert_eq!(handshake.daemon_phase, RuntimePhase::Ready);
    assert_eq!(handshake.available_backends, vec![terminal_domain::BackendKind::Native]);
    assert_eq!(runtime.session_count(), 0);
}
