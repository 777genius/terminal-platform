# Terminal Persistence v2 - Completion Plan to 100%

**Дата**: 2026-05-25
**Ветка**: `codex/terminal-persistence-v2-20260430`
**PR**: <https://github.com/777genius/terminal-platform/pull/5>
**Базовые документы**:

- [terminal-persistence-v2-implementation-plan.md](./terminal-persistence-v2-implementation-plan.md)
- [terminal-persistence-v2-status.md](./terminal-persistence-v2-status.md)
- [deep-dive-terminal-history-journal-research.md](./deep-dive-terminal-history-journal-research.md)

## 0. Цель документа

Этот документ фиксирует, что именно должно быть сделано, чтобы основная фича `Terminal Persistence v2` считалась готовой на 100%.

100% здесь не означает "магически воскресить старый native process tree после перезапуска". Это технически другой класс задачи. 100% означает:

1. Пользователь не теряет полезную историю терминала.
2. Команды, вывод, snapshots, topology, restore evidence and degraded states сохраняются в durable DB.
3. Native restore честно показывает восстановленную историю и границу с новым live process.
4. Zellij/mux path имеет честную parity matrix: live attach, visible history restore, command/output history where technically possible.
5. Ошибки записи, gaps, corruption, storage pressure, unsupported backend semantics видимы пользователю и тестам.
6. Все ключевые сценарии покрыты unit, integration, Rust bootstrap, real browser E2E and long-running reliability tests.

Текущая оценка готовности после closeout: **100% для текущих явных PR-гарантий**, без overpromise.

Важно: эта отметка не включает non-goals из раздела 8 и не означает, что native process tree resurrected после restart. Также imported zellij saved-session restore не обещан как native saved layout: текущий контракт честно блокирует `SaveSession` для zellij через capability/API, а persistence сохраняет live import/control, command history, rendered output history and restore evidence.

## 1. Definition of Done

Фича считается готовой на 100%, когда выполнены все условия ниже.

### 1.1 Product guarantees

- Пользователь видит историю команд и вывода после restart приложения, browser host, daemon, and saved-session restore.
- Большая история не обрезается молча. Есть pagination, explicit cursors, UI load more, and preferably auto-load on scroll-up.
- Restore использует v2 journal/stream as primary source. Legacy screen snapshot is fallback only.
- Paste хранится как journal input event, но не попадает в verified rerunnable command history by default.
- Native restore clearly says: process restarted, history restored.
- Zellij restore clearly says one of:
  - attached to live zellij session, process state preserved by zellij;
  - restored historical visible transcript only;
  - degraded, with reason.
- Any persistence failure becomes visible:
  - session health degraded;
  - durable health/gap/storage-pressure record where possible;
  - UI diagnostic or restore semantics code.
- Retention/pruning never silently deletes canonical history.
- Export/support flows are redacted by default and audit-friendly.

### 1.2 Engineering guarantees

- Canonical history is append-only journal/stream segments, not browser localStorage and not UI snapshots.
- Save session orchestration is v2-first and publish-last, so legacy visible saved sessions cannot be orphaned from v2 evidence.
- Single-writer path is real, not just serialized wrapper around fresh connections.
- Diesel remains the ORM boundary for v2 persistence.
- No async `await` while holding long SQLite read transaction or cursor.
- Large restore uses paged reads and bounded memory.
- All schema/domain enum additions have generated SDK/runtime bindings and tests.
- CI runs the critical matrix on Windows and does not depend on one huge timeout-prone command.

### 1.3 Test guarantees

- Every P1 review finding has a regression test.
- Native and zellij have separate real E2E suites.
- Browser tests verify DOM and workspace state, not only command success.
- Long-history restore tests force pagination, not a tiny first page.
- Crash/restart tests kill processes in the middle of capture/save and check recovery semantics.
- Storage pressure tests simulate `SQLITE_FULL` or failpoints and check degraded health.
- All tests have deterministic markers and do not depend on manual sleeps where a state predicate can be used.

## 2. Completion Scorecard

| Niche | Closeout | Confidence | Reliability | Complexity | Final decision |
| --- | ---: | ---: | ---: | ---: | --- |
| Native terminal persistence | 100% scoped | 🎯 9 | 🛡️ 9 | 🧠 6 | Durable command/output history, restart restore, v2-first save, paged UI |
| Command and output history | 100% scoped | 🎯 9 | 🛡️ 9 | 🧠 7 | Paste is journal-only, command history dedup/scoped, rendered/raw evidence is explicit |
| Saved native session restore | 100% scoped | 🎯 9 | 🛡️ 9 | 🧠 8 | v2 history primary, snapshot fallback, restore boundary, restore drill |
| Reliability and diagnostics | 100% scoped | 🎯 9 | 🛡️ 9 | 🧠 8 | Durable fault records, storage pressure tests, integrity/restore downgrade paths |
| zellij/mux explicit guarantees | 100% scoped | 🎯 8 | 🛡️ 8 | 🧠 9 | Live import/control, full scrollback snapshot, rendered history persistence, unsupported saved-session API is explicit |
| Production UX and polish | 100% scoped | 🎯 8 | 🛡️ 8 | 🧠 7 | Restore boundary, load-more/auto-load, source/degraded semantics covered by tests |
| CI/release gates | 100% local, GitHub approval pending | 🎯 9 | 🛡️ 8 | 🧠 6 | Local matrix green; GitHub Actions can still require maintainer approval |

## 3. Requirement Matrix

