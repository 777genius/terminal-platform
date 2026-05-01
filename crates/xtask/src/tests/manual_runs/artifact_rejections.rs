use crate::verify_recorded_passes;

use super::super::fixtures::TestDir;

#[test]
fn verify_recorded_passes_rejects_unexpected_artifacts() {
    let dir = TestDir::new();
    dir.write_file("README.md", "# Recorded Manual Passes\n");
    dir.write_file("_template.md", "# Run Title\n");
    dir.write_file(
        "scratch-notes-2026-04-20.md",
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

not a canonical v1 pass artifact.

## Findings

no issues found

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected unexpected artifact to fail"),
        Err(error) => error,
    };
    assert!(
        error.contains("unexpected manual run artifact"),
        "expected unexpected artifact error, got: {error}"
    );
}

#[test]
fn verify_recorded_passes_rejects_empty_required_sections() {
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

Electron embed lifecycle.

## Findings

no issues found

## Notes

",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected empty notes section to fail"),
        Err(error) => error,
    };
    assert!(error.contains("empty ## Notes"), "expected empty notes error, got: {error}");
}

#[test]
fn verify_recorded_passes_rejects_missing_findings_section() {
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

Electron embed lifecycle.

Findings:

no issues found

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected findings section mismatch to fail"),
        Err(error) => error,
    };
    assert!(error.contains("missing ## Findings"), "expected findings section error, got: {error}");
}

#[test]
fn verify_recorded_passes_rejects_missing_runtime_value_for_required_pass() {
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
tmux: n/a
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
Workflow: https://github.com/example/terminal-platform/actions/runs/123456789
Job: windows-v1 (https://github.com/example/terminal-platform/actions/runs/123456789/job/987654321)

## Scope

Native create or attach plus imported zellij mutation lane.

## Findings

no issues found

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected runtime-specific n/a marker to fail"),
        Err(error) => error,
    };
    assert!(
        error.contains("must record a real value"),
        "expected runtime value error, got: {error}"
    );
}
