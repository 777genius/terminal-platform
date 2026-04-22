# Windows Native + Zellij Hosted Acceptance

Date: 2026-04-21
OS: Windows GitHub-hosted runner image - fill from workflow log
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pending

Rust: fill from workflow log
Node: fill from workflow log
tmux: n/a
Zellij: fill from workflow log

## Scope

- Hosted `windows-v1` workflow job on the release commit.
- `cargo clippy --workspace --all-targets --all-features`.
- `cargo nextest run --profile ci -p terminal-backend-native -p terminal-daemon -p terminal-daemon-client -p terminal-node -p terminal-node-napi -p terminal-protocol -p terminal-testing`.
- Checklist-equivalent coverage for Windows native PTY lifecycle, staged and installed Node package flows, live `Zellij 0.44+` import/control path, screen and topology observation, ordered mutation lane, and Electron bridge lifecycle through `terminal-node-napi` smoke.
- `tmux` remains absent from Windows workflow coverage by design.

## Findings

fill after hosted run completes

## Notes

- Workflow URL: fill after hosted run completes
- Job name: windows-v1
- Record the exact runner image, printed tool versions, and any relevant annotations from the job log.