### 3.1 Native Terminal Persistence

#### TPV2-NATIVE-001 - Native command input is persisted before dispatch

**Required behavior**

- `SendInput` that contains a command submit is persisted before dispatch reaches backend.
- If persistence fails, command dispatch fails with explicit backend error.
- User must not believe command history was saved if DB write failed.

**Implementation requirements**

- Keep awaited persistence barrier for UI input capture.
- Preserve idempotency through `client_event_id`.
- Add structured operation name for diagnostics.
- Ensure command capture does not block raw PTY output path longer than bounded budget.

**Tests**

- Rust unit: `dispatch_send_input_records_v2_command_history`.
- Rust regression: `dispatch_send_input_fails_when_ui_input_persistence_fails`.
- Browser E2E: submit command, reload browser, clear localStorage/sessionStorage, command remains from DB.
- Browser E2E: simulate persistence failure, send button reports failure and command is not shown as saved.

#### TPV2-NATIVE-002 - Native raw output is durably captured

**Required behavior**

- Raw output stream writes v2 stream segments.
- Stream ranges are ordered and non-overlapping.
- Receiver lag records durable history gap.
- Segment corruption is detected and surfaced as degraded restore, not silent truncation.

**Implementation requirements**

- Keep raw capture loop separate from rendered snapshot loop.
- Add clear capture budgets:
  - max batch bytes;
  - max flush interval;
  - max pending queue.
- Add failpoint or test adapter for raw segment storage failure.
- Store capture semantics as `raw_vt_stream`.

**Tests**

- Rust bootstrap: `native_runtime_capture_persists_raw_output_to_v2`.
- New Rust bootstrap: `native_raw_output_survives_daemon_restart`.
- New persistence unit: `raw_output_segments_have_strict_non_overlapping_ranges`.
- New persistence unit: `corrupt_raw_segment_hydrates_as_visible_gap`.
- New browser E2E: command output visible after browser host restart and daemon restart.

#### TPV2-NATIVE-003 - Long history is paged end to end

**Required behavior**

- Initial history hydration can return first page only.
- UI stores `hasMoreSegments` and `nextEventSeq`.
- User can load all remaining history.
- No hard 1 MiB first-page ceiling becomes silent data loss.

**Implementation requirements**

- Keep `loadMorePaneHistory` in workspace core.
- Keep `Load history` action in workspace elements.
- Add auto-load on scroll-up when user reaches historical top.
- Add "more history available" affordance when auto-load is disabled or failed.
- Preserve source identity for restored pane:
  - live pane id;
  - source session id;
  - source pane id.

**Tests**

- SDK unit: `loadMorePaneHistory_merges_next_page`.
- SDK unit: `loadMorePaneHistory_preserves_source_identity_for_restored_session`.
- Workspace element unit: load-more button visible only with cursor.
- New browser E2E: create 500+ output segments, save, restore, click/load all pages, oldest marker appears.
- New browser E2E: scroll to top triggers auto-load, oldest marker appears without explicit button click.

#### TPV2-NATIVE-004 - Native restore is honest about process state

**Required behavior**

- Restored history is visually separated from new live output.
- UI states "restored history above" or equivalent boundary.
- New live process prompt is not confused with old process.
- Restored output never replays side effects like clipboard/title/OSC as live actions.

**Implementation requirements**

- Keep historical lines tagged with `data-line-source="history"`.
- Keep restore boundary line in DOM.
- Add machine-readable restore semantics to workspace state.
- Add UI state for "live process restarted".
- Do not auto-rerun commands during restore.

**Tests**

- Browser E2E: saved session restore shows historical output marker and boundary.
- Browser E2E: after restore, new live command output appears below boundary.
- Browser E2E: restored OSC/title output does not change current page title/clipboard.
- SDK unit: restored historical pane and live screen are separate read-model regions.

### 3.2 Command and Output History

#### TPV2-HISTORY-001 - Paste is journal input, not verified command history

**Required behavior**

- `SendPaste` is persisted as `terminal_paste_input`.
- Paste does not create verified command block.
- Paste does not appear as rerunnable command history.
- Future explicit single-command paste confirmation can create command history only with explicit user action.

**Implementation requirements**

- Keep `is_paste = true` mapping.
- Keep `command_text = None` for paste transactions.
- Add separate low-trust input journal view if needed.
- Add command history source policy enum if not already explicit enough.

**Tests**

- Persistence unit: `records_paste_as_journal_event_without_verified_command_history`.
- Runtime unit: `send_paste_records_ui_input_with_is_paste_true`.
- Browser E2E: paste multi-line script, command dock does not show the script.
- Browser E2E: paste secret-looking token, support/export redacted views do not expose it by default.

#### TPV2-HISTORY-002 - Command history is scoped correctly

**Required behavior**

- Per-session history does not leak into another session.
- Same command text in two sessions produces separate entries.
- Global command dock, if enabled later, must be opt-in and visibly scoped.

**Implementation requirements**

- Keep session id on command history entries.
- Keep pane id for pane-local history when present.
- Verify queries accept session id and do not accidentally use global fallback.

**Tests**

- Rust bootstrap: scope command history by session.
- SDK unit: hydrated command history belongs to selected session only.
- Browser E2E: two sessions, same command marker, switching sessions shows only relevant history.

#### TPV2-HISTORY-003 - Command output ranges are exact

**Required behavior**

- Command block output range points to exactly the command output, not next prompt/output.
- Byte ranges use half-open `[start, end)`.
- Event ranges use documented inclusive/exclusive convention consistently.

