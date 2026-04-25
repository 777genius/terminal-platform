# Manual Run Drafts

This directory is for in-progress manual acceptance notes.

Use it when you want to scaffold a run file before the run is complete without
accidentally creating a fake recorded pass inside [`../runs`](../runs/).

Recommended flow:

1. Create a draft with `cargo run -p xtask -- scaffold-manual-run --kind <kind> --date YYYY-MM-DD`
   For hosted Windows evidence you may also prefill the GitHub metadata with `--workflow <run-url>` and `--job "windows-v1 (...)"`.
2. Execute the real checklist and update the draft file with actual findings and notes
   For Windows hosted evidence, replace the draft `Workflow:` and `Job:` lines with the real GitHub Actions run URL and exact `windows-v1` job.
3. Change `Result: pending` to `Result: pass` only after the run is genuinely complete
4. Move the file into [`../runs`](../runs/) with the same `kind-date.md` filename
5. Re-run `cargo run -p xtask -- verify-v1-readiness --require-recorded-passes`

Draft files in this directory are intentionally ignored by the strict readiness audit.
