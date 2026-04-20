# V1 Manual Closeout Runbook

This runbook exists to make the last v1 acceptance-evidence gate explicit and repeatable.

The strict readiness audit requires recorded pass artifacts for:

- Electron embed
- Unix `tmux`
- Windows `Native + Zellij`

## Rule

Do not write fake recorded passes directly into `crates/terminal-testing/manual/runs/`.

Use draft files first, complete the real run, then promote the finished artifact into `manual/runs/`.

For Windows or other target environments where hosted CI is the strongest source of truth,
you may promote a hosted acceptance run instead of a local hands-on run, but the artifact must
link the workflow URL, identify the exact job, and record the printed tool versions.

## 1. Scaffold draft files

```bash
cargo run -p xtask -- scaffold-manual-run --kind electron --date YYYY-MM-DD
cargo run -p xtask -- scaffold-manual-run --kind unix-tmux --date YYYY-MM-DD
cargo run -p xtask -- scaffold-manual-run --kind windows-native-zellij --date YYYY-MM-DD --zellij "zellij 0.44.x"
```

By default these commands create draft files under:

- `crates/terminal-testing/manual/drafts/`

They intentionally use `Result: pending`.

## 2. Execute the real checklist

Use the checklist files as source of truth:

- `crates/terminal-testing/manual/electron.md`
- `crates/terminal-testing/manual/tmux.md`
- `crates/terminal-testing/manual/windows-native-zellij.md`

Minimum expectations:

- actually run the listed flows
- record the real OS and tool versions
- write real findings, even if the value is `no issues found`
- keep the checklist path accurate

## 3. Promote a completed draft into recorded evidence

After the run is genuinely complete:

1. change `Result: pending` to `Result: pass`
2. keep the filename aligned with `Date:`
3. move the file into `crates/terminal-testing/manual/runs/`

Example:

```bash
mv \
  crates/terminal-testing/manual/drafts/electron-YYYY-MM-DD.md \
  crates/terminal-testing/manual/runs/electron-YYYY-MM-DD.md
```

## 4. Re-run the strict readiness gate

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```

The v1 closeout is not complete until this command passes.

## 5. Final release closeout

After strict readiness is green:

1. confirm hosted `ci` is green on the release commit
2. trigger `release-readiness`
3. review the `release-plz` PR
4. publish from the release PR or merge it as the v1 release candidate handoff