**Implementation requirements**

- Keep command block range model separate from command history entry.
- Add shell-specific confidence:
  - high: UI submit with output window;
  - medium: shell markers;
  - low: heuristic rendered parsing.
- Do not expose low-confidence command as one-click rerun without confirmation.

**Tests**

- Persistence unit: `command_output_byte_range_is_half_open`.
- New persistence unit: `command_block_output_excludes_next_command`.
- New runtime integration: two rapid commands, output ranges remain separate.
- Browser E2E: command details copy only selected command output.

#### TPV2-HISTORY-004 - Search/export/read models are derived and rebuildable

**Required behavior**

- Canonical journal remains source of truth.
- Search snippets are redacted derived data.
- Export raw transcript requires explicit approval.
- Support bundles are redacted and auditable by default.

**Implementation requirements**

- Keep command/search docs rebuildable from journal.
- Add rebuild command for derived indexes if missing.
- Add export request status table and UI diagnostics if not complete.

**Tests**

- Persistence unit: search documents are redacted.
- Persistence unit: raw export requires approval.
- E2E: support bundle generation does not include raw secret marker by default.
- E2E: derived index rebuild after deletion/repair produces same visible search results.

### 3.3 Saved Session Restore

#### TPV2-RESTORE-001 - Save is v2-first and publish-last

**Required behavior**

- If v2 save fails, no visible legacy saved session is published as restorable.
- If legacy publish fails after v2 succeeds, system either retries publish or marks v2 save as unpublished.
- User never sees a saved session that cannot be explained by v2 evidence unless explicitly marked legacy-only fallback.

**Implementation requirements**

- Extract save orchestration into a clear application service:
  - collect live topology/screen;
  - write v2 snapshot/evidence;
  - run restore drill or lightweight preflight;
  - publish legacy/API-facing saved session;
  - update manifest/status.
- Add status field:
  - `saving`;
  - `v2_recorded`;
  - `published`;
  - `failed`;
  - `legacy_only`.
- Avoid dual-write split across unrelated services.

**Tests**

- Runtime unit: v2 failure prevents legacy saved session publication.
- Runtime unit: legacy publish failure records unpublished v2 save state.
- Bootstrap E2E: save session during injected v2 failure, list saved sessions does not show broken row.
- Browser E2E: save button reports degraded/failure, not success, when v2 write fails.

#### TPV2-RESTORE-002 - Restore uses v2 history primary, snapshot fallback secondary

**Required behavior**

- Restore maps saved pane ids to new live pane ids.
- UI hydrates v2 pane history using source saved session id and source pane id.
- Snapshot fallback is used only when v2 history is absent, corrupt, or unsupported.
- Restore semantics explain which source won.

**Implementation requirements**

- Keep `mapSavedPaneIdsToLivePaneIds`.
- Persist evidence refs in restore semantics:
  - stream segment refs;
  - snapshot refs;
  - gap refs;
  - restore drill ref.
- Add fallback reason code when snapshot wins.

**Tests**

- SDK unit: restored historical panes hydrate from v2 before snapshot.
- SDK unit: snapshot fallback works when `getPaneHistory` unavailable.
- Browser E2E: v2 marker older than snapshot is visible after restore.
- Browser E2E: corrupt v2 segment creates visible degraded gap and snapshot fallback.

#### TPV2-RESTORE-003 - Restore drill is not optional for release confidence

**Required behavior**

- Save/restore path has an automated drill that proves the saved session can be hydrated.
- Restore drill writes durable result.
- Failed drill downgrades restore guarantee.

**Implementation requirements**

- Use lightweight drill for normal save.
- Use full drill for release/maintenance:
  - temp DB or isolated replay context;
  - hydrate topology;
  - hydrate pane histories;
  - validate high-water vectors;
  - validate no critical health records.
- Expose latest drill status through API.

**Tests**

- Persistence unit: failed restore drill downgrades plan.
- Runtime unit: saved session summary includes latest drill status.
- Browser E2E: saved session card shows degraded restore reason when drill failed.

### 3.4 Reliability, Fault Handling, and Diagnostics

#### TPV2-RELIABILITY-001 - Persistence faults become session health

**Required behavior**

- Raw output, rendered snapshot, topology snapshot, history gap, backend capability report failures update session health.
- Health reason is machine-readable: `HistoryPersistenceFault`.
- UI can show degraded state.

**Implementation requirements**

- Keep `CapturePersistenceDiagnostics`.
- Add durable health record in persistence layer where possible, not only in-memory session registry.
- Add event subscription propagation test.

**Tests**

- Runtime unit: capture persistence failure marks session health degraded.
- Runtime integration: subscription receives session health degraded after injected capture failure.
- Browser E2E: persistence fault banner appears.

#### TPV2-RELIABILITY-002 - Storage pressure is first-class

**Required behavior**

- `SQLITE_FULL`, WAL growth limit, temp space issues, and quota exhaustion produce explicit storage pressure events.
- System does not silently delete canonical history.
- User sees storage pressure and can run maintenance.

**Implementation requirements**

- Map SQLite full/disk I/O categories to persistence error domain.
- Persist storage pressure records.
- Add maintenance action:
  - checkpoint;
  - optimize;
  - vacuum backup;
  - retention preview;
  - user-approved prune.

**Tests**

