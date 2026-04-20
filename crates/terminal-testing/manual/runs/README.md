# Recorded Manual Passes

This directory stores the recorded human acceptance artifacts required before calling
the project ship-ready v1.

Required pass files:

- `electron-YYYY-MM-DD.md`
- `unix-tmux-YYYY-MM-DD.md`
- `windows-native-zellij-YYYY-MM-DD.md`

Use [`_template.md`](./_template.md) as the source of truth for each run artifact.

Every recorded pass must:

- reference the checklist file that was executed
- record OS plus tool versions
- say `Result: pass`
- list findings explicitly, even if the value is `no issues found`

Do not add fake or placeholder pass files. If a manual run fails, capture the failure outside
this directory until it is resolved or explicitly documented as degraded behavior.

The readiness audit command enforces the required structure:

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```
