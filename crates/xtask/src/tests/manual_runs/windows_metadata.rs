use std::fs;

use crate::{ManualRunKind, ManualRunScaffoldOptions, scaffold_manual_run, verify_recorded_passes};

use super::super::fixtures::TestDir;

#[test]
fn verify_recorded_passes_rejects_windows_hosted_artifact_without_workflow_metadata() {
    let dir = TestDir::new();
    dir.write_file("README.md", "# Recorded Manual Passes\n");
    dir.write_file("_template.md", "# Run Title\n");
    dir.write_file(
        "electron-2026-04-20.md",
        "\
Date: 2026-04-20
OS: macOS 15.4
Checklist: crates/terminal-testing/manual/electron.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: n/a

## Scope

Electron embed lifecycle and resize churn.

## Findings

no issues found

## Notes

none
",
    );
    dir.write_file(
        "unix-tmux-2026-04-20.md",
        "\
Date: 2026-04-20
OS: Ubuntu 24.04
Checklist: crates/terminal-testing/manual/tmux.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: 3.5a
Zellij: n/a

## Scope

tmux import and detach or reattach.

## Findings

no issues found

## Notes

none
",
    );
    dir.write_file(
        "windows-native-zellij-2026-04-20.md",
        "\
Date: 2026-04-20
OS: Windows 11 24H2
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pass

Rust: rustc 1.88.0
Node: v20.19.0
tmux: n/a
Zellij: 0.44.1

## Scope

Native create or attach plus imported zellij mutation lane.

## Findings

no issues found

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected missing workflow metadata to fail"),
        Err(error) => error,
    };
    assert!(error.contains("Workflow: "), "expected hosted workflow marker error, got: {error}");
}

#[test]
fn scaffold_manual_run_injects_windows_workflow_metadata() {
    let dir = TestDir::new();
    let output = dir.path().join("windows-native-zellij-2026-04-22.md");

    let output_path = scaffold_manual_run(
        ManualRunKind::WindowsNativeZellij,
        "2026-04-22",
        ManualRunScaffoldOptions {
            output: Some(output.clone()),
            os: Some("Windows GitHub-hosted runner image".to_string()),
            rust: Some("rustc 1.90.0".to_string()),
            node: Some("v20.19.0".to_string()),
            tmux: Some("n/a".to_string()),
            zellij: Some("0.44.2".to_string()),
            workflow: Some(
                "https://github.com/example/terminal-platform/actions/runs/123456789"
                    .to_string(),
            ),
            job: Some(
                "windows-v1 (https://github.com/example/terminal-platform/actions/runs/123456789/job/987654321)"
                    .to_string(),
            ),
            force: false,
        },
    )
    .expect("windows scaffold should write");

    let contents =
        fs::read_to_string(&output_path).expect("scaffolded manual run should read back");

    assert_eq!(output_path, output);
    assert!(contents.contains("Result: pending"), "contents: {contents}");
    assert!(
        contents.contains(
            "Workflow: https://github.com/example/terminal-platform/actions/runs/123456789"
        ),
        "contents: {contents}"
    );
    assert!(
        contents.contains(
            "Job: windows-v1 (https://github.com/example/terminal-platform/actions/runs/123456789/job/987654321)"
        ),
        "contents: {contents}"
    );
}