- Persistence unit: storage full failpoint records pressure without mutation.
- Runtime integration: `SQLITE_FULL` during raw output capture degrades health.
- Browser E2E: storage pressure appears and terminal remains usable with clear degraded semantics.

#### TPV2-RELIABILITY-003 - Crash consistency is tested, not assumed

**Required behavior**

- Crash after input capture but before backend dispatch is safe.
- Crash after raw output write but before snapshot is safe.
- Crash during save does not publish broken saved session.
- Crash during restore does not corrupt canonical history.

**Implementation requirements**

- Add crash harness in `terminal-testing`:
  - start daemon;
  - inject marker/failpoint;
  - kill daemon/browser host;
  - restart;
  - assert DB state and UI state.
- Add failpoints behind test-only feature or env var.
- Record recovery diagnostics.

**Tests**

- Bootstrap E2E: `native_crash_after_input_capture_before_dispatch`.
- Bootstrap E2E: `native_crash_after_output_segment_before_snapshot`.
- Bootstrap E2E: `save_crash_before_publish_does_not_list_broken_session`.
- Browser E2E: browser host killed mid-command, history recovers after restart.

#### TPV2-RELIABILITY-004 - Single-writer is real

**Required behavior**

- Single writer executor uses its owned connection for v2 operations.
- Multi-step persistence operations can be atomic on that writer connection.
- No hot path reopens fresh DB connection per job unless intentionally read-only.

**Implementation requirements**

- Refactor facade executor to pass `&mut SqliteConnection` into real v2 repository methods.
- Split repositories:
  - write repositories require connection;
  - read repositories can open short-lived read connections.
- Add compile-level boundaries so v2 write methods cannot accidentally create fresh writer connection.

**Tests**

- Persistence unit: executor serializes jobs and uses same connection identity in test instrumentation.
- Persistence unit: multi-step v2 save rolls back atomically on injected second-step failure.
- Load test: concurrent command/output capture does not produce lock storm.

### 3.5 Zellij and Mux Persistence

#### TPV2-ZELLIJ-001 - Guarantee matrix is explicit

**Required behavior**

Zellij must never inherit native guarantees by backend name. It needs capability evidence:

| Zellij state | Process continuity | History source | Guarantee |
| --- | --- | --- | --- |
| Live zellij session attached | Preserved by zellij | rendered viewport/snapshots plus command input journal | `live_mux_attach` |
| Zellij session gone, our DB has history | Not preserved | v2 journal/snapshots | `visual_history_restore` |
| Import lacks rich surface | Unknown | limited snapshot | `history_degraded` |
| Capability probe fails | Unknown | none or stale | `unsupported_or_degraded` |

**Implementation requirements**

- Add backend capability report for zellij:
  - zellij version;
  - rich surface available;
  - rendered viewport snapshot;
  - scrollback/dump-screen availability;
  - targeted input support;
  - tab lifecycle support.
- Saved session semantics for zellij must include:
  - `preserves_process_state` only when live attach is proven;
  - `capture_semantics = rendered_viewport_snapshot` unless stronger proof exists.

**Tests**

- Rust zellij: capability report records version and supported surfaces.
- Node zellij: import surface exposes capability evidence.
- Browser zellij E2E: UI shows `live mux attach` when importing live zellij.
- Browser zellij E2E: UI shows degraded/visual restore when zellij session is gone.

#### TPV2-ZELLIJ-002 - Zellij command/output history is persisted honestly

**Required behavior**

- Commands sent through our UI into zellij are persisted in DB.
- Output visible from zellij rendered pane is captured as rendered history/snapshot evidence.
- Raw outer zellij PTY is not treated as one shell pane transcript.
- Paste remains non-rerunnable by default.

**Implementation requirements**

- Capture zellij pane subscriptions into v2 as rendered snapshots/deltas.
- Use source pane id mapping from zellij route to canonical pane id.
- Mark command blocks from UI submit as high-trust input, but output association as rendered evidence unless shell markers prove exact range.
- Add zellij restore evidence refs.

**Tests**

- Bootstrap zellij: import, send command, output snapshot persisted, restart daemon, history still visible.
- Browser zellij E2E: command history survives browser reload and daemon restart.
- Browser zellij E2E: paste marker visible but paste text not in command history.
- Persistence unit: zellij rendered capture cannot claim raw replay strategy.

#### TPV2-ZELLIJ-003 - Zellij save/restore has product-grade semantics

**Required behavior**

- Saving imported zellij session is allowed only with explicit zellij restore guarantee level.
- If live zellij still exists, restore/import can reconnect to live mux.
- If live zellij no longer exists, restore shows historical DB content and says process state is gone.
- Unsupported zellij actions are explicit and do not break session persistence.

**Implementation requirements**

- Decide saved-session route kind:
  - imported foreign live ref;
  - historical zellij saved session;
  - unsupported legacy zellij surface.
- Add restore path:
  - try live zellij attach by route/fingerprint;
  - if found, attach and hydrate DB history;
  - if not found, create historical pane view or native placeholder with visual history.
- Add UI degraded reason for zellij live session missing.

**Tests**

- Bootstrap zellij: save imported session, keep zellij alive, restore reattaches live and history visible.
- Bootstrap zellij: save imported session, kill zellij, restore shows historical visual history and degraded process continuity.
- Browser zellij E2E: rename/new tab before save, restore preserves topology evidence when possible.
- Browser zellij E2E: close unsupported path does not corrupt saved history.

### 3.6 Production UX and Polish

#### TPV2-UX-001 - Restore boundary and source badges

**Required behavior**

