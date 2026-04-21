# Recorded Acceptance Passes

This directory stores the recorded acceptance artifacts required before calling
the project ship-ready v1.

Required pass files:

- `electron-YYYY-MM-DD.md`
- `unix-tmux-YYYY-MM-DD.md`
- `windows-native-zellij-YYYY-MM-DD.md`

Use [`_template.md`](./_template.md) as the source of truth for each run artifact.

Every recorded pass must:

- keep the filename date aligned with the `Date:` field
- reference the checklist file that was executed
- record OS plus tool versions
- say `Result: pass`
- keep the `## Scope`, `## Findings`, and `## Notes` headings from the template
- list findings explicitly, even if the value is `no issues found`
- replace every template placeholder with a real value
- remove draft helper text and placeholder versions such as `fill from workflow log`,
  `fill after hosted run completes`, `placeholder`, `YYYY-MM-DD`, `rustc 1.xx.x`,
  or `vxx.x.x`

Do not add fake or placeholder pass files. If a manual run fails, capture the failure outside
this directory until it is resolved or explicitly documented as degraded behavior.

Hosted target-OS evidence is allowed when it is stronger than a local approximation.
If you use a hosted run, capture:

- the exact workflow URL
- the relevant job name
- the executed commands or checklist-equivalent coverage
- the real tool versions printed by that run

The readiness audit command enforces the required structure:

```bash
cargo run -p xtask -- verify-v1-readiness --require-recorded-passes
```

Do not draft incomplete files in this directory. Use [`../drafts`](../drafts/) for in-progress notes.

You can scaffold a draft file with detected local tool versions:

```bash
cargo run -p xtask -- scaffold-manual-run --kind electron --date 2026-04-20
```

That command now writes to `manual/drafts/` on purpose. Move the finished file here only after:

- the real checklist run is complete
- `Result:` is changed from `pending` to `pass`
- the findings and notes reflect the actual run

Supported kinds:

- `electron`
- `unix-tmux`
- `windows-native-zellij`
