use terminal_backend_api::{
    BackendRawOutputEvent, CreateSessionSpec, MuxBackendPort, MuxCommand, SendInputSpec,
    ShellLaunchSpec,
};
use terminal_projection::{ScreenColor, ScreenCursorShape, ScreenUnderlineStyle};

use crate::NativeBackend;

use super::support::{cat_launch_spec, echo_input, wait_for_screen_line};

#[tokio::test]
async fn writes_input_into_live_pty_backed_session() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "ready").await;
    let before = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let result = session
        .dispatch(MuxCommand::SendInput(SendInputSpec {
            pane_id,
            data: echo_input("hello from backend test"),
            client_event_id: None,
        }))
        .await
        .expect("send input should succeed");

    assert!(!result.changed);
    wait_for_screen_line(&*session, pane_id, "hello from backend test").await;
    let delta =
        session.screen_delta(pane_id, before.sequence).await.expect("screen delta should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, before.sequence);
    assert!(delta.to_sequence > before.sequence);
    assert!(
        patch.line_updates.iter().any(|line| line.line.text.contains("hello from backend test"))
    );
    assert!(delta.full_replace.is_none());
}

#[tokio::test]
async fn streams_raw_output_for_live_pty_backed_session() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "ready").await;
    let mut raw_subscription =
        session.subscribe_raw_output(pane_id).await.expect("raw output should subscribe");
    let marker = "hello from native raw output";
    session
        .dispatch(MuxCommand::SendInput(SendInputSpec {
            pane_id,
            data: echo_input(marker),
            client_event_id: None,
        }))
        .await
        .expect("send input should succeed");

    let mut payload = Vec::new();
    for _ in 0..40 {
        if let Some(BackendRawOutputEvent::Bytes(bytes)) = raw_subscription.events.recv().await {
            payload.extend(bytes.payload);
            if payload.windows(marker.len()).any(|window| window == marker.as_bytes()) {
                raw_subscription.cancel();
                return;
            }
        }
    }

    panic!("raw output never contained marker; payload={:?}", String::from_utf8_lossy(&payload));
}

#[cfg(unix)]
#[tokio::test]
async fn advertises_truecolor_terminal_capabilities_to_live_pty_processes() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("terminal-env".to_string()),
            launch: Some(terminal_environment_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    let expected = "TERM=xterm-256color COLORTERM=truecolor TERMINAL_PLATFORM=1";
    wait_for_screen_line(&*session, pane_id, expected).await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("TERM=xterm-256color"))
        .expect("terminal capability environment line should be present");

    assert_eq!(line.text, expected);
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_rich_terminal_styles_in_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("rich-output".to_string()),
            launch: Some(rich_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "red true back under link").await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("red true back under link"))
        .expect("rich output line should be present");

    assert_eq!(line.text, "red true back under link");
    assert!(
        line.spans.iter().any(|span| {
            span.text == "red"
                && span.style.bold
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }),
        "red span should preserve bold named foreground: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "true"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        }),
        "truecolor span should preserve rgb foreground: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "back" && span.style.background == Some(ScreenColor::Indexed { index: 22 })
        }),
        "indexed span should preserve 256-color background: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "under" && span.style.underline == Some(ScreenUnderlineStyle::Single)
        }),
        "underline span should preserve underline style: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "link" && span.style.hyperlink.as_deref() == Some("https://example.com")
        }),
        "OSC 8 span should preserve hyperlink uri: {:?}",
        line.spans
    );
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_advanced_rich_terminal_styles_in_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("advanced-rich-output".to_string()),
            launch: Some(advanced_rich_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "bright curl strike rgba cmyk").await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("bright curl strike rgba cmyk"))
        .expect("advanced rich output line should be present");

    assert_eq!(line.text, "bright curl strike rgba cmyk");
    assert!(
        line.spans.iter().any(|span| {
            span.text == "bright"
                && span.style.foreground
                    == Some(ScreenColor::Named { name: "bright_red".to_string() })
        }),
        "bright SGR color should preserve named bright foreground: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "curl"
                && span.style.underline == Some(ScreenUnderlineStyle::Curly)
                && span.style.underline_color == Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 })
        }),
        "curly underline and truecolor underline color should survive: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| span.text == "strike" && span.style.strikethrough),
        "strikethrough should survive: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "rgba"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 12, g: 34, b: 56 })
        }),
        "RGBA SGR should degrade to RGB in live screen snapshots: {:?}",
        line.spans
    );
    assert!(
        line.spans.iter().any(|span| {
            span.text == "cmyk"
                && span.style.foreground == Some(ScreenColor::Rgb { r: 191, g: 95, b: 0 })
        }),
        "CMYK SGR should convert to RGB in live screen snapshots: {:?}",
        line.spans
    );
}