- User sees source of restored content:
  - v2 journal;
  - screen snapshot fallback;
  - zellij live attach;
  - degraded/gap.
- Boundary between restored and live output is visible but not noisy.
- Accessibility labels explain restored/degraded state.

**Implementation requirements**

- Add compact restore badge in terminal screen chrome.
- Add detailed tooltip or semantics panel.
- Add `data-restore-source` and `data-history-state` attributes for E2E.
- Avoid huge explanatory in-app text. Keep UI concise.

**Tests**

- Workspace element unit: restore badge labels each source state.
- Browser E2E: restored native session shows v2 source badge.
- Browser E2E: snapshot fallback shows snapshot fallback badge.
- Browser E2E: zellij live attach shows live mux badge.

#### TPV2-UX-002 - Large history ergonomics

**Required behavior**

- User can load older history without understanding cursors.
- UI does not jump unexpectedly.
- Loading failure is visible and retryable.
- Search can operate across loaded history and signal when older unloaded history may also match.

**Implementation requirements**

- Auto-load on scroll near top.
- Keep manual load button as fallback.
- Preserve scroll anchor when prepending history lines.
- Add "search loaded history" vs "search full persisted history" distinction if full DB search is not yet wired.

**Tests**

- Element unit: scroll anchor preserved after prepending history.
- Browser E2E: load older history keeps viewport near same logical line.
- Browser E2E: failed page load shows retry state.
- Browser E2E: search finds marker after loading older page.

#### TPV2-UX-003 - Command history controls are durable and honest

**Required behavior**

- Command dock uses DB-backed history after reload.
- Clear history either means:
  - clear local command dock view only, clearly labeled; or
  - create durable user-approved retention/delete request.
- It must not pretend canonical history is deleted if only UI cache is cleared.

**Implementation requirements**

- Rename or clarify command history clear semantics if currently local-only.
- Add durable command history hide/delete workflow if product requires it.
- Add audit record for destructive history action.

**Tests**

- Browser E2E: clear command dock, reload, expected behavior matches product semantics.
- Persistence unit: durable command delete request tombstones command history without deleting canonical journal unexpectedly.

### 3.7 Retention, Privacy, Export, and Maintenance

#### TPV2-DATA-001 - No silent canonical deletion

**Required behavior**

- Retention/pruning never directly deletes canonical history without audit.
- User-visible policy defines storage budget.
- Deleted/hid history leaves tombstone or request record.

**Implementation requirements**

- Separate legacy saved session pruning from v2 retention.
- Add retention preview API before deletion.
- Add chunked retention worker.
- Add durable audit rows.

**Tests**

- Persistence unit: parent canonical history rejects direct cascade deletion.
- Persistence unit: retention preview reports affected sessions/panes/bytes.
- E2E: user-approved prune reduces storage and restore semantics show pruned ranges as gaps.

#### TPV2-DATA-002 - Backup proves restore

**Required behavior**

- Backup is not successful until temp restore drill passes.
- WAL mode backup copies consistent DB state.
- Backup manifest records schema version, feature gates, checksums.

**Implementation requirements**

- Use `VACUUM INTO` baseline or SQLite Online Backup adapter.
- Run `quick_check` on backup.
- Run minimal restore hydrate from backup.
- Record backup drill result.

**Tests**

- Persistence integration: backup created while DB is in WAL mode can reopen and hydrate history.
- Bootstrap E2E: create session, write history, backup, restore temp DB, markers visible.
- Failure E2E: corrupt backup fails closed and does not mark backup as valid.

#### TPV2-DATA-003 - Redaction defaults are safe

**Required behavior**

- Support bundle and AI context do not include raw transcript by default.
- Raw export requires explicit user approval.
- Command hashes are local/keyed and not exported as stable dictionary target.

**Implementation requirements**

- Keep raw output classified.
- Keep support bundle manifest.
- Add redaction smoke markers:
  - password-like string;
  - token-like string;
  - path with username.

**Tests**

- Persistence unit: support bundle redacts sensitive markers.
- Browser E2E: support/export UI default artifact excludes secret marker.
- Integration: raw export blocked when health has critical corruption.

### 3.8 CI, Release Gates, and Observability

#### TPV2-CI-001 - CI matrix is explicit

**Required behavior**

PR cannot be considered ready unless critical matrix passes:

```powershell
cargo fmt --check
cargo test -p terminal-persistence
cargo test -p terminal-runtime
cargo test -p terminal-node
cargo test -p terminal-daemon-client
npm run check
cd apps/terminal-demo; npm run test
cd apps/terminal-demo; npm run smoke:browser
```

Additional Windows long lane:

```powershell
cargo test -p terminal-testing --test bootstrap_smoke daemon_native -- --nocapture
cargo test -p terminal-testing --test bootstrap_smoke saved_sessions -- --nocapture
cargo test -p terminal-testing --test bootstrap_smoke native_layout -- --nocapture
cargo test -p terminal-testing --test bootstrap_smoke zellij::import_surface -- --nocapture
cd apps/terminal-demo; npm run smoke:browser:foreign
```

**Implementation requirements**

- Split slow zellij jobs from normal smoke.
- Save screenshots/logs/session DB artifacts on failure.
- Use per-job timeouts that match Windows reality.
- Do not run one giant cargo command as the only signal.

**Tests**

- CI workflow validation test in `xtask`.
- Browser smoke lock test already exists, keep it.
- Add artifact existence check when browser smoke fails.

