## Summary

- what changed
- why it changed
- expected user or integrator impact

## Architecture Notes

- canonical runtime truth affected: yes or no
- backend-specific behavior affected: Native, tmux, Zellij, or none
- host surface affected: Rust, Node/Electron, C ABI, docs, CI, or none

## Support Matrix Check

- [ ] macOS + Linux promise unchanged
- [ ] Windows promise unchanged
- [ ] tmux remains Unix-only
- [ ] no fake parity introduced between Native, tmux, and Zellij

## Verification

- [ ] cargo fmt --all --check
- [ ] cargo clippy --workspace --all-targets --all-features
- [ ] cargo nextest run --workspace
- [ ] cargo run -p xtask -- verify-v1-readiness

List any focused extra checks here:

```text
```

## Manual Or Hosted Evidence

- hosted CI links, if relevant
- manual pass artifacts, if relevant
- note any intentionally deferred evidence

## Risks

- compatibility risk
- lifecycle or recovery risk
- follow-up work, if any
