use std::path::PathBuf;

#[test]
fn generates_header_with_core_c_api_symbols() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = crate_dir.join("cbindgen.toml");
    let config = match cbindgen::Config::from_file(&config_path) {
        Ok(config) => config,
        Err(error) => panic!("cbindgen config should load from {}: {error}", config_path.display()),
    };
    let builder =
        cbindgen::Builder::new().with_crate(crate_dir.display().to_string()).with_config(config);
    let bindings = match builder.generate() {
        Ok(bindings) => bindings,
        Err(error) => panic!("cbindgen should generate header: {error}"),
    };
    let mut header = Vec::new();
    bindings.write(&mut header);
    let header = match String::from_utf8(header) {
        Ok(header) => header,
        Err(error) => panic!("generated header should be valid UTF-8: {error}"),
    };

    assert!(header.contains("typedef enum TerminalCapiStatus"));
    assert!(header.contains("typedef struct TerminalCapiStringResult"));
    assert!(header.contains("typedef struct TerminalCapiClientResult"));
    assert!(header.contains("typedef struct TerminalCapiSubscriptionResult"));
    assert!(header.contains("terminal_capi_client_new_from_runtime_slug"));
    assert!(header.contains("terminal_capi_client_handshake_info_json"));
    assert!(header.contains("terminal_capi_client_dispatch_mux_command_json"));
    assert!(header.contains("terminal_capi_client_open_subscription"));
    assert!(header.contains("terminal_capi_subscription_next_event_json"));
    assert!(header.contains("terminal_capi_subscription_close"));
    assert!(header.contains("terminal_capi_subscription_free"));
    assert!(header.contains("terminal_capi_string_free"));
}