#### TPV2-CI-002 - Release gate is fail-closed

**Required behavior**

- If migration fails, v2 authoritative reads disabled.
- If restore drill fails, restore guarantee downgraded.
- If capability probe fails, backend guarantee downgraded.
- If SQLite runtime gate fails, app refuses production persistence mode.

**Implementation requirements**

- Add startup gate report.
- Expose gate status through handshake/capabilities.
- Add UI degraded reason when gate disables v2.

**Tests**

- Runtime unit: migration failure disables v2 reads.
- Browser E2E: startup gate failure shows degraded persistence status.
- Persistence unit: SQLite version gate fails for simulated unsupported runtime.

## 4. Phase Plan

### Closeout implementation summary

Status after commit `049d7c1abb3a6370a3300092b1d0b47d5a220b80`:

- Phase 1: completed for scoped PR guarantees.
  - `loadMorePaneHistory` carries `nextEventSeq`.
  - Restored panes preserve source identity.
  - Terminal screen has manual load-more and scroll-top auto-load with anchor preservation.
- Phase 2: completed for native saved sessions.
  - Save orchestration is extracted.
  - v2 evidence is persisted before legacy/API publish.
  - Regression tests prove v2 failure prevents publish.
- Phase 3: completed for scoped fault handling.
  - Executor has worker-owned connection support.
  - Capture failures persist durable health records.
  - Storage pressure, corruption and integrity downgrade paths are covered in persistence tests.
- Phase 4: completed for explicit zellij guarantees.
  - zellij rich surface advertises scrollback snapshot support.
  - zellij snapshot capture uses `dump-screen --full`.
  - zellij rendered snapshots persist by pane event cursor instead of projection sequence.
  - zellij E2E verifies rendered output history and paste policy.
  - zellij saved-session API remains explicitly unsupported rather than pretending native parity.
- Phase 5: completed for current real smoke gates.
  - Native browser smoke covers browser host restart recovery.
  - zellij bootstrap and browser foreign smoke cover real imported mux behavior.
  - Dedicated degraded browser smoke covers v2 pane-history failure during saved-session restore, snapshot fallback, diagnostic recording, and continued terminal usability.
- Phase 6: completed for current UI contract.
  - Restore boundary and partial-history messaging are present.
  - Load-more failure and retry states are represented in screen actions.
- Phase 7: completed locally.
  - Local Windows matrix is green.
  - GitHub Actions can still be blocked by maintainer approval, which is external to code.

### Phase 0 - Traceability and baseline hardening

**Goal**: make every remaining gap traceable to requirement and test.

**Tasks**

1. Add this document to PR and link from status doc.
2. Add a compact checklist table to `terminal-persistence-v2-status.md`.
3. Tag existing tests with requirement ids in names or comments.
4. Add missing regression tests for the original 7 review findings if any are not directly covered.
5. Add CI note: do not use one timeout-prone giant cargo command.

**Exit criteria**

- Every review finding maps to at least one committed test.
- `gh pr checks 5` passes after doc update.

### Phase 1 - Long-history UX and paging to product quality

**Goal**: large restored history is usable, not just technically pageable.

**Tasks**

1. Add auto-load on scroll-top for historical pane.
2. Preserve viewport anchor when prepending older history.
3. Add visible retry state when history page load fails.
4. Add browser E2E with forced multiple pages.
5. Add SDK test for restored saved session source mapping across pages.

**Files likely involved**

- `sdk/packages/workspace-core/src/services/session-command-service.ts`
- `sdk/packages/workspace-elements/src/elements/terminal-screen-element.ts`
- `apps/terminal-demo/scripts/browser-smoke.mjs` or new dedicated E2E script

**Acceptance tests**

```powershell
npm run check
cd apps/terminal-demo; npm run smoke:browser
cd apps/terminal-demo; node ./scripts/browser-persistence-long-history-smoke.mjs
cd apps/terminal-demo; node ./scripts/browser-persistence-degraded-smoke.mjs
```

### Phase 2 - Save orchestration v2-first

**Goal**: no broken visible saved session if v2 evidence write fails.

**Tasks**

1. Extract `SavedSessionSaveOrchestrator`.
2. Move v2 snapshot/evidence write before legacy/API publish.
3. Add save status/manifest state.
4. Add rollback/cleanup path for partial publish failure.
5. Add failpoints for v2 write failure and legacy publish failure.

**Files likely involved**

- `crates/terminal-runtime/src/sessions/saved_sessions_service/save.rs`
- `crates/terminal-persistence/src/legacy/v2_facade/*`
- `crates/terminal-persistence/src/v2/*`

**Acceptance tests**

```powershell
cargo test -p terminal-runtime saved_session
cargo test -p terminal-persistence restore_plan
cargo test -p terminal-testing --test bootstrap_smoke saved_sessions -- --nocapture
```

### Phase 3 - Single-writer and fault durability

**Goal**: single-writer architecture is clean and failure evidence survives restart.

**Tasks**

1. Refactor v2 facade executor so jobs use worker-owned `&mut SqliteConnection`.
2. Split write repositories from read repositories.
3. Persist history persistence faults as durable health records where possible.
4. Add storage pressure domain errors and records.
5. Add worker crash/restart recovery tests.

**Files likely involved**

- `crates/terminal-persistence/src/legacy/v2_facade/executor.rs`
- `crates/terminal-persistence/src/db/executor.rs`
- `crates/terminal-runtime/src/sessions/runtime/capture/*`

**Acceptance tests**

