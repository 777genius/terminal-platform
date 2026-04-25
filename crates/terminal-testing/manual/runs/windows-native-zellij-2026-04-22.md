# Windows Native + Zellij Acceptance

Date: 2026-04-22
OS: Microsoft Windows Server 2025 10.0.26100 Datacenter (GitHub-hosted image windows-2025 20260413.84.1)
Checklist: crates/terminal-testing/manual/windows-native-zellij.md
Result: pass

Rust: rustc 1.95.0 (59807616e 2026-04-14)
Node: v20.20.2
tmux: n/a
Zellij: zellij 0.44.1

Workflow: https://github.com/777genius/terminal-platform/actions/runs/24758664762
Job: windows-v1 (https://github.com/777genius/terminal-platform/actions/runs/24758664762/job/72437350078)

## Scope

Verified the Windows v1 promise through the green hosted `windows-v1` lane: native PTY session create, attach, send input, save or restore semantics, screen snapshot and delta, ordered tab lifecycle, staged and installed Node package flows, live `Zellij` discovery plus import through the package surface, topology observation, ordered imported `Zellij` mutation lane, repeated subscribe or unsubscribe cycles, `watchSessionState` focus churn, Electron bridge and preload lifecycle, resize churn under active viewport observation, native fullscreen viewport fidelity for `vim`, `less`, and `fzf`, and imported `Zellij` fullscreen viewport fidelity for `vim` plus `less`.

## Findings

no issues found

## Notes

- Hosted evidence source is the green `ci` workflow linked above, specifically the `windows-v1` job.
- Executed workflow commands included `cargo clippy --workspace --all-targets --all-features`, `cargo nextest run --profile ci --test-threads 1 -p terminal-backend-native -p terminal-daemon -p terminal-daemon-client -p terminal-node -p terminal-node-napi -p terminal-protocol -p terminal-testing`, staged package verification through `build-local-package.mjs` plus `verify-package.mjs`, and installed package verification through `pack-local-package.mjs`, `npm install`, and CJS plus ESM import probes.
- The package smoke flow exercised `runSmoke`, `runPackageWatchSmoke`, `runZellijImportSmoke`, `runElectronBridgeSmoke`, Electron preload resize churn, `screenSnapshot`, `screenDelta`, live viewport observation, `subscribeSessionState`, and ordered imported `Zellij` `new_tab`, `rename_tab`, `focus_tab`, and `close_tab` mutations.
- Printed tool versions in the recorded job were `rustc 1.95.0 (59807616e 2026-04-14)`, `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`, `node v20.20.2`, `VIM - Vi IMproved 9.2`, `less 679`, `fzf 0.71.0 (62899fd7)`, and `zellij 0.44.1`.
- `tmux` was not installed or exercised in the Windows lane and remains Unix-only in the published v1 support matrix.
- Known degraded semantic stays explicit instead of hidden: imported Windows `Zellij` fullscreen `fzf` viewport fidelity is still a manual or degraded proof path, so hosted automated proof currently uses imported `vim` plus `less` and native `vim`, `less`, and `fzf`.
