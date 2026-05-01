use terminal_backend_api::{
    BackendErrorKind, MuxCommand, NewTabSpec, SendInputSpec, SendPasteSpec,
};
use terminal_domain::DegradedModeReason;

use crate::ZellijAction;

use super::fixtures::{sample_attached_session, single_tab_attached_session};

#[test]
fn builds_targeted_dispatch_actions_for_rich_surface() {
    let (attached, snapshot, first_tab, second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    assert_eq!(
        attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::NewTab(NewTabSpec { title: Some("debug".to_string()) }),
            )
            .expect("new-tab should map"),
        vec![ZellijAction::NewTab { title: Some("debug".to_string()) }]
    );
    if cfg!(windows) {
        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::FocusTab { tab_id: second_tab })
            .expect_err("windows focus-tab should be unsupported");
        assert_eq!(error.kind, BackendErrorKind::Unsupported);
    } else {
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusTab { tab_id: second_tab })
                .expect("focus-tab should map"),
            vec![ZellijAction::FocusTab { backend_tab_id: 2, display_index: 1 }]
        );
    }
    assert_eq!(
        attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::RenameTab { tab_id: second_tab, title: "renamed".to_string() },
            )
            .expect("rename-tab should map"),
        vec![ZellijAction::RenameTab { backend_tab_id: 2, title: "renamed".to_string() }]
    );
    assert_eq!(
        attached
            .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: second_tab })
            .expect("close-tab should map"),
        vec![ZellijAction::CloseTab { backend_tab_id: 2 }]
    );
    if cfg!(windows) {
        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::FocusPane { pane_id: terminal_pane })
            .expect_err("windows focus-pane should be unsupported");
        assert_eq!(error.kind, BackendErrorKind::Unsupported);
    } else {
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusPane { pane_id: terminal_pane })
                .expect("focus-pane should map"),
            vec![ZellijAction::FocusPane { pane_ref: "terminal_1".to_string() }]
        );
    }
    assert_eq!(
        attached
            .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id: terminal_pane })
            .expect("close-pane should map"),
        vec![ZellijAction::ClosePane { pane_ref: "terminal_1".to_string() }]
    );
    assert_ne!(first_tab, second_tab);
}

#[test]
fn splits_terminal_input_into_ordered_rich_actions() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let actions = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: terminal_pane,
                data: "echo\tok\r\u{1b}[A\u{0003}\u{007f}\r\n".to_string(),
                client_event_id: None,
            }),
        )
        .expect("send-input should map");

    assert_eq!(
        actions,
        vec![
            ZellijAction::WriteChars {
                pane_ref: "terminal_1".to_string(),
                chars: "echo".to_string(),
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Tab".to_string()],
            },
            ZellijAction::WriteChars {
                pane_ref: "terminal_1".to_string(),
                chars: "ok".to_string(),
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Enter".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Up".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Ctrl c".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Backspace".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Enter".to_string()],
            },
        ]
    );
}

#[test]
fn rejects_unmapped_zellij_control_input_explicitly() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let error = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: terminal_pane,
                data: "\u{0002}".to_string(),
                client_event_id: None,
            }),
        )
        .expect_err("unmapped control input should stay explicit");

    assert!(error.to_string().contains("control character"));
}

#[test]
fn maps_paste_to_target_terminal_pane() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let actions = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendPaste(SendPasteSpec {
                pane_id: terminal_pane,
                data: "hello\nworld".to_string(),
                client_event_id: None,
            }),
        )
        .expect("send-paste should map");

    assert_eq!(
        actions,
        vec![ZellijAction::Paste {
            pane_ref: "terminal_1".to_string(),
            text: "hello\nworld".to_string(),
        }]
    );
}

#[test]
fn rejects_plugin_input_writes() {
    let (attached, snapshot, _first_tab, _second_tab, _terminal_pane, plugin_pane) =
        sample_attached_session();

    let error = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: plugin_pane,
                data: "hello".to_string(),
                client_event_id: None,
            }),
        )
        .expect_err("plugin input should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}

#[test]
fn rejects_closing_last_foreign_tab() {
    let (attached, snapshot, first_tab, _pane) = single_tab_attached_session();

    let error = attached
        .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: first_tab })
        .expect_err("closing the last tab should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}

#[test]
fn rejects_closing_last_pane_in_tab() {
    let (attached, snapshot, _first_tab, pane_id) = single_tab_attached_session();

    let error = attached
        .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id })
        .expect_err("closing the last pane should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}