#[cfg(unix)]
#[tokio::test]
async fn strips_raw_c1_privacy_control_strings_in_live_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("c1-privacy-output".to_string()),
            launch: Some(raw_c1_privacy_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "before  middle  after").await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("before"))
        .expect("C1 privacy output line should be present");

    assert_eq!(line.text, "before  middle  after");
    assert!(
        !line.text.contains("secret") && !line.text.contains("private"),
        "raw C1 privacy payload should not leak into live snapshot: {:?}",
        line
    );
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_raw_c1_compat_controls_in_live_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("c1-compat-output".to_string()),
            launch: Some(raw_c1_compat_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    let expected = "before A middle B guarded x after";
    wait_for_screen_line(&*session, pane_id, expected).await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("before"))
        .expect("C1 compat output line should be present");

    assert_eq!(line.text, expected);
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_fullwidth_terminal_cells_without_spacer_artifacts() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("fullwidth-output".to_string()),
            launch: Some(fullwidth_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "wide: 表A").await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("wide:"))
        .expect("fullwidth output line should be present");

    assert_eq!(line.text, "wide: 表A");
    assert!(
        line.spans.iter().any(|span| {
            span.text == "表"
                && span.style.foreground == Some(ScreenColor::Named { name: "red".to_string() })
        }),
        "fullwidth styled cell should preserve foreground span: {:?}",
        line.spans
    );
    assert!(
        !line.text.contains("表 A"),
        "wide spacer cell should not be surfaced as a visible space: {:?}",
        line
    );
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_combining_marks_in_styled_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("combining-output".to_string()),
            launch: Some(combining_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let expected = combining_output_text();

    wait_for_screen_line(&*session, pane_id, &expected).await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let line = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text.contains("accent:"))
        .expect("combining output line should be present");
    let styled_accented = format!("e{}", '\u{0301}');

    assert_eq!(line.text, expected);
    assert!(
        line.spans.iter().any(|span| {
            span.text == styled_accented
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }),
        "combining mark should remain attached to the styled base glyph: {:?}",
        line.spans
    );
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_soft_wrapped_screen_lines_in_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("soft-wrap-output".to_string()),
            launch: Some(soft_wrap_output_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, &soft_wrap_second_line()).await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let first_line = soft_wrap_first_line();
    let second_line = soft_wrap_second_line();
    let first = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text == first_line)
        .expect("first wrapped row should be present");
    let second = screen
        .surface
        .lines
        .iter()
        .find(|line| line.text == second_line)
        .expect("second wrapped row should be present");

    assert!(first.wrapped, "first row should preserve soft-wrap metadata");
    assert!(!second.wrapped, "continuation row should not be marked as wrapping onward");
}

#[cfg(unix)]
#[tokio::test]
async fn preserves_cursor_shape_and_blinking_in_screen_snapshot() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("cursor-style".to_string()),
            launch: Some(cursor_style_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "cursor-style-ready").await;
    let screen = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let cursor = screen.surface.cursor.expect("screen cursor should be present");

    assert_eq!(cursor.shape, Some(ScreenCursorShape::Beam));
    assert!(cursor.blinking, "cursor should preserve DECSCUSR blinking style");
}

#[cfg(unix)]
fn rich_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args([
        "-lc",
        "printf '\\033[31;1mred\\033[0m \\033[38;2;12;34;56mtrue\\033[0m \\033[48;5;22mback\\033[0m \\033[4munder\\033[0m \\033]8;;https://example.com\\033\\\\link\\033]8;;\\033\\\\\\n'; sleep 0.2",
    ])
}

#[cfg(unix)]
fn terminal_environment_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args([
        "-lc",
        "printf 'TERM=%s COLORTERM=%s TERMINAL_PLATFORM=%s\\n' \"$TERM\" \"$COLORTERM\" \"$TERMINAL_PLATFORM\"; sleep 0.2",
    ])
}

#[cfg(unix)]
fn advanced_rich_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args([
        "-lc",
        "printf '\\033[91mbright\\033[0m \\033[4:3;58:2::1:2:3mcurl\\033[0m \\033[9mstrike\\033[0m \\033[38:6::12:34:56:128mrgba\\033[0m \\033[38:4::0:128:255:64mcmyk\\033[0m\\n'; sleep 0.2",
    ])
}

#[cfg(unix)]
fn raw_c1_privacy_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args([
        "-lc",
        "printf 'before \\230secret\\234 middle \\236private\\234 after\\n'; sleep 0.2",
    ])
}

#[cfg(unix)]
fn raw_c1_compat_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args([
        "-lc",
        "printf 'before \\216A middle \\217B guarded \\226x\\227 \\232after\\n'; sleep 0.2",
    ])
}

#[cfg(unix)]
fn fullwidth_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh")
        .with_args(["-lc", "printf 'wide: \\033[31m表\\033[0mA\\n'; sleep 0.2"])
}

#[cfg(unix)]
fn combining_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(vec![
        "-lc".to_string(),
        format!("printf 'accent: \\033[32me{}\\033[0mZ\\n'; sleep 0.2", '\u{0301}'),
    ])
}

#[cfg(unix)]
fn combining_output_text() -> String {
    format!("accent: e{}Z", '\u{0301}')
}

#[cfg(unix)]
fn soft_wrap_output_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(vec![
        "-lc".to_string(),
        format!("printf '{}\\n'; sleep 0.2", soft_wrap_full_line()),
    ])
}

#[cfg(unix)]
fn soft_wrap_full_line() -> String {
    "0123456789".repeat(10)
}

#[cfg(unix)]
fn soft_wrap_first_line() -> String {
    soft_wrap_full_line().chars().take(80).collect()
}

#[cfg(unix)]
fn soft_wrap_second_line() -> String {
    soft_wrap_full_line().chars().skip(80).collect()
}

#[cfg(unix)]
fn cursor_style_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh")
        .with_args(["-lc", "printf '\\033[5 qcursor-style-ready\\n'; sleep 0.2"])
}
