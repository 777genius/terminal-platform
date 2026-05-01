use super::{prelude::*, support::*};

#[test]
fn exports_typescript_bindings_for_node_surface() {
    let export_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
    export_typescript_bindings_to(&export_dir).expect("bindings export should succeed");

    assert!(export_dir.join("NodeBindingVersion.ts").exists());
    assert!(export_dir.join("NodeHandshakeInfo.ts").exists());
    assert!(export_dir.join("NodeBackendCapabilitiesInfo.ts").exists());
    assert!(export_dir.join("NodeSavedSessionSummary.ts").exists());
    assert!(export_dir.join("NodeScreenDelta.ts").exists());
    assert!(export_dir.join("NodeMuxCommand.ts").exists());
    assert!(export_dir.join("NodeSubscriptionSpec.ts").exists());
    assert!(export_dir.join("NodeSubscriptionEvent.ts").exists());
    assert!(export_dir.join("NodeAttachedSession.ts").exists());
    let binding = std::fs::read_to_string(export_dir.join("NodeHandshakeInfo.ts"))
        .expect("handshake binding should be readable");
    assert!(binding.contains("NodeHandshakeInfo"));
}

#[test]
fn uses_platform_appropriate_launch_contract_for_node_host_smoke() {
    let request = cat_launch_request("shell");

    #[cfg(unix)]
    {
        let launch = request.launch.expect("cat launch request should include launch");
        assert_eq!(launch.program, "/bin/sh");
        assert_eq!(launch.args, vec!["-lc".to_string(), "printf 'ready\\n'; exec cat".to_string()]);
    }

    #[cfg(windows)]
    {
        assert!(request.launch.is_none());
    }
}