```powershell
cargo test -p terminal-persistence executor
cargo test -p terminal-runtime capture_persistence
cargo test -p terminal-testing --test bootstrap_smoke daemon_native -- --nocapture
```

### Phase 4 - zellij persistence parity by explicit guarantees

**Goal**: zellij becomes production-usable without pretending it is native.

**Tasks**

1. Add zellij capability report persistence.
2. Capture zellij rendered pane updates into v2 as rendered history evidence.
3. Add zellij saved-session semantics.
4. Add live attach vs historical restore split.
5. Add zellij kill/restart restore E2E.
6. Add UI source badges for zellij live/historical/degraded.

**Files likely involved**

- `crates/terminal-backend-zellij/*`
- `crates/terminal-runtime/src/sessions/runtime/capture/*`
- `crates/terminal-node/src/tests/zellij.rs`
- `crates/terminal-testing/tests/bootstrap_smoke/zellij/*`
- `apps/terminal-demo/scripts/browser-foreign-backends-smoke.mjs`

**Acceptance tests**

```powershell
cargo test -p terminal-backend-zellij --lib
cargo test -p terminal-node zellij -- --nocapture
cargo test -p terminal-testing --test bootstrap_smoke zellij::import_surface -- --nocapture
cd apps/terminal-demo; npm run smoke:browser:foreign
```

### Phase 5 - Crash, restart, and storage-pressure E2E

**Goal**: reliability is proven under ugly real scenarios.

**Tasks**

1. Add daemon kill/restart harness around active command.
2. Add browser host restart during active command.
3. Add save-session crash before publish.
4. Add storage full failpoint.
5. Add WAL growth/maintenance scenario.
6. Persist artifacts on failure for debugging.

**Acceptance tests**

```powershell
cargo test -p terminal-testing --test bootstrap_smoke persistence_crash -- --nocapture
cd apps/terminal-demo; node ./scripts/browser-persistence-chaos-smoke.mjs
```

### Phase 6 - Product UX completion

**Goal**: user can understand what was restored and what was not.

**Tasks**

1. Restore source badge.
2. Degraded persistence banner.
3. Storage pressure affordance.
4. Command history clear semantics cleanup.
5. Support/export redaction UX.
6. Accessibility checks for restored/degraded labels.

**Acceptance tests**

```powershell
npm run check
cd apps/terminal-demo; npm run test
cd apps/terminal-demo; npm run smoke:browser
```

### Phase 7 - Release gate and docs

**Goal**: PR can be merged with clear release confidence.

**Tasks**

1. Update status doc with final matrix.
2. Add manual runbook for native and zellij persistence.
3. Add troubleshooting doc:
   - history missing;
   - restore degraded;
   - zellij unsupported;
   - storage pressure;
   - corrupted segment.
4. Add CI checklist to PR description.
5. Verify GitHub checks and local matrix.

**Acceptance**

- Working tree clean.
- Branch pushed.
- `gh pr checks 5` green.
- Status doc lists exact commit and test commands.

## 5. E2E Test Suite to Add

### 5.1 Rust bootstrap E2E

Add new module:

```text
crates/terminal-testing/tests/bootstrap_smoke/persistence_v2.rs
```

Suggested tests:

1. `native_long_history_pagination_survives_daemon_restart`
   - create native session;
   - emit 500+ deterministic lines;
   - save;
   - restart daemon;
   - hydrate first page;
   - load all pages;
   - assert first, middle, last markers.

2. `native_restore_uses_v2_history_before_snapshot_fallback`
   - write output before latest snapshot;
   - save;
   - restore;
   - assert older v2-only marker appears.

3. `native_restore_snapshot_fallback_when_v2_history_corrupt`
   - corrupt one segment through test helper;
   - restore;
   - assert visible gap and snapshot fallback.

4. `save_v2_failure_does_not_publish_legacy_session`
   - enable failpoint;
   - save;
   - assert list saved sessions excludes broken row.

5. `save_crash_before_publish_recovers_without_orphan`
   - kill daemon after v2 write before publish;
   - restart;
   - assert no broken visible saved session or status is `unpublished`.

6. `history_fault_degrades_session_health_subscription`
   - inject capture write error;
   - subscribe to health;
   - assert `HistoryPersistenceFault`.

7. `storage_pressure_does_not_delete_history_silently`
   - simulate storage full;
   - assert pressure record and prior history still restores.

8. `multi_session_same_command_history_isolated`
   - run same command in two sessions;
   - assert per-session history isolation after restart.

### 5.2 Zellij Rust E2E

Extend:

```text
crates/terminal-testing/tests/bootstrap_smoke/zellij/
```

Suggested tests:

1. `zellij_import_persists_capability_report`
   - spawn zellij;
   - import;
   - assert capability report has zellij version and rendered snapshot support.

2. `zellij_command_history_survives_daemon_restart`
   - import live zellij;
   - send command through our API;
   - restart daemon;
   - re-import or restore view;
   - assert command history.

3. `zellij_rendered_output_restores_as_rendered_not_raw`
   - capture visible marker;
   - inspect restore plan;
   - assert `capture_semantics = rendered_viewport_snapshot`.

4. `zellij_live_attach_preserves_process_state_when_session_alive`
   - start long-running zellij command;
   - restart browser/daemon;
   - attach live;
   - assert process continuity label.

5. `zellij_missing_live_session_restores_visual_history_only`
   - save/import;
   - kill zellij;
   - restore;
   - assert no process continuity claim.

