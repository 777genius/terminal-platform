use super::prelude::*;

#[test]
fn runtime_handshake_reflects_available_backends() {
    let runtime = TerminalRuntime::new(BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>
    ]));
    let handshake = runtime.handshake();

    assert_eq!(handshake.daemon_phase, RuntimePhase::Ready);
    assert_eq!(handshake.available_backends, vec![terminal_domain::BackendKind::Native]);
    assert_eq!(runtime.session_count(), 0);
}
