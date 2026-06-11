# Release Checklist

**Checked**: 2026-04-22  
**Status**: required release artifact

## Goal

Give release owners one ordered checklist for cutting SDK previews and stable releases.

This checklist is host-facing release discipline, not runtime product truth.

## Release Candidate Checklist

Run these checks from a clean release branch before publishing any SDK release candidate:

- confirm `sdk/docs/compatibility-matrix.md` contains the current runtime protocol and package versions
- run `npm run check` from `sdk/`
- run `npm run test:public-api` from `sdk/`
- run `npm run test:packed-consumer` from `sdk/`
- run `npm run check:examples` from `sdk/`
- run `npm run check:release-readiness` from `sdk/`
- run `cargo run -p xtask -- verify-v1-readiness` from the repo root
- confirm all release notes mention public API changes, compatibility changes, and known degraded semantics
- confirm every public API deprecation has a migration note in `sdk/docs/deprecation-checkpoints.md`
- confirm the rollback owner and rollback path in `sdk/docs/rollback-plan.md`

## Stable Promotion Checklist

Stable SDK promotion additionally requires:

- browser matrix reviewed and recorded
- adapter conformance results reviewed
- packed consumer smoke green on the release commit
- demo consumer smoke green on the release commit
- `cargo run -p xtask -- verify-v1-readiness --require-recorded-passes` green on the release commit
- compatibility matrix status updated from `preview` only when stable evidence exists

## Exit State

A release is ready only when the release owner can point to:

- the release commit
- the green command output or CI run for every required gate
- the compatibility matrix row for the release
- the changelog or release notes
- the rollback decision path
- the deprecation checkpoint review
