# Rollback Plan

**Checked**: 2026-04-22  
**Status**: required release artifact

## Goal

Make SDK release rollback explicit before a package is published.

The rollback path must protect downstream embedders from silent broken package behavior.

## Preview Release Rollback

If a preview release exposes a public package regression:

- stop promoting the broken version
- publish a fixed preview version with a higher prerelease number
- call out the affected package entrypoints in release notes
- keep compatibility matrix status at `preview`
- add a regression test or release gate before another preview cut

## Stable Release Rollback

If a stable release exposes a public package regression:

- decide whether the release must be withdrawn or superseded
- publish a fixed patch version when the public contract can be preserved
- publish a documented major release only when the fix requires a breaking public contract change
- update release notes with affected versions and migration guidance
- update the compatibility matrix when package or protocol compatibility changes

## Rollback Owner Checklist

Before any release cut, assign one release owner who can answer:

- which package versions are affected
- which public entrypoints are affected
- whether packed consumers are broken or only workspace-linked consumers are broken
- whether a compatibility matrix update is required
- whether deprecation or migration notes are required
- whether host apps need a config change, package pin, or package upgrade

## Non-Goals

Rollback does not mean:

- editing history on published package versions
- silently changing public behavior without release notes
- hiding degraded semantics behind adapter fallback behavior
