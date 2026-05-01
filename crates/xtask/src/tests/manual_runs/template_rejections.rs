use crate::verify_recorded_passes;

use super::super::fixtures::TestDir;

#[test]
fn verify_recorded_passes_rejects_template_placeholders() {
    let dir = TestDir::new();
    dir.write_file("README.md", "# Recorded Manual Passes\n");
    dir.write_file("_template.md", "# Run Title\n");
    dir.write_file(
        "electron-2026-04-20.md",
        "\
Date: YYYY-MM-DD
OS: macOS 15.4 / Ubuntu 24.04 / Windows 11 24H2
Checklist: crates/terminal-testing/manual/<checklist>.md
Result: pass

Rust: rustc 1.xx.x
Node: vxx.x.x
tmux: n/a
Zellij: n/a

## Scope

placeholder

## Findings

no issues found

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected placeholder artifact to fail"),
        Err(error) => error,
    };
    assert!(error.contains("placeholder"), "expected placeholder error, got: {error}");
}

#[test]
fn verify_recorded_passes_rejects_fill_from_draft_text() {
    let dir = TestDir::new();
    dir.write_file("README.md", "# Recorded Manual Passes\n");
    dir.write_file("_template.md", "# Run Title\n");
    dir.write_file(
        "windows-native-zellij-2026-04-20.md",
        "\
Date: 2026-04-20
OS: Windows GitHub-hosted runner image - fill from workflow log
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pass

Rust: fill from workflow log
Node: v20.19.0
tmux: n/a
Zellij: fill from workflow log
Workflow: fill from workflow log
Job: fill from workflow log

## Scope

Windows Native + Zellij hosted acceptance.

## Findings

fill after hosted run completes

## Notes

none
",
    );

    let error = match verify_recorded_passes(dir.path()) {
        Ok(()) => panic!("expected unresolved draft text to fail"),
        Err(error) => error,
    };
    assert!(
        error.contains("unresolved placeholder text"),
        "expected unresolved placeholder error, got: {error}"
    );
}