### 5.3 Browser E2E scripts

Add dedicated scripts instead of overloading one giant smoke:

```text
apps/terminal-demo/scripts/browser-persistence-long-history-smoke.mjs
apps/terminal-demo/scripts/browser-persistence-chaos-smoke.mjs
apps/terminal-demo/scripts/browser-zellij-persistence-smoke.mjs
apps/terminal-demo/scripts/browser-persistence-degraded-smoke.mjs
```

#### browser-persistence-long-history-smoke

Must verify:

- command output with many lines;
- save session;
- restore saved session;
- v2 history source;
- load-more button or auto-load;
- oldest marker visible;
- search finds loaded old marker;
- copy visible output includes restored marker.

#### browser-persistence-chaos-smoke

Must verify:

- command before browser host restart persists;
- command before daemon restart persists;
- active command during restart creates either output or clear gap;
- no duplicate command history entries after reconnect;
- health/degraded state visible when injected fault happens.

#### browser-zellij-persistence-smoke

Must verify:

- import zellij;
- send command;
- command history survives browser reload;
- command/output history survives daemon restart;
- paste does not enter command history;
- live attach badge appears when zellij still alive;
- visual restore/degraded badge appears when zellij gone.

#### browser-persistence-degraded-smoke

Must verify:

- gateway-induced v2 pane-history failure during saved-session restore;
- gateway-induced `storage_pressure` failure during command dispatch;
- snapshot fallback is used instead of losing restored context;
- `saved_pane_history_hydration_failed` diagnostic is visible in workspace state;
- `storage_pressure` diagnostic is visible in workspace state and browser notices;
- restored history and restore boundary still render in DOM;
- terminal remains usable after both degradations.

## 6. Suggested Implementation Order

Do not start with zellij. Finish native and save orchestration first, because zellij depends on the same restore semantics.

Recommended order:

1. Phase 1 - long-history UX and tests.
2. Phase 2 - v2-first save orchestration.
3. Phase 3 - single-writer and durable fault records.
4. Phase 5 - crash/restart and storage pressure tests for native.
5. Phase 4 - zellij explicit guarantee parity.
6. Phase 6 - UX completion.
7. Phase 7 - release gates and docs.

Why this order:

- Native path is the foundation.
- Save orchestration must be correct before zellij saved sessions.
- Fault durability must be in place before chaos tests.
- Zellij should reuse the guarantee model, not invent a separate one.

## 7. "100%" Acceptance Checklist

Use this checklist before calling the feature complete.

- [x] Native command history persists across browser reload, browser host restart, daemon restart.
- [x] Native output history persists across browser reload, browser host restart, daemon restart.
- [x] Saved native session restore hydrates v2 history before snapshot fallback.
- [x] Long history can be fully loaded through pages.
- [x] Scroll-up auto-load works and preserves viewport anchor.
- [x] Paste is durable journal input but not verified command history.
- [x] Save session is v2-first and publish-last.
- [x] Partial save failure does not publish broken saved session.
- [x] Persistence capture fault degrades session health and is visible to tests.
- [x] Storage pressure is visible to persistence diagnostics and does not silently delete canonical history.
- [x] Single-writer executor owns and reuses a worker connection for connection-aware jobs.
- [x] zellij capability evidence is durable and includes live-process/scrollback booleans.
- [x] zellij command history persists through the verified UI/browser paths.
- [x] zellij rendered output restore is labeled as rendered evidence, not raw replay.
- [x] zellij live attach and unsupported saved-session restore are separate product states.
- [x] Browser E2E covers native restore and history behavior in the main smoke.
- [x] Dedicated browser E2E covers native long-history restore, v2 paging cursors, load-more, and DOM history/boundary rendering.
- [x] Dedicated browser E2E covers degraded restore when v2 pane-history hydration fails and snapshot fallback must keep the terminal usable.
- [x] Dedicated browser E2E covers browser-facing storage-pressure diagnostics during command dispatch and proves the next command still works.
- [x] Browser E2E covers native browser host restart recovery.
- [x] Browser E2E covers zellij persistence semantics through foreign smoke.
- [x] Degraded/fault behavior is covered by Rust persistence/runtime tests plus a real browser degraded restore smoke.
- [x] CI has separate fast and slow Windows lanes documented.
- [x] Status doc includes final commit, matrix, and known limitations.

## 8. Non-goals for This 100%

These should not block 100% for the current persistence feature unless product scope changes:

- Native process checkpoint/resurrection after OS/process restart.
- Treating zellij outer raw PTY stream as exact inner shell transcript.
- Full encrypted-at-rest rollout if schema and gates are already prepared.
- Compression of cold history segments.
- Cloud sync or multi-device history.
- AI semantic command analysis beyond redacted derived context.

## 9. Future Hardening Backlog

These are no longer blockers for the current PR guarantees, but they are the right next investments if the product scope expands.

1. zellij historical saved-session restore.
   - Add a new product state for "zellij session gone, DB history visible only".
   - This should not reuse native saved layout semantics blindly.
   - Expected effort: `1.5k-3.5k` changed lines.
2. Full connection-aware v2 facade migration.
   - The executor now owns a reusable writer connection; more v2 write repositories can be moved behind connection-aware ports over time.
   - Expected effort: `1k-2.5k` changed lines.
3. Crash harness with deterministic failpoints.
   - Kill daemon between v2 save and publish, during raw segment write, and during restore.
   - Expected effort: `1.5k-3k` changed lines.
