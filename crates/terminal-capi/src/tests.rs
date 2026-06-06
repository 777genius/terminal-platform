use std::ffi::CString;

use std::{sync::mpsc, thread};

use serde_json::{Value, json};
use terminal_daemon::spawn_local_socket_server;
use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::LocalSocketAddress;
use terminal_testing::{
    daemon, daemon_fixture, echo_shell_launch_spec, unique_socket_address, wait_for_daemon_ready,
};

use super::*;
use crate::prelude::{TerminalCapiClientHandle, TerminalCapiSubscriptionHandle};

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_c_api_host_flow_against_daemon_fixture() {
    let fixture = match daemon_fixture("terminal-capi-smoke") {
        Ok(fixture) => fixture,
        Err(error) => panic!("fixture should start: {error}"),
    };
    let address = fixture.client.address().clone();

    let join = std::thread::spawn(move || {
        let handle = match address {
            LocalSocketAddress::Namespaced(value) => {
                let value = c_string(&value);
                let result = terminal_capi_client_new_from_namespaced_address(value.as_ptr());
                assert_eq!(result.status, TerminalCapiStatus::Ok);
                result.client
            }
            LocalSocketAddress::Filesystem(path) => {
                let path = c_string(&path.display().to_string());
                let result = terminal_capi_client_new_from_filesystem_path(path.as_ptr());
                assert_eq!(result.status, TerminalCapiStatus::Ok);
                result.client
            }
        };

        let binding_version = read_json_result(terminal_capi_client_binding_version_json(handle));
        assert_eq!(binding_version["protocol"]["major"], 0);
        assert_eq!(binding_version["protocol"]["minor"], 2);

        let handshake = read_json_result(terminal_capi_client_handshake_info_json(handle));
        assert_eq!(handshake["assessment"]["can_use"], true);
        let backend = c_string("native");
        let capabilities = read_json_result(terminal_capi_client_backend_capabilities_json(
            handle,
            backend.as_ptr(),
        ));
        assert_eq!(capabilities["backend"], "native");
        assert_eq!(capabilities["capabilities"]["explicit_session_save"], true);

        let create_request = create_native_session_request("capi-smoke");
        let created = read_json_result(terminal_capi_client_create_native_session_json(
            handle,
            create_request.as_ptr(),
        ));
        let session_id = created["session_id"].as_str().unwrap_or_default().to_string();
        assert!(!session_id.is_empty());

        let listed = read_json_result(terminal_capi_client_list_sessions_json(handle));
        assert!(
            listed
                .as_array()
                .map(|sessions| {
                    sessions
                        .iter()
                        .any(|session| session["session_id"].as_str() == Some(session_id.as_str()))
                })
                .unwrap_or(false),
        );

        let session_id_c = c_string(&session_id);
        let attached = read_json_result(terminal_capi_client_attach_session_json(
            handle,
            session_id_c.as_ptr(),
        ));
        let pane_id =
            attached["focused_screen"]["pane_id"].as_str().unwrap_or_default().to_string();
        assert!(!pane_id.is_empty());

        let topology_subscription_spec = c_string(r#"{"kind":"session_topology"}"#);
        let topology_subscription =
            read_subscription_result(terminal_capi_client_open_subscription(
                handle,
                session_id_c.as_ptr(),
                topology_subscription_spec.as_ptr(),
            ));
        let topology_meta =
            read_json_result(terminal_capi_subscription_meta_json(topology_subscription));
        assert!(!topology_meta["subscription_id"].as_str().unwrap_or_default().is_empty());
        let initial_topology_event =
            read_json_result(terminal_capi_subscription_next_event_json(topology_subscription));
        assert_eq!(initial_topology_event["kind"], "topology_snapshot");
        assert_eq!(initial_topology_event["tabs"].as_array().map_or(0, Vec::len), 1);

        let pane_subscription_spec =
            c_string(&format!(r#"{{"kind":"pane_surface","pane_id":"{pane_id}"}}"#));
        let pane_subscription = read_subscription_result(terminal_capi_client_open_subscription(
            handle,
            session_id_c.as_ptr(),
            pane_subscription_spec.as_ptr(),
        ));
        let pane_meta = read_json_result(terminal_capi_subscription_meta_json(pane_subscription));
        assert!(!pane_meta["subscription_id"].as_str().unwrap_or_default().is_empty());
        let initial_pane_event =
            read_json_result(terminal_capi_subscription_next_event_json(pane_subscription));
        assert_eq!(initial_pane_event["kind"], "screen_delta");
        assert!(initial_pane_event["full_replace"].is_object());

        let pane_id_c = c_string(&pane_id);
        let snapshot = read_json_result(terminal_capi_client_screen_snapshot_json(
            handle,
            session_id_c.as_ptr(),
            pane_id_c.as_ptr(),
        ));
        assert_eq!(snapshot["pane_id"], pane_id);

        let delta = read_json_result(terminal_capi_client_screen_delta_json(
            handle,
            session_id_c.as_ptr(),
            pane_id_c.as_ptr(),
            0,
        ));
        assert_eq!(delta["pane_id"], pane_id);

        let mux_command = c_string(r#"{"kind":"new_tab","title":"ffi"}"#);
        let dispatch = read_json_result(terminal_capi_client_dispatch_mux_command_json(
            handle,
            session_id_c.as_ptr(),
            mux_command.as_ptr(),
        ));
        assert_eq!(dispatch["changed"], true);
        let topology_update = wait_for_topology_event(topology_subscription, 2);
        assert_eq!(topology_update["tabs"].as_array().map_or(0, Vec::len), 2);

        let input_command = c_string(&format!(
            r#"{{"kind":"send_input","pane_id":"{pane_id}","data":"ffi subscription input\r"}}"#
        ));
        let input_result = read_json_result(terminal_capi_client_dispatch_mux_command_json(
            handle,
            session_id_c.as_ptr(),
            input_command.as_ptr(),
        ));
        assert_eq!(input_result["changed"], false);
        let pane_update =
            wait_for_screen_delta_with_text(pane_subscription, "ffi subscription input");
        assert_eq!(pane_update["kind"], "screen_delta");

        let save_command = c_string(r#"{"kind":"save_session"}"#);
        let save = read_json_result(terminal_capi_client_dispatch_mux_command_json(
            handle,
            session_id_c.as_ptr(),
            save_command.as_ptr(),
        ));
        assert_eq!(save["changed"], false);

        let saved_sessions =
            read_json_result(terminal_capi_client_list_saved_sessions_json(handle));
        assert!(
            saved_sessions
                .as_array()
                .map(|sessions| {
                    sessions
                        .iter()
                        .any(|session| session["session_id"].as_str() == Some(session_id.as_str()))
                })
                .unwrap_or(false),
        );

        let saved = read_json_result(terminal_capi_client_saved_session_json(
            handle,
            session_id_c.as_ptr(),
        ));
        assert_eq!(saved["session_id"], session_id);
        assert_eq!(saved["compatibility"]["can_restore"], true);

        let restored = read_json_result(terminal_capi_client_restore_saved_session_json(
            handle,
            session_id_c.as_ptr(),
        ));
        assert_eq!(restored["saved_session_id"], session_id);

        let topology = read_json_result(terminal_capi_client_topology_snapshot_json(
            handle,
            session_id_c.as_ptr(),
        ));
        assert_eq!(topology["tabs"].as_array().map_or(0, Vec::len), 2);

        let deleted = read_json_result(terminal_capi_client_delete_saved_session_json(
            handle,
            session_id_c.as_ptr(),
        ));
        assert_eq!(deleted["session_id"], session_id);

        let closed_topology =
            read_json_result(terminal_capi_subscription_close(topology_subscription));
        assert_eq!(closed_topology["closed"], true);

        let closed_pane = read_json_result(terminal_capi_subscription_close(pane_subscription));
        assert_eq!(closed_pane["closed"], true);

        // SAFETY: subscription handles were returned by this crate and are freed exactly once.
        unsafe {
            terminal_capi_subscription_free(topology_subscription);
            terminal_capi_subscription_free(pane_subscription);
        }

        // SAFETY: handle was returned by this crate and is freed exactly once here.
        unsafe { terminal_capi_client_free(handle) };
    });

    match join.join() {
        Ok(()) => {}
        Err(_) => panic!("c api worker thread should succeed"),
    }

    fixture.shutdown().await.unwrap_or_else(|error| panic!("fixture should stop cleanly: {error}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn repeatedly_reopens_subscriptions_through_c_api_surface() {
    let fixture = daemon_fixture("terminal-capi-reopen").expect("fixture should start");
    let address = fixture.client.address().clone();
    let join = thread::spawn(move || {
        let handle = client_handle_from_address(&address);

        let (session_id, pane_id) = create_native_session_and_attach(handle, "capi-reopen");
        let session_id_c = c_string(&session_id);

        for cycle in 0..24 {
            let topology_subscription = open_topology_subscription(handle, session_id_c.as_ptr());
            let initial_topology =
                read_json_result(terminal_capi_subscription_next_event_json(topology_subscription));
            assert_eq!(initial_topology["kind"], "topology_snapshot");
            assert_eq!(initial_topology["session_id"], session_id);
            let closed_topology =
                read_json_result(terminal_capi_subscription_close(topology_subscription));
            assert_eq!(closed_topology["closed"], true);
            unsafe {
                terminal_capi_subscription_free(topology_subscription);
            }

            let pane_subscription = open_pane_subscription(handle, session_id_c.as_ptr(), &pane_id);
            let initial_pane =
                read_json_result(terminal_capi_subscription_next_event_json(pane_subscription));
            assert_eq!(initial_pane["kind"], "screen_delta");
            assert!(initial_pane["full_replace"].is_object());

            if cycle % 6 == 5 {
                let marker = format!("capi reopen cycle {cycle}");
                let input_command = c_string(&format!(
                    r#"{{"kind":"send_input","pane_id":"{pane_id}","data":"{marker}\r"}}"#
                ));
                let dispatched = read_json_result(terminal_capi_client_dispatch_mux_command_json(
                    handle,
                    session_id_c.as_ptr(),
                    input_command.as_ptr(),
                ));
                assert_eq!(dispatched["changed"], false);
                let pane_update = wait_for_screen_delta_with_text(pane_subscription, &marker);
                assert_eq!(pane_update["kind"], "screen_delta");
            }

            let closed_pane = read_json_result(terminal_capi_subscription_close(pane_subscription));
            assert_eq!(closed_pane["closed"], true);
            unsafe {
                terminal_capi_subscription_free(pane_subscription);
            }
        }

        let listed = read_json_result(terminal_capi_client_list_sessions_json(handle));
        assert!(
            listed
                .as_array()
                .map(|sessions| {
                    sessions
                        .iter()
                        .any(|session| session["session_id"].as_str() == Some(session_id.as_str()))
                })
                .unwrap_or(false),
        );

        unsafe {
            terminal_capi_client_free(handle);
        }
    });

    expect_thread_success(join);
    fixture.shutdown().await.unwrap_or_else(|error| panic!("fixture should stop cleanly: {error}"));
}

#[tokio::test(flavor = "multi_thread")]
async fn recovers_c_api_client_across_multiple_daemon_restart_cycles() {
    let address = unique_socket_address("terminal-capi-restart-cycles");
    let (event_tx, event_rx) = mpsc::channel::<usize>();
    let (ack_tx, ack_rx) = mpsc::channel::<usize>();
    let worker_address = address.clone();
    let join = thread::spawn(move || {
        let handle = client_handle_from_address(&worker_address);

        for cycle in 0..3 {
            event_tx.send(cycle).expect("worker should signal cycle start");
            ack_rx.recv().expect("worker should wait for daemon-ready ack");

            let listed = read_json_result(terminal_capi_client_list_sessions_json(handle));
            assert_eq!(
                listed.as_array().map_or(0, Vec::len),
                0,
                "cycle {cycle} should start with a fresh daemon state"
            );

            let (session_id, pane_id) =
                create_native_session_and_attach(handle, &format!("capi-restart-cycle-{cycle}"));
            let session_id_c = c_string(&session_id);
            let pane_subscription = open_pane_subscription(handle, session_id_c.as_ptr(), &pane_id);
            let initial_pane =
                read_json_result(terminal_capi_subscription_next_event_json(pane_subscription));
            assert_eq!(initial_pane["kind"], "screen_delta");
            assert!(
                initial_pane["full_replace"].is_object(),
                "cycle {cycle} should receive an initial pane delta"
            );
            let closed_pane = read_json_result(terminal_capi_subscription_close(pane_subscription));
            assert_eq!(closed_pane["closed"], true);
            unsafe {
                terminal_capi_subscription_free(pane_subscription);
            }

            event_tx.send(cycle).expect("worker should signal daemon stop boundary");
            ack_rx.recv().expect("worker should wait for daemon-stopped ack");

            let stale = read_json_result_with_status(
                terminal_capi_client_list_sessions_json(handle),
                TerminalCapiStatus::ProtocolError,
            );
            assert!(
                stale["code"].as_str().is_some_and(|code| !code.is_empty()),
                "cycle {cycle} stale daemon error should expose a protocol code"
            );
        }

        unsafe {
            terminal_capi_client_free(handle);
        }
    });

    for cycle in 0..3 {
        let started_cycle = event_rx.recv().expect("test should receive cycle-start signal");
        assert_eq!(started_cycle, cycle);
        let readiness_client = LocalSocketDaemonClient::new(address.clone());
        let server = spawn_local_socket_server(daemon(), address.clone())
            .expect("daemon should bind for restart cycle");
        wait_for_daemon_ready(&readiness_client).await;
        ack_tx.send(cycle).expect("test should send daemon-ready ack");

        let stop_cycle = event_rx.recv().expect("test should receive daemon-stop signal");
        assert_eq!(stop_cycle, cycle);
        server.shutdown().await.expect("daemon should stop cleanly");
        ack_tx.send(cycle).expect("test should send daemon-stopped ack");
    }

    expect_thread_success(join);
}

fn c_string(value: &str) -> CString {
    match CString::new(value) {
        Ok(value) => value,
        Err(error) => panic!("CString creation should succeed: {error}"),
    }
}

fn create_native_session_request(title: &str) -> CString {
    c_string(&json!({ "title": title, "launch": echo_shell_launch_spec() }).to_string())
}

fn read_json_result(result: TerminalCapiStringResult) -> Value {
    read_json_result_with_status(result, TerminalCapiStatus::Ok)
}

fn read_json_result_with_status(
    result: TerminalCapiStringResult,
    expected_status: TerminalCapiStatus,
) -> Value {
    assert_eq!(result.status, expected_status);
    let raw = result.value;
    assert!(!raw.is_null());

    let owned = {
        // SAFETY: the pointer comes from this crate and is consumed exactly once here.
        unsafe { CString::from_raw(raw) }
    };
    let value = match owned.into_string() {
        Ok(value) => value,
        Err(error) => error.into_cstring().to_string_lossy().into_owned(),
    };

    serde_json::from_str(&value)
        .unwrap_or_else(|error| panic!("result JSON should parse: {error}\n{value}"))
}

fn client_handle_from_address(address: &LocalSocketAddress) -> *mut TerminalCapiClientHandle {
    match address {
        LocalSocketAddress::Namespaced(value) => {
            let value = c_string(value);
            let result = terminal_capi_client_new_from_namespaced_address(value.as_ptr());
            assert_eq!(result.status, TerminalCapiStatus::Ok);
            result.client
        }
        LocalSocketAddress::Filesystem(path) => {
            let path = c_string(&path.display().to_string());
            let result = terminal_capi_client_new_from_filesystem_path(path.as_ptr());
            assert_eq!(result.status, TerminalCapiStatus::Ok);
            result.client
        }
    }
}

fn create_native_session_and_attach(
    handle: *mut TerminalCapiClientHandle,
    title: &str,
) -> (String, String) {
    let create_request = create_native_session_request(title);
    let created = read_json_result(terminal_capi_client_create_native_session_json(
        handle,
        create_request.as_ptr(),
    ));
    let session_id = created["session_id"].as_str().unwrap_or_default().to_string();
    assert!(!session_id.is_empty());

    let session_id_c = c_string(&session_id);
    let attached =
        read_json_result(terminal_capi_client_attach_session_json(handle, session_id_c.as_ptr()));
    let pane_id = attached["focused_screen"]["pane_id"].as_str().unwrap_or_default().to_string();
    assert!(!pane_id.is_empty());

    (session_id, pane_id)
}

fn read_subscription_result(
    result: TerminalCapiSubscriptionResult,
) -> *mut TerminalCapiSubscriptionHandle {
    assert_eq!(result.status, TerminalCapiStatus::Ok);
    assert!(!result.subscription.is_null());
    result.subscription
}

fn open_topology_subscription(
    handle: *mut TerminalCapiClientHandle,
    session_id: *const std::ffi::c_char,
) -> *mut TerminalCapiSubscriptionHandle {
    let topology_subscription_spec = c_string(r#"{"kind":"session_topology"}"#);
    read_subscription_result(terminal_capi_client_open_subscription(
        handle,
        session_id,
        topology_subscription_spec.as_ptr(),
    ))
}

fn open_pane_subscription(
    handle: *mut TerminalCapiClientHandle,
    session_id: *const std::ffi::c_char,
    pane_id: &str,
) -> *mut TerminalCapiSubscriptionHandle {
    let pane_subscription_spec =
        c_string(&format!(r#"{{"kind":"pane_surface","pane_id":"{pane_id}"}}"#));
    read_subscription_result(terminal_capi_client_open_subscription(
        handle,
        session_id,
        pane_subscription_spec.as_ptr(),
    ))
}

fn wait_for_topology_event(
    subscription: *mut TerminalCapiSubscriptionHandle,
    expected_tabs: usize,
) -> Value {
    for _ in 0..8 {
        let event = read_json_result(terminal_capi_subscription_next_event_json(subscription));
        if event["kind"] == "topology_snapshot"
            && event["tabs"].as_array().map_or(0, Vec::len) == expected_tabs
        {
            return event;
        }
    }

    panic!("topology subscription should yield snapshot with {expected_tabs} tabs");
}

fn wait_for_screen_delta_with_text(
    subscription: *mut TerminalCapiSubscriptionHandle,
    needle: &str,
) -> Value {
    for _ in 0..16 {
        let event = read_json_result(terminal_capi_subscription_next_event_json(subscription));
        if event["kind"] == "screen_delta" && json_value_contains_text(&event, needle) {
            return event;
        }
    }

    panic!("pane subscription should yield screen delta containing `{needle}`");
}

fn json_value_contains_text(value: &Value, needle: &str) -> bool {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
        Value::String(text) => text.contains(needle),
        Value::Array(items) => items.iter().any(|item| json_value_contains_text(item, needle)),
        Value::Object(entries) => {
            entries.values().any(|item| json_value_contains_text(item, needle))
        }
    }
}

fn expect_thread_success(join: thread::JoinHandle<()>) {
    match join.join() {
        Ok(()) => {}
        Err(_) => panic!("c api worker thread should succeed"),
    }
}
