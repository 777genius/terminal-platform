mod support;

#[test]
fn generates_header_with_core_c_api_symbols() {
    let header = support::read_generated_header()
        .unwrap_or_else(|error| panic!("header should generate: {error}"));

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
