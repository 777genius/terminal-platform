# Terminal Persistence v2 - Implementation Plan

**Статус**: implementation blueprint
**Дата**: 2026-04-29
**Решение**: делать `Terminal Persistence v2` через Diesel ORM, journal-first, local-first, Windows-first
**Связанный ресерч**: [deep-dive-terminal-history-journal-research.md](./deep-dive-terminal-history-journal-research.md)

## Короткий вывод

Нужно реализовать не "сохранение экрана", а отдельный durable persistence layer:

```text
PTY / ConPTY / zellij / tmux
  -> capture events
  -> single durable writer
  -> Diesel + SQLite canonical journal
  -> command blocks
  -> screen snapshots
  -> outbox jobs
  -> restore/search/export/AI read models
```

Самое важное правило:

> Canonical truth - append-only journal and stream segments. Screen snapshots, command dock, search indexes, exports and AI context are derived read models.

Оценка выбранной архитектуры: 🎯 10   🛡️ 9   🧠 8

Примерный общий объем первой production-ready версии: `14k-28k` строк поэтапно.

## Что усилено после архитектурного аудита

Я отдельно пересмотрел места с меньшей уверенностью и поправил план так, чтобы он был ближе к реальной реализации в этом репозитории.

Самые важные исправления:

1. **Diesel executor теперь описан compile-realistic closure-based API**
   Старый пример с trait object и associated `Output` был слишком псевдокодным. Реализация должна отправлять в worker boxed closure, а typed `oneshot` остается внутри `execute<T>`.

2. **Добавлен `terminal_topology_snapshots`**
   Без отдельного topology/layout snapshot нельзя надежно восстановить tabs/splits/focus. Одних `terminal_panes` недостаточно, потому layout tree and split ratios являются snapshot state.

3. **`terminal_journal_events` получил non-null event scope**
   Nullable `pane_id` в unique constraint на SQLite опасен: `NULL` ломает ожидания уникальности. Теперь event scope задается через `event_scope_kind` + `event_scope_id`.

4. **Добавлен `terminal_delivery_offsets` в базовую schema**
   Reconnect/ack/replay был в итерациях, но отсутствовал в initial schema. Это исправлено, потому доставка в браузер и durable journal должны проектироваться вместе.

5. **Outbox claim уточнен под SQLite**
   На первом этапе один worker достаточно надежен. Если workers станут параллельными, claim должен идти через `immediate_transaction` and conditional update, not optimistic prose.

6. **Backpressure policy стал явным**
   PTY reader нельзя блокировать на DB. Если queue переполнена, система должна перейти в degraded state and persist/emit gap marker when possible.

7. **Dependency section обновлен под текущие проверенные версии**
   `cargo search/info` на 2026-04-29 показывает `diesel 2.3.8`, `diesel_migrations 2.3.2`, `libsqlite3-sys 0.37.0`, `blake3 1.8.5`.

8. **Уточнена bundled SQLite стратегия**
   У Diesel нет отдельной публичной feature `sqlite-bundled`. Для Windows стабильности оставляем единый `libsqlite3-sys` с `bundled` через workspace dependency и проверяем дерево через `cargo tree -e features -i libsqlite3-sys@0.37.0`.

9. **Добавлен `terminal_stream_cursors`**
   `terminal_panes.last_event_seq` остается read-model полем, но не должен быть единственным allocator state. После restart writer должен атомарно продолжать event sequence allocation без коллизий.

10. **Добавлен `terminal_history_gaps`**
   Gap markers не должны жить только как свободный JSON в journal. Нужна отдельная компактная таблица для UI, diagnostics, restore drill and support reports.

11. **Добавлен `terminal_restore_drills`**
   Restore drill должен оставлять durable audit trail. Иначе нельзя уверенно отвечать, какие сессии реально восстановимы после обновления/краша.

12. **Checksum policy стал явным**
   Все stream/snapshot/topology payloads получают `checksum_algorithm` + `checksum`. Baseline: `blake3` over stored bytes; для будущего encryption это будет storage checksum, not plaintext leakage.

13. **Добавлены durability profiles**
   Для требования "история не должна теряться" production default должен быть `reliable_history`: writer connection uses `synchronous = FULL`, bounded batching and command-boundary barriers. `NORMAL` остается performance profile, not reliability default.

14. **Добавлены retention/maintenance правила**
   У "полной истории" должен быть explicit policy: по умолчанию не удалять raw history silently, но иметь quotas, user-visible pressure, WAL checkpoints, `PRAGMA optimize`, incremental vacuum and deletion audit.

15. **Search/FTS5 отложен в derived redacted index**
   FTS не должен индексировать raw terminal stream. V1 search работает по command blocks and small redacted snippets; FTS5 добавляется later as rebuildable derived index.

16. **zellij/tmux стали backend adapters with guarantee matrix**
   Для mux нельзя обещать ту же точность, что у native PTY, если нет shell integration. План теперь требует per-backend capture strategy, exact capability detection and visible restore guarantee.

17. **Migration safety усилена**
   Startup migration теперь описана как guarded flow: app id check, embedded migrations, identity row, migration audit, no silent destructive rollback.

18. **Добавлен global commit sequence**
   Per-pane sequence недостаточно для multi-pane restore. Теперь session-level atomic commits фиксируются в `terminal_commit_log`, а `terminal_session_cursors` выделяет `commit_seq`.

19. **Topology snapshot получил high-water vector**
   Один `high_water_event_seq` не описывает состояние нескольких panes. Topology/session snapshots должны хранить `pane_high_water_json` and `high_water_commit_seq`.

20. **Capture ingestion стал retry-safe**
   Добавлен `terminal_capture_receipts`, чтобы повтор доставки capture event/batch после reconnect/crash не создавал дубли.

21. **Command history отделена от command blocks**
   `terminal_command_blocks` описывает lifecycle команды, а `terminal_command_history_entries` обслуживает command dock/autocomplete/rerun history per session/pane/global policy.

22. **Replay sandbox стал явным**
   Restore/replay не должен выполнять terminal side effects. OSC52 clipboard, title changes, hyperlinks, shell integration and prompt-injection-like output are inert data during historical replay.

23. **Compression policy уточнена**
   V1 остается raw SQLite BLOB. Zstd допускается как later codec for cold/large segments with explicit `uncompressed_byte_len`, `stored_byte_len`, codec metadata and checksum over stored bytes.

24. **Canonical FK policy исправлена**
   `terminal_commit_log` теперь append-only anchor. Canonical history rows must not cascade-delete through session, pane or commit refs; those refs use `ON DELETE RESTRICT` / default no-action semantics.

25. **ID policy стала явной**
   Для DB rows используем app-generated UUIDv7 strings through `uuid 1.23.1` with `v7` feature. Это дает ordered-ish IDs without relying on SQLite rowid as domain identity.

26. **Command history privacy усилена**
   Command dock/autocomplete больше не хранит только raw `command_text`. Добавлены display/redacted text fields, sensitivity classification and rerun policy.

27. **Browser/client identity вынесена в таблицу**
   `terminal_delivery_offsets.client_id` должен ссылаться на durable `terminal_clients`, чтобы reconnect/ack state не был случайной строкой из браузера.

28. **Export/delete workflows стали auditable**
   Добавлены export/delete request tables. Экспорт, удаление, crypto erase and retention pruning не должны быть hidden side effects without audit trail.

29. **Single-writer guarantee поднята до process level**
   Добавлен `terminal_writer_generations`: writer identity, lease/heartbeat and stale-writer recovery. Один mpsc worker внутри процесса не защищает от второго app process.

30. **Outbox lease модель усилена**
   Outbox claim теперь должен иметь `claimed_until_ms`, `lease_token`, dedupe key and stale claim recovery, иначе worker crash can leave jobs stuck.

31. **Clock drift/jump audit добавлен**
   Terminal history mixes wall-clock and monotonic ordering. Добавлен `terminal_clock_anchors`, чтобы диагностировать system time jumps and restore ordering issues.

32. **Versioned JSON payload contract добавлен**
   Все `payload_json` больше не "просто JSON". Добавлены `terminal_payload_schemas`, schema ids, schema semver and payload validation rules через typed Rust structs + schemars-generated schemas.

33. **Alternate screen/TUI политика стала явной**
   Terminal output is not only line scrollback. Добавлен режим normal/alternate buffer, frame coalescing for TUI apps and restore rules for vim/top/zellij-like screens.

34. **Outbox nullable dedupe исправлен**
   Inline `UNIQUE(dedupe_key)` заменен на partial unique index `WHERE dedupe_key IS NOT NULL`, чтобы намерение было точным и переносимым в тестах.

35. **Integrity check workflow добавлен**
   Restore drills проверяют восстановимость сессий, но нужна еще проверка самой БД: `PRAGMA quick_check`, `foreign_key_check`, invariant SQL and projection drift checks. Добавлен durable `terminal_integrity_checks`.

36. **Support bundle workflow добавлен**
   Диагностика должна быть redacted and auditable. Добавлен `terminal_support_bundles`, чтобы support reports не превращались в случайный raw transcript export.

37. **Parser/projection upgrade policy добавлена**
   `parser_version` теперь не просто строка. План фиксирует invalidation/rebuild rules for snapshots/search/command projections after parser upgrades.

38. **Command hash privacy уточнена**
   Хэш команды может быть dictionary-attack vector. Command dedupe/search hashes should be keyed/local where possible and never exported as raw stable fingerprint by default.

39. **Live backup policy добавлена**
   При WAL нельзя считать обычную копию `.db` полноценным backup. План теперь требует SQLite Online Backup API / `VACUUM INTO` path, durable backup records, manifest, checksum and post-backup integrity check.

40. **Storage pressure стал first-class режимом**
   `SQLITE_FULL`, большой WAL, нехватка temp space and quota pressure не должны выглядеть как обычный `Query` error. Добавлены storage-pressure events, degraded writer mode, UI diagnostics and explicit no-silent-delete rule.

41. **Schema constraints policy добавлена**
   Текстовые `status/kind/policy` поля больше не остаются free-form по умолчанию. Stable domains get Rust enums + DB `CHECK`; extension domains stay text but must be validated and documented.

42. **SQLite runtime gate усилен**
   Для packaged Windows build план требует bundled SQLite through `libsqlite3-sys 0.37.0`, which docs.rs currently maps to SQLite `3.51.3`. Это важно из-за официально описанного WAL-reset bug in older SQLite lines.

43. **Raw-vs-rendered capture semantics добавлена**
   Native ConPTY дает raw VT stream, а zellij/tmux часто дают rendered viewport/scrollback. План теперь запрещает replay-ить rendered surface как raw terminal bytes and requires explicit `capture_semantics`.

44. **Backend capability evidence стала durable**
   Restore guarantee больше не вычисляется по имени backend. Добавлен durable capability report, где фиксируются version, probe result, capture strategy and confidence.

45. **Data health/quarantine workflow добавлен**
   Если checksum/snapshot/parser/projection ломается, restore не должен либо падать целиком, либо молча пропускать данные. Добавлен durable health record with quarantine/degraded actions.

46. **Command trust model стал источниковым, а не текстовым**
   Высокое доверие дают UI submit, direct pane launch and verified shell markers. Rendered mux output and heuristic parsing cannot create rerunnable command blocks without confirmation.

47. **Migration rollout переведен в expand/contract policy**
   Опасные destructive migrations должны идти отдельной cleanup-фазой после dual-read/dual-write verification, backup and restore drills.

48. **Restore guarantee taxonomy добавлена**
   `Rich history`, `Basic history`, `Visual restore only` and `History degraded` теперь имеют строгие условия, а не являются маркетинговыми словами.

49. **Performance/storage budgets добавлены**
   Writer batching, queue depth, restore time, WAL size and DB growth получили target budgets and failure actions.

50. **Privacy data classification добавлена**
   Raw output, command text, cwd, user-agent hashes, support bundles and AI context теперь имеют явные data classes and default handling.

51. **Encryption/key management стал конкретнее**
   Добавлен Windows-first key hierarchy plan: DB key protected by OS credential store/DPAPI capability, later SQLCipher/envelope decision, `zeroize`/`secrecy` for key material handling.

52. **Feature gates and rollout controls добавлены**
   v2 persistence, mux capture, compression, raw export and encryption должны включаться через explicit gates with rollback behavior.

53. **Read-path/WAL starvation policy добавлена**
   Restore/search/support reads теперь не могут держать долгую read transaction без бюджета. План требует paged reads, progressive restore and no `await` while holding SQLite statements/transactions.

54. **MVP cutline зафиксирован**
   Чтобы scope не расползся, план теперь разделяет production MVP, reliability gate and later advanced tracks. Encryption, compression and mux structured capture остаются gated tracks.

55. **Windows mux support boundary уточнен**
   Zellij has official Windows binary, but still needs capability probing. Tmux on Windows is WSL/MSYS2-specific unless a separately probed native-compatible mux is integrated.

56. **Sequence domains разведены явно**
   `event_seq`, `byte_seq`, `commit_seq` and `frame_seq` больше не смешиваются под общим `seq`. Это критично для точного restore, command output ranges, gap markers, search documents and replay after snapshot.

57. **Range boundary rules зафиксированы**
   `event_seq` ranges inclusive, `byte` ranges half-open `[low, high)`, optional ranges must be either fully `NULL` or fully filled. Это снижает off-by-one баги в replay, export and command output extraction.

58. **Canonical FK policy ужесточена**
   Canonical history tables не должны каскадно удаляться от `terminal_sessions`/`terminal_panes`. Direct delete parent rows must fail closed; deletion/pruning goes through request + tombstone + chunked service flow.

59. **Protocol semantics rollout привязан к evidence**
   Текущий `SavedSessionRestoreSemantics.replays_saved_screen_buffers = false` остается до v2 hydrate/replay/drill evidence. API v2 должен добавить machine-readable guarantee fields without breaking old clients.

60. **Mux capture перепроверен по свежим docs и понижен до evidence, not truth**
   Zellij `subscribe` streams rendered pane updates, `dump-screen` dumps scrollback, tmux `capture-pane` is also snapshot/scrollback evidence. Это полезно для hydration/reconciliation, но не равно raw VT stream и не должно получать `Rich history` без отдельного raw/structured proof.

61. **Restore read path переведен в two-phase UX**
   Большие истории нельзя грузить одним буфером. Restore должен сначала показать nearest snapshot/topology, потом догружать historical pages по курсорам, закрывая SQLite cursor before async/browser streaming.

62. **Live boundary стал product invariant**
   После native restore новый процесс не является старым процессом. UI должен явно отделять restored historical region от new live output and prompt, иначе пользователь поверит, что process state survived.

63. **Command confidence gates стали строже**
   UI submit and verified shell markers create high-trust command blocks. Raw typed input, rendered mux output and heuristic prompt parsing can create only low/medium-confidence records and cannot be auto-rerun without confirmation.

64. **Backup/WAL wording ужесточен**
   В production API не должно быть helper, который копирует только `.db`. MVP backup path is `VACUUM INTO` with manifest and post-backup `quick_check`; Online Backup API остается later adapter if incremental backup is required.

65. **Capability reports стали versioned and expiring**
   Backend guarantee нельзя кэшировать навсегда. Zellij/tmux/native capability rows expire after backend version change, binary path change, config change or probe failure.

66. **Low-confidence areas теперь имеют отдельные acceptance tests**
   Zellij/tmux fidelity, alternate screen/TUI restore, command source trust, large restore paging, WAL backup and storage pressure each get explicit test bullets. Это снижает риск, что сложные зоны останутся только архитектурным текстом.

67. **Table-class ownership должен быть проверяемым**
   Canonical, derived, audit, ephemeral and external-artifact tables должны иметь явные FK/delete rules. План теперь требует не только общий принцип, а matrix and tests, чтобы `CASCADE` случайно не попал в canonical history.

68. **Добавлена трассировка требований к реализации**
   Ключевые требования теперь должны проверяться не общими словами, а через matrix: requirement -> schema/API section -> iteration -> acceptance test. Это защищает от ситуации, когда БД сделана, но protocol/UI не показывают историю.

69. **Protocol transition привязан к текущему коду**
   В репозитории `SavedSessionRestoreSemantics` сейчас boolean-only, а `protocol_mapping.rs` ставит `replays_saved_screen_buffers = false`. План фиксирует, что v2 semantics добавляются рядом с legacy bools and must stay backward-compatible.

70. **Legacy snapshot pruning отделен от v2 retention**
   Текущий v1 умеет `prune_saved_sessions` через прямой delete snapshot rows. Для v2 это недопустимо как модель для canonical history: pruning becomes audited retention workflow, not count-based delete.

71. **Backend capability gap стал явной work item**
   `BackendCapabilities.raw_output_stream` уже есть в API, но native path must not set/claim it until durable capture writer is wired. План теперь требует separate capability proof before any `RichHistory` label.

72. **Browser/localStorage история исключена из core semantics**
   Command dock может кешироваться в UI, но authoritative command history belongs to DB rows. После restart app/browser command history must come from `terminal_command_history_entries`, not localStorage.

73. **Release gates стали fail-closed**
   Если migration, SQLite runtime gate, restore drill, backup smoke or capability probe fails, feature gate must disable authoritative v2 reads instead of exposing overpromised history.

74. **Diesel table-width risk закрыт явно**
   `cargo info diesel` на 2026-04-29 подтверждает `32-column-tables`, `64-column-tables` and `128-column-tables`. Текущий planned max table width is `27` columns, so `32-column-tables` is enough now, but CI must count table columns and fail before Diesel macro limits surprise implementation.

75. **Schema generation workflow добавлен как gate**
   `schema.rs` не должен редактироваться руками без проверки. План теперь требует `diesel print-schema`/checked generated schema, `diesel.toml` and CI diff, чтобы migrations and Rust models did not drift.

76. **Core vs derived table rollout разделен четче**
   Не все `CREATE TABLE` blocks должны попасть в PR 1. Core journal/session tables идут first; search documents, AI/context, compression/encryption extensions remain derived/gated tracks unless MVP acceptance explicitly needs them.

## Риск-регистр по слабым местам

| Риск | Уверенность | Надежность выбранного решения | Сложность | Решение |
| --- | --- | --- | --- | --- |
| Diesel + existing `rusqlite` link to SQLite | 🎯 8 | 🛡️ 8 | 🧠 7 | Проверить `cargo tree -i libsqlite3-sys`, временно держать `rusqlite` только в `legacy`, v2 writes только Diesel |
| Sync Diesel inside async runtime | 🎯 10 | 🛡️ 9 | 🧠 7 | Single named persistence worker thread with boxed closures |
| Session-level journal event uniqueness | 🎯 10 | 🛡️ 10 | 🧠 4 | Non-null `event_scope_kind/event_scope_id`, not nullable unique key |
| Restore topology fidelity | 🎯 9 | 🛡️ 9 | 🧠 7 | `terminal_topology_snapshots` with versioned JSON and checksum |
| Multi-pane atomic restore consistency | 🎯 9 | 🛡️ 10 | 🧠 8 | Session-level `terminal_commit_log` + pane high-water vectors |
| Stream sequence allocation after restart | 🎯 9 | 🛡️ 9 | 🧠 7 | `terminal_stream_cursors` owns next `event_seq`/`byte_seq`, `terminal_panes.last_event_seq` is denormalized |
| Mixed byte/event sequence semantics | 🎯 10 | 🛡️ 10 | 🧠 6 | Separate `event_seq`, `byte_seq`, `commit_seq`, `frame_seq`; never reuse one column for multiple domains |
| Command output range maps to wrong payload | 🎯 9 | 🛡️ 9 | 🧠 7 | Command blocks store canonical event range and optional byte range for raw extraction |
| Off-by-one in replay/export ranges | 🎯 9 | 🛡️ 10 | 🧠 5 | Event ranges are inclusive, byte ranges are half-open; invariant tests cover both |
| Accidental parent delete removes canonical history | 🎯 10 | 🛡️ 10 | 🧠 6 | Canonical tables use `ON DELETE RESTRICT` from sessions/panes; explicit delete service writes tombstones |
| Duplicate capture after retry/reconnect | 🎯 8 | 🛡️ 9 | 🧠 7 | `terminal_capture_receipts` with source event hashes |
| PTY output backpressure | 🎯 8 | 🛡️ 8 | 🧠 8 | Bounded queue, visible gap markers, never block live PTY indefinitely |
| Persisted gap visibility | 🎯 9 | 🛡️ 9 | 🧠 6 | `terminal_history_gaps` plus journal gap events |
| Restore drill auditability | 🎯 9 | 🛡️ 10 | 🧠 6 | `terminal_restore_drills` records each verification result |
| Durability vs terminal responsiveness | 🎯 8 | 🛡️ 10 | 🧠 8 | Default `reliable_history` for writer, explicit `performance_history` opt-in |
| DB growth/WAL bloat | 🎯 8 | 🛡️ 9 | 🧠 8 | Retention policies, maintenance runs, no silent raw-history deletion |
| Search index correctness/privacy | 🎯 8 | 🛡️ 9 | 🧠 8 | Search index is redacted derived data, rebuildable from canonical journal |
| Outbox parallel workers | 🎯 8 | 🛡️ 8 | 🧠 8 | v1 single worker, later `immediate_transaction` conditional claim |
| Direct typed terminal input command capture | 🎯 7 | 🛡️ 7 | 🧠 8 | UI submit is trusted first, raw typed commands lower confidence until shell integration |
| Command dock history vs terminal transcript | 🎯 9 | 🛡️ 9 | 🧠 6 | `terminal_command_history_entries` derived from trusted command blocks |
| Historical replay side effects | 🎯 10 | 🛡️ 10 | 🧠 7 | Inert replay sandbox, never execute clipboard/window/shell side effects |
| Segment compression timing | 🎯 8 | 🛡️ 8 | 🧠 7 | Raw v1, optional zstd codec later for cold/large segments |
| Accidental cascade deletion of canonical history | 🎯 10 | 🛡️ 10 | 🧠 5 | Restrict commit/history FKs, deletes go through audited tombstones |
| Command history secret leakage | 🎯 8 | 🛡️ 9 | 🧠 8 | Redacted display text, sensitivity class, private-mode rules |
| Browser reconnect identity | 🎯 8 | 🛡️ 9 | 🧠 6 | `terminal_clients` table, not arbitrary client strings |
| Export/delete auditability | 🎯 9 | 🛡️ 9 | 🧠 7 | Explicit export/delete request rows and approval/status tracking |
| Multiple app processes writing same DB | 🎯 8 | 🛡️ 9 | 🧠 8 | Writer generation lease + stale writer recovery |
| Outbox worker crash while claimed | 🎯 9 | 🛡️ 9 | 🧠 7 | Claim lease with expiry, token and retry policy |
| System clock jumps | 🎯 8 | 🛡️ 8 | 🧠 6 | Clock anchors plus monotonic deltas in journal |
| JSON payload schema drift | 🎯 9 | 🛡️ 9 | 🧠 7 | Typed payload structs, schema registry, semver and fixture validation |
| Alternate screen/TUI restore fidelity | 🎯 7 | 🛡️ 8 | 🧠 9 | Explicit buffer mode events, coalesced frames and restore guarantee labels |
| Nullable outbox dedupe semantics | 🎯 10 | 🛡️ 9 | 🧠 4 | Partial unique index for non-null dedupe keys |
| DB/projection integrity drift | 🎯 9 | 🛡️ 10 | 🧠 7 | Durable integrity checks plus invariant SQL |
| Live SQLite backup under WAL | 🎯 10 | 🛡️ 10 | 🧠 7 | Use Online Backup API / `VACUUM INTO`, never plain hot `.db` copy |
| Online Backup API with Diesel connection | 🎯 8 | 🛡️ 9 | 🧠 7 | V1 backup via `VACUUM INTO`; add tiny `libsqlite3-sys` adapter only if incremental API is required |
| Disk full / temp full / quota pressure | 🎯 9 | 🛡️ 9 | 🧠 8 | Explicit storage-pressure state, gap/degraded markers, no silent pruning |
| Free-form status strings drift | 🎯 9 | 🛡️ 9 | 🧠 6 | Rust enums + SQL `CHECK` for bounded domains; validated extension domains |
| Support bundles leaking secrets | 🎯 8 | 🛡️ 9 | 🧠 7 | Redacted support bundle requests with explicit scopes |
| Parser upgrade invalidates snapshots | 🎯 8 | 🛡️ 9 | 🧠 8 | Projection versions and rebuild jobs |
| Command hash leaks sensitive commands | 🎯 8 | 🛡️ 9 | 🧠 7 | Local keyed hashes and no raw hash export by default |
| Encryption rollout timing | 🎯 9 | 🛡️ 8 | 🧠 9 | Schema-ready now, SQLCipher/envelope implementation after journal restore is stable |
| zellij/tmux fidelity | 🎯 7 | 🛡️ 8 | 🧠 9 | Backend-specific capture semantics, capability detection, no outer mux PTY as single shell truth |
| Forward migrations and legacy DBs | 🎯 8 | 🛡️ 9 | 🧠 8 | Guarded migrations, audit rows, legacy sessions marked degraded |
| Rendered mux surface mistaken for raw stream | 🎯 9 | 🛡️ 9 | 🧠 7 | `capture_semantics` field, lower-fidelity restore label, no raw replay claim |
| Backend capability drift after upgrade | 🎯 8 | 🛡️ 9 | 🧠 7 | Durable capability reports with expiry/reprobe and evidence JSON |
| Corrupt segment/snapshot during restore | 🎯 8 | 🛡️ 9 | 🧠 8 | Data health records, quarantine status, fallback from snapshot to raw replay where possible |
| Destructive migration too early | 🎯 9 | 🛡️ 10 | 🧠 8 | Expand/contract migrations, pre-migration backup, dual-read verification |
| Restore badge overpromises capability | 🎯 9 | 🛡️ 10 | 🧠 5 | Strict restore guarantee taxonomy backed by drills/capability reports |
| Protocol booleans cannot express v2 guarantee | 🎯 9 | 🛡️ 9 | 🧠 7 | Additive protocol fields: guarantee level, evidence refs, gap state and history replay state |
| Reliable profile hurts responsiveness | 🎯 8 | 🛡️ 9 | 🧠 8 | Budgets, barriers, bounded queue and explicit degradation |
| Secret data class drift | 🎯 8 | 🛡️ 9 | 🧠 7 | Privacy classification matrix and default redacted derived views |
| Windows key storage ambiguity | 🎯 8 | 🛡️ 9 | 🧠 8 | OS key-store capability probe, DPAPI/keyring-backed DB key, no plaintext fallback |
| Rollout cannot be disabled safely | 🎯 9 | 🛡️ 9 | 🧠 6 | Feature gates with kill switches and downgrade semantics |
| Long restore/search read blocks WAL checkpoint | 🎯 9 | 🛡️ 9 | 🧠 7 | Paged read APIs, short transactions, no async await with open SQLite cursor |
| Scope creep delays durable history MVP | 🎯 10 | 🛡️ 9 | 🧠 5 | Explicit MVP cutline and later gated tracks |
| Windows mux support overpromised | 🎯 9 | 🛡️ 9 | 🧠 6 | Zellij Windows probe required; tmux Windows treated as WSL/MSYS2-specific |
| Zellij rendered stream mistaken for raw replay | 🎯 9 | 🛡️ 9 | 🧠 7 | `subscribe`/`dump-screen` output is rendered evidence unless a raw capture source is proven |
| Capability report stale after mux upgrade | 🎯 8 | 🛡️ 9 | 🧠 6 | Expire reports on binary/version/config/probe changes and reprobe before guarantee upgrade |
| Restore loads huge history into memory | 🎯 9 | 🛡️ 9 | 🧠 7 | Two-phase restore: snapshot first, paged history reads, browser streaming after DB cursor close |
| Restored history visually merges with new process | 🎯 9 | 🛡️ 10 | 🧠 5 | Mandatory live boundary marker and separate historical region semantics |
| Command dock stores unsafe low-trust commands | 🎯 8 | 🛡️ 9 | 🧠 7 | Confidence gates, rerun policy, redacted/raw fields and explicit user confirmation |
| Backup implementation copies only `.db` under WAL | 🎯 10 | 🛡️ 10 | 🧠 5 | Production API exposes `VACUUM INTO`/Online Backup only, with manifest and restore check |
| Table class delete rules drift over time | 🎯 9 | 🛡️ 10 | 🧠 6 | Table-class matrix plus migration tests for canonical/derived/audit FK behavior |
| v2 DB exists but protocol/UI still shows legacy semantics | 🎯 9 | 🛡️ 9 | 🧠 7 | Add `restore_semantics_v2`/evidence refs beside legacy bools and test mapping |
| Legacy prune pattern leaks into v2 canonical history | 🎯 9 | 🛡️ 10 | 🧠 6 | v1 snapshot pruning stays legacy-only; v2 retention uses request/tombstone workflow |
| `raw_output_stream` capability claimed before durable capture exists | 🎯 10 | 🛡️ 10 | 🧠 5 | Capability is false/degraded until writer capture + restore drill prove it |
| Browser command history becomes source of truth | 🎯 9 | 🛡️ 10 | 🧠 5 | DB-backed command history is authoritative; browser cache is optional projection |
| Feature gate exposes partial v2 after failed migration/drill | 🎯 9 | 🛡️ 9 | 🧠 6 | Fail-closed rollout gates with legacy visual fallback |
| Diesel table macro limit exceeded by future schema change | 🎯 9 | 🛡️ 9 | 🧠 4 | Keep `32-column-tables` while max <= 32; CI column counter forces `64-column-tables` decision before merge |
| Diesel schema.rs drifts from migrations | 🎯 9 | 🛡️ 10 | 🧠 5 | `diesel print-schema`/checked schema diff in CI, no hand-edited schema drift |
| Derived search/AI tables accidentally block MVP | 🎯 9 | 🛡️ 9 | 🧠 5 | Core/derived rollout phases and feature gates keep MVP focused on durable history |

Most important mitigation:

```text
Ship smaller guarantees first, but every guarantee must be represented in schema, diagnostics and tests.
```

## Последний аудит low-confidence зон

Эти решения были самыми рискованными после повторного чтения плана и внешних docs. Ниже финальная позиция, которую надо считать архитектурной нормой.

1. **Command capture без shell integration** - выбранный путь: source-confidence model, not text parsing. 🎯 8   🛡️ 9   🧠 8. Примерно `900-1800` строк.
   Почему так: UI submit надежен, verified shell markers надежны, но raw terminal text and rendered mux output cannot prove where a command starts/ends. Поэтому low-trust commands can be displayed/searchable, but not auto-rerunnable.

2. **Zellij on Windows** - выбранный путь: advanced adapter after native MVP, capability-probed per install/session. 🎯 8   🛡️ 9   🧠 9. Примерно `1400-3200` строк.
   Почему так: Zellij can provide rendered pane streams/scrollback and control actions, but those are not automatically raw VT evidence. It is valuable, but it must not delay native ConPTY durable-history MVP.

3. **Alternate screen/TUI restore** - выбранный путь: raw stream + derived frame/snapshot model with explicit fidelity label. 🎯 7   🛡️ 9   🧠 9. Примерно `1800-4200` строк.
   Почему так: Vim/top/zellij-like apps are screen-state workloads, not line-log workloads. Exact replay is possible only when raw stream and parser version are trustworthy; rendered snapshots get `VisualRestoreOnly` or `BasicHistory`.

4. **Large restore/search path** - выбранный путь: snapshot-first, paged history reads, no long read transaction. 🎯 9   🛡️ 9   🧠 7. Примерно `800-1800` строк.
   Почему так: SQLite WAL works well with many readers, but long readers can interfere with checkpointing and memory. The UI should hydrate quickly and progressively load the rest.

5. **Backup under WAL** - выбранный путь: no raw file copy API; `VACUUM INTO` MVP, Online Backup API later if required. 🎯 10   🛡️ 10   🧠 6. Примерно `500-1200` строк.
   Почему так: history durability is meaningless if backup/restore loses WAL-contained transactions. Every backup must have a manifest and post-backup verification.

6. **Delete/retention semantics** - выбранный путь: audited request/tombstone service, not parent cascade. 🎯 10   🛡️ 10   🧠 7. Примерно `900-2200` строк.
   Почему так: "history never silently disappears" conflicts with convenient cascades. Canonical rows must fail closed; cleanup is a service workflow with evidence.

## Requirement traceability matrix

This is the checklist that keeps the plan tied to the product requirements already discussed.

| Requirement | Canonical implementation | API/UI behavior | Verification |
| --- | --- | --- | --- |
| Session history survives app/browser restart | `terminal_stream_segments`, `terminal_journal_events`, `terminal_screen_snapshots`, `terminal_topology_snapshots` | saved session restore shows prior visible output and live boundary | Playwright restart smoke, restore drill, checksum checks |
| User sees what they typed and what command output returned | `terminal_command_blocks` + event/byte ranges + `terminal_command_history_entries` | command list/dock is loaded from DB and links to output range | command source tests, output range invariant SQL, browser command dock restart test |
| Command history is per session/pane and not only browser cache | `terminal_command_history_entries.scope_kind`, `session_id`, `pane_id`, `trust_level` | UI cache is optional; DB is authoritative after restart | localStorage-disabled browser test, DB read API test |
| Native Windows path is stable first | native ConPTY capture emits typed events into writer | native sessions get best guarantee only after writer + drill pass | PowerShell/cmd/ConPTY burst/resize/restart tests |
| Zellij/tmux are supported honestly | `terminal_backend_capability_reports`, `capture_semantics`, route metadata | mux attach/process preservation is separate from persisted history guarantee | zellij probe tests, rendered-vs-raw downgrade tests |
| No false promise of process restore | restore semantics evidence contract | native restart keeps `preserves_process_state = false`; mux live attach can be separate | protocol mapping tests and browser badge tests |
| ORM is used for new persistence feature | Diesel migrations/models/repositories own v2 writes | legacy rusqlite remains isolated until refactor | cargo tests, dependency tree check, no new v2 raw SQL outside migrations/invariant tests |
| Full history does not disappear silently | canonical FK `RESTRICT`, retention request/tombstone flow | storage pressure warns/degrades instead of silently pruning | FK delete tests, retention tests, storage pressure tests |
| Backup is reliable under WAL | `terminal_backup_records`, `VACUUM INTO` MVP, manifest/checksum | backup status is visible and auditable | backup quick_check roundtrip test |
| Scaling does not freeze terminal | bounded writer queue, stream cursors, paged reads | visible degraded/gap state under pressure | writer lag tests, large restore paging tests, gap tests |

Rule:

- a requirement is not considered implemented until all three columns have code and tests.
- if any verification fails, the feature gate must disable authoritative v2 reads or downgrade the guarantee.

## Развилки, которые закрыты решением

### Storage для raw output

1. **SQLite BLOB stream segments first** - выбранный вариант. 🎯 9   🛡️ 8   🧠 6. Примерно `1200-2600` строк.
   Лучший fit для Windows-first v1: меньше path hazards, проще transaction atomicity, проще restore drill.

2. **External artifact store immediately**. 🎯 6   🛡️ 7   🧠 9. Примерно `3500-7000` строк.
   Нужен позже для больших artifacts, но слишком рано тянет path safety, orphan cleanup, fsync and crash recovery.

3. **Only screen snapshots, no raw journal**. 🎯 3   🛡️ 4   🧠 4. Примерно `800-1500` строк.
   Дешевле, но не дает полной истории, command ranges, replay, search and trustworthy restore.

### Diesel execution model

1. **Single named persistence worker + sync Diesel** - выбранный вариант. 🎯 10   🛡️ 10   🧠 7. Примерно `900-2200` строк.
   Самый предсказуемый вариант для SQLite writes and Windows app runtime.

2. **Connection pool for reads/writes сразу**. 🎯 7   🛡️ 7   🧠 8. Примерно `1600-3200` строк.
   Можно добавить позже для read scaling, но early write concurrency усложняет locks and correctness.

3. **Async ORM layer around SQLite**. 🎯 5   🛡️ 6   🧠 8. Примерно `1800-3600` строк.
   Не дает реального выигрыша для local SQLite write path и повышает риск cancellation/transaction bugs.

### Durability profile

1. **`reliable_history` default for v2 writer** - выбранный вариант. 🎯 8   🛡️ 10   🧠 8. Примерно `500-1200` строк.
   Writer connection uses WAL + `synchronous = FULL`, command-boundary barriers, restore drills and visible lag metrics. Это лучше соответствует требованию, что история максимально не должна теряться.

2. **Balanced default + strict opt-in**. 🎯 9   🛡️ 8   🧠 6. Примерно `300-800` строк.
   Лучше по latency, но хуже по продуктовой гарантии. Можно оставить как explicit performance profile later.

3. **Always `synchronous = NORMAL`**. 🎯 6   🛡️ 6   🧠 4. Примерно `100-300` строк.
   Проще, но не соответствует цели надежного terminal history.

### Search index

1. **Relational search v1, FTS5 later over redacted documents** - выбранный вариант. 🎯 9   🛡️ 9   🧠 7. Примерно `900-2200` строк.
   Не индексируем raw stream, избегаем FTS consistency/security traps, оставляем rebuildable derived index.

2. **FTS5 immediately over raw transcript**. 🎯 4   🛡️ 5   🧠 7. Примерно `1200-2600` строк.
   Быстро даст поиск, но высок риск утечки secret output and stale index issues.

3. **No persisted search model**. 🎯 5   🛡️ 6   🧠 2. Примерно `100-300` строк.
   Слишком ограниченно для command dock, AI context and support diagnostics.

### Session consistency model

1. **Per-pane `event_seq` + session commit log** - выбранный вариант. 🎯 9   🛡️ 10   🧠 8. Примерно `900-2200` строк.
   Дает атомарную точку восстановления всей сессии: topology, panes, segments, gaps and snapshots are tied to one commit order.

2. **Only per-pane sequence**. 🎯 6   🛡️ 6   🧠 5. Примерно `300-900` строк.
   Проще, но multi-pane snapshot can mix states from different moments.

3. **Only global sequence**. 🎯 7   🛡️ 7   🧠 7. Примерно `700-1600` строк.
   Удобно для total ordering, но хуже для per-pane replay, range queries and reconnect.

### Segment compression

1. **Raw hot segments, optional zstd cold/large segments later** - выбранный вариант. 🎯 8   🛡️ 8   🧠 7. Примерно `600-1600` строк.
   Сначала простая надежность и restore drills, потом compression without changing canonical model.

2. **Zstd immediately for every segment**. 🎯 6   🛡️ 7   🧠 7. Примерно `900-2200` строк.
   Уменьшит DB size, но усложнит writer hot path, drills and corruption diagnostics.

3. **Never compress**. 🎯 7   🛡️ 8   🧠 2. Примерно `0-200` строк.
   Надежно и просто, но плохо для долгих terminal histories and storage pressure.

### Delete semantics

1. **Audited tombstone/delete request flow** - выбранный вариант. 🎯 9   🛡️ 10   🧠 8. Примерно `900-2200` строк.
   User delete, retention pruning and crypto erase become explicit operations with status, scope and visible consequences.

2. **Direct SQL cascade delete from session**. 🎯 4   🛡️ 4   🧠 3. Примерно `200-600` строк.
   Быстро, но слишком легко потерять canonical history and diagnostics without explanation.

3. **Never delete, only hide**. 🎯 6   🛡️ 7   🧠 4. Примерно `300-900` строк.
   Хорошо для forensic history, но плохо для privacy, private mode and user expectations.

### Writer ownership

1. **DB-backed writer lease + process-local worker** - выбранный вариант. 🎯 8   🛡️ 9   🧠 8. Примерно `900-1800` строк.
   Защищает от двух процессов приложения, stale worker recovery and Windows restart edge cases.

2. **Only process-local single worker**. 🎯 6   🛡️ 6   🧠 5. Примерно `300-800` строк.
   Работает в happy path, но не защищает общий DB file от второго runtime.

3. **OS file lock only**. 🎯 7   🛡️ 7   🧠 6. Примерно `500-1200` строк.
   Нужен как дополнительный guard later, но DB lease лучше диагностируется and tests easier.

### Terminal screen model

1. **Persist raw stream + derived normal/alternate screen models** - выбранный вариант. 🎯 8   🛡️ 9   🧠 9. Примерно `1800-4200` строк.
   Raw stream remains canonical, but restore/UI can distinguish scrollback, alternate screen and coalesced TUI frames.

2. **Persist only raw bytes and let UI infer everything**. 🎯 6   🛡️ 7   🧠 5. Примерно `700-1600` строк.
   Simpler write path, but restore fidelity and diagnostics for TUI apps become fragile.

3. **Persist only screen snapshots for TUI apps**. 🎯 5   🛡️ 6   🧠 5. Примерно `900-1800` строк.
   Good for final visual state, weak for history/search/drill and command output ranges.

### JSON schema evolution

1. **Typed Rust payloads + schema registry + fixture validation** - выбранный вариант. 🎯 9   🛡️ 9   🧠 7. Примерно `700-1600` строк.
   Keeps JSON flexible but prevents unversioned payload drift.

2. **Ad hoc `serde_json::Value` everywhere**. 🎯 4   🛡️ 4   🧠 3. Примерно `100-300` строк.
   Fast initially, but migrations and old DB compatibility degrade quickly.

3. **Fully normalized tables for every event payload**. 🎯 6   🛡️ 8   🧠 10. Примерно `3000-7000` строк.
   Strong SQL shape, but too heavy for v1 event diversity.

### Integrity checks

1. **Scheduled durable integrity checks** - выбранный вариант. 🎯 9   🛡️ 10   🧠 7. Примерно `700-1800` строк.
   Combines SQLite checks, invariant SQL, projection drift checks and restore drill sampling.

2. **Only run tests in CI**. 🎯 5   🛡️ 5   🧠 3. Примерно `200-600` строк.
   CI catches new bugs, but not user DB corruption, interrupted migrations or disk issues.

3. **Only rely on SQLite constraints**. 🎯 6   🛡️ 6   🧠 3. Примерно `100-300` строк.
   Constraints help, but cannot prove projections, restore fidelity or redaction state.

### Support diagnostics

1. **Redacted support bundle request flow** - выбранный вариант. 🎯 8   🛡️ 9   🧠 7. Примерно `700-1600` строк.
   Makes diagnostics useful while keeping raw transcript and secrets out by default.

2. **Manual log file collection**. 🎯 4   🛡️ 4   🧠 2. Примерно `100-300` строк.
   Easy, but high privacy risk and inconsistent evidence.

3. **Always include full DB copy**. 🎯 2   🛡️ 3   🧠 2. Примерно `100-300` строк.
   Useful for debugging but unacceptable as default because it can contain secrets and raw output.

## Зафиксированные решения

### 1. ORM

Выбор: **Diesel ORM, sync API, SQLite backend**.

Оценка: 🎯 10   🛡️ 9   🧠 7
Объем инфраструктуры: примерно `700-1600` строк.

Почему:

- Diesel дает compile-time checked schema and query model.
- SQLite уже подходит текущей local-first архитектуре.
- Diesel sync API надежнее и зрелее для SQLite, чем пытаться делать async ORM вокруг SQLite.
- Async runtime не должен напрямую блокироваться на DB, поэтому нужен отдельный persistence executor.

Не делаем:

- не продолжаем расширять `rusqlite` для новой фичи;
- не пишем новую фичу на raw SQL API;
- не тащим enterprise DB или distributed store в первый этап.

ORM boundary:

```text
Diesel is required for:
  v2 repository writes
  v2 repository reads
  typed query models
  schema-backed insert/select/update/delete operations

Raw SQL is allowed only for:
  Diesel migrations
  SQLite PRAGMA initialization/diagnostics
  invariant SQL tests
  backup/maintenance commands such as VACUUM INTO
  rare SQLite feature probes that Diesel does not model

Raw SQL is not allowed for:
  normal v2 session/pane/journal/command persistence APIs
  command history repository writes
  restore plan repository reads
  delete/retention service mutations
```

Rule:

- if a new v2 repository method needs raw SQL, it must document why Diesel cannot express it and add a test that validates the exact SQL shape.
- legacy `rusqlite` code remains only behind `legacy::rusqlite_v1_store` until a later refactor.

### 2. DB execution model

Выбор: **single writer + dedicated blocking persistence executor**.

Оценка: 🎯 10   🛡️ 10   🧠 7
Объем: примерно `900-2200` строк.

Правило:

- одна writer lane отвечает за write transactions;
- read operations могут сначала идти через тот же executor, позже можно добавить read pool;
- все операции возвращают typed results/errors;
- Diesel `SqliteConnection` живет внутри blocking worker, а не гуляет по async tasks.

### 3. Raw output storage в первой версии

Выбор: **SQLite BLOB stream segments first, schema with future artifact refs**.

Оценка: 🎯 9   🛡️ 8   🧠 6
Объем: примерно `1200-2600` строк.

Почему:

- меньше Windows path/fcntl/fsync complexity на первом этапе;
- проще сделать crash-safe transaction;
- проще restore drill;
- проще тестировать старые DB fixtures;
- external artifact store можно добавить позже без перелома schema.

Ограничение:

- сегменты должны быть bounded по размеру;
- большие/media artifacts позже уйдут в external artifact store.

### 4. Restore semantics

Выбор: **native restores history/layout, not live process**.

Оценка: 🎯 10   🛡️ 10   🧠 5

Для native backend:

```text
restores_topology = true
restores_command_blocks = true
restores_output_history = true
restores_screen_snapshot = true
replays_terminal_journal = true
preserves_process_state = false
```

Для zellij/tmux:

- отдельный backend adapter;
- если mux session live, можно attach and preserve process state;
- если mux resurrected, использовать mux semantics;
- raw outer mux PTY не считать single-pane transcript truth.

### 4.1 Restore guarantee taxonomy

Выбор: **badges derive from evidence, not hope**.

Оценка: 🎯 10   🛡️ 10   🧠 5
Объем: примерно `400-900` строк.

Guarantee levels:

```text
Rich history:
  raw_vt_stream or verified high-fidelity capture
  successful restore drill
  no open critical data health records
  command blocks available from trusted sources

Basic history:
  output and command blocks persisted
  restore drill passes with minor gaps or lower command confidence
  no process preservation claim

Visual restore only:
  rendered snapshot/surface available
  no raw replay guarantee
  command rerun disabled by default

History degraded:
  known gaps, corrupt/quarantined rows, failed drill, storage pressure, or unknown backend capability

Live process attach:
  mux session is alive and attach succeeds
  process preservation comes from mux, not native restore
```

Rules:

- UI copy must not say "restored" without specifying level;
- API response should expose machine-readable `restore_guarantee_level`;
- `Rich history` requires both persisted data and evidence that restore works;
- any open critical `terminal_data_health_records` downgrades guarantee;
- mux live attach can be shown next to history level, not as replacement for history evidence.

### 5. Encryption

Выбор: **архитектуру заложить сразу, реализацию после journal/restore/outbox**.

Оценка: 🎯 9   🛡️ 8   🧠 7

Почему:

- encryption не должна блокировать первый durable journal;
- schema должна сразу иметь `key_ref`, `encryption_state`, `redaction_profile_id` fields там, где later будет нужно;
- нельзя делать design, который потом невозможно encrypted-migrate.

Concrete encryption decision:

1. **SQLCipher-style DB encryption with OS-protected DB key** - target later. 🎯 8   🛡️ 9   🧠 9. Примерно `2500-6000` строк.
   Лучший fit для local-first SQLite: protects broad DB contents, simpler mental model for users, but requires careful migration and startup key handling.

2. **Application-level envelope encryption per payload**. 🎯 7   🛡️ 8   🧠 9. Примерно `3500-8000` строк.
   Полезно для future external artifacts/selective crypto, но усложняет query/search/projection rebuild.

3. **Only OS-protect a few secrets, DB stays plaintext**. 🎯 5   🛡️ 5   🧠 4. Примерно `600-1400` строк.
   Easier, but does not satisfy sensitive terminal history expectations.

Windows-first key stance:

```text
terminal DB key:
  generated locally
  stored through OS credential store / DPAPI-backed provider
  never written to logs/support bundles

payload/table metadata:
  encryption_state
  key_ref
  crypto erase records

memory handling:
  zeroize/secrecy for key material
  no Debug/Serialize for raw keys
```

Encryption rollout rule:

- plaintext v2 journal ships first;
- encryption support ships only after backup, restore drills, data health and migration rollback are reliable;
- no silent plaintext fallback if encrypted DB open fails;
- support bundles must show key capability state without exposing key material.

### 6. Reliability

Выбор: **restore drill + invariant checks + fault injection skeleton сразу**.

Оценка: 🎯 9   🛡️ 9   🧠 8
Объем: примерно `1800-4200` строк тестов/инфраструктуры.

Минимум первой версии:

- old DB migration fixtures;
- journal ordering invariants;
- idempotency invariants;
- restore drill from temp DB;
- duplicate input/reconnect tests;
- failpoint skeleton around writer transaction and segment flush.

## Текущее состояние в репозитории

Сейчас persistence расположен в `crates/terminal-persistence` и использует `rusqlite`.

Что есть:

- `native_saved_sessions` хранит topology and screen snapshots as JSON;
- `session_routes` хранит route registry;
- `SavedNativeSession` содержит `topology` and `screens`;
- `SqliteSessionStore` сейчас один файл `src/lib.rs`, запускает `rusqlite_migration` and `ensure_manifest_column` from `ensure_schema`;
- `open_connection()` вызывает `ensure_schema()` на каждый operation, что приемлемо для v1, но не для v2 hot writer path;
- `terminal-backend-native::TranscriptBuffer` keeps only `256 KiB` in memory and is not a durable source of truth;
- native backend capabilities currently expose rendered viewport/snapshot paths, but not durable `raw_output_stream` evidence;
- restore native сейчас пересоздает layout, но не replay-ит старые screen buffers.

Критичная текущая граница:

```rust
SavedSessionRestoreSemantics {
    restores_topology: true,
    restores_focus_state: true,
    restores_tab_titles: true,
    uses_saved_launch_spec: has_launch,
    replays_saved_screen_buffers: false,
    preserves_process_state: false,
}
```

Что надо изменить:

- не удалять старую таблицу сразу;
- добавить новую Diesel persistence v2 рядом;
- вынести v1 store в `legacy::rusqlite_v1_store` or adapter module without changing public behavior first;
- v2 migrations must run once during controlled startup/connection initialization, not inside every hot read/write operation;
- ad hoc `ensure_manifest_column` style migrations stay legacy-only; v2 schema changes go through embedded Diesel migrations and fixture tests;
- native engine must emit typed capture events before `raw_output_stream` or `Rich history` can be claimed;
- старый API `list_saved_sessions`, `saved_session`, `restore_saved_session` постепенно перевести на v2 read models;
- `replays_saved_screen_buffers` должен стать `true` только после реального replay/hydration path.

Protocol rollout rule:

- old boolean fields remain for compatibility;
- v2 adds additive fields such as `restore_guarantee_level`, `history_replay_state`, `latest_restore_drill_status`, `has_known_gaps`, `source_session_id` and `restored_session_id`;
- `terminal-daemon::adapters::protocol_mapping` must keep `replays_saved_screen_buffers = false` for legacy snapshot-only rows;
- `replays_saved_screen_buffers = true` only when persisted v2 journal/snapshot data was actually hydrated or replayed in the response path;
- compatibility tests must cover old clients that only know the boolean fields and new clients that read guarantee taxonomy.

## Protocol compatibility and restore semantics v2

Current protocol has useful booleans, but v2 needs evidence-based semantics rather than one optimistic flag.

Chosen policy: 🎯 9   🛡️ 9   🧠 7

Additive response shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSessionRestoreSemanticsV2 {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,

    pub restore_guarantee_level: RestoreGuaranteeLevel,
    pub history_replay_state: HistoryReplayState,
    pub source_session_id: SessionId,
    pub restored_session_id: Option<SessionId>,
    pub latest_restore_drill_status: Option<RestoreDrillStatus>,
    pub has_known_gaps: bool,
    pub evidence_refs: Vec<String>,
}
```

Preferred wire compatibility shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSessionRecord {
    // existing fields stay unchanged
    pub restore_semantics: SavedSessionRestoreSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreSavedSessionResponse {
    // existing fields stay unchanged
    pub restore_semantics: SavedSessionRestoreSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}
```

Fallback wire shape if adding fields to existing structs is blocked by release cadence:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSessionRecordEnvelope {
    #[serde(flatten)]
    pub legacy: SavedSessionRecord,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}
```

Why not replace the existing bool struct immediately:

- current `terminal-protocol::SavedSessionRestoreSemantics` is already consumed by clients;
- old clients must continue to see the conservative booleans;
- new clients need evidence/gap/drill fields without forcing a breaking protocol-major change;
- `protocol_mapping.rs` must keep legacy snapshot-only rows conservative until v2 read path hydrates real history.
- preferred implementation is an optional additive field in existing response DTOs; wrapper/envelope types are only a fallback when the current protocol release process requires them.

Enums:

```rust
pub enum RestoreGuaranteeLevel {
    RichHistory,
    BasicHistory,
    VisualRestoreOnly,
    HistoryDegraded,
}

pub enum HistoryReplayState {
    NotAvailable,
    SnapshotOnly,
    HydratedFromSnapshot,
    ReplayedFromJournal,
    PartiallyReplayedWithGaps,
}
```

Rules:

- add fields as protocol-minor compatible extension with serde defaults where possible;
- old clients keep using the existing booleans;
- new clients show guarantee taxonomy and evidence refs;
- `replays_saved_screen_buffers` maps to true only for `HydratedFromSnapshot`, `ReplayedFromJournal` or `PartiallyReplayedWithGaps` when user-visible history was actually restored;
- `VisualRestoreOnly` can still show old snapshot text, but cannot claim raw journal replay;
- `preserves_process_state` stays false for native host restart restore;
- mux live attach status is separate from persisted history guarantee.

## Evidence-backed restore guarantee contract

Restore guarantees are product promises and must be backed by durable evidence, not backend names.

Guarantee inputs:

```text
topology evidence:
  terminal_topology_snapshots row
  checksum verified
  high_water_commit_seq present

screen evidence:
  terminal_screen_snapshots rows for panes
  parser/projection version known
  checksum verified

journal evidence:
  terminal_stream_segments rows
  terminal_journal_events rows
  stream cursor invariants pass
  no unclosed terminal_history_gaps in requested range

backend evidence:
  terminal_backend_capability_reports row
  probe status fresh
  capture_semantics known
  backend version/path/config has not drifted since probe

drill evidence:
  latest terminal_restore_drills row is passed or accepted degraded
```

Guarantee derivation:

```text
RichHistory:
  raw_vt_stream journal available
  verified snapshots/topology
  no known gaps in restored range
  restore drill passed after latest relevant parser/schema/backend version

BasicHistory:
  user-visible history is hydrated/replayed
  command blocks exist where trusted
  minor gaps or rendered evidence are clearly labeled

VisualRestoreOnly:
  only screen snapshots/rendered scrollback/imported text are available
  no raw replay claim
  no process preservation claim

HistoryDegraded:
  gaps, corrupt/quarantined rows, stale capability report, failed drill or missing evidence
```

Evidence refs should be machine-readable strings such as:

```text
topology_snapshot:<id>
screen_snapshot:<id>
stream_segment:<id>
journal_event_range:<pane_id>:<event_seq_low>:<event_seq_high>
history_gap:<id>
backend_capability_report:<id>
restore_drill:<id>
data_health_record:<id>
```

Rules:

- the UI badge must be computed from this contract, not hardcoded per backend;
- any stale/missing evidence downgrades the guarantee, it does not silently pass;
- `replays_saved_screen_buffers = true` is a compatibility projection from evidence, not a source field;
- legacy v1 snapshot-only rows map to `VisualRestoreOnly` unless v2 journal exists;
- mux live attach can add `preserves_process_state = true` only for that live mux attach response, not for persisted native restore.

## Целевая crate/module структура

Оставляем crate:

```text
crates/terminal-persistence/
```

Внутри постепенно приводим к структуре:

```text
src/
  lib.rs
  config.rs
  error.rs
  clock.rs
  ids.rs

  db/
    mod.rs
    connection.rs
    executor.rs
    migrations.rs
    schema.rs              # Diesel schema, generated/checked
    models.rs
    types.rs

  journal/
    mod.rs
    capture_event.rs
    writer.rs
    segmenter.rs
    sequence.rs
    replay.rs

  command_blocks/
    mod.rs
    state_machine.rs
    shell_markers.rs
    trust.rs

  snapshots/
    mod.rs
    snapshot_writer.rs
    snapshot_reader.rs
    restore_manifest.rs

  outbox/
    mod.rs
    worker.rs
    claim.rs
    jobs.rs

  idempotency/
    mod.rs
    keys.rs

  redaction/
    mod.rs
    profiles.rs
    scanner.rs

  restore/
    mod.rs
    service.rs
    drill.rs

  legacy/
    mod.rs
    rusqlite_v1_store.rs
    migrate_v1.rs

  testing/
    mod.rs
    fixtures.rs
    invariants.rs
    failpoints.rs
```

Dependency direction:

```text
terminal-runtime
  -> terminal-persistence public services
terminal-persistence
  -> terminal-domain
  -> terminal-projection
  -> terminal-backend-api DTOs only where needed
terminal-backend-*
  -> terminal-backend-api events/ports
```

Persistence must not depend on `terminal-runtime`.

## Dependencies

Перед добавлением зависимостей версии надо еще раз проверить через `cargo search`, `cargo info` and docs.rs. На 2026-04-29 baseline повторно проверен через `cargo search`:

```toml
[workspace.dependencies]
diesel = { version = "2.3.8", default-features = false, features = ["sqlite", "returning_clauses_for_sqlite_3_35", "32-column-tables"] }
diesel_migrations = { version = "2.3.2", default-features = false, features = ["sqlite"] }
blake3 = "1.8.5"
uuid = { version = "1.23.1", features = ["serde", "v4", "v5", "v7"] }
schemars = { version = "1.2.1", features = ["derive"] }
semver = { version = "1.0.28", features = ["serde"] }
# Later, when compression iteration starts:
zstd = { version = "0.13.3", default-features = false }
# Later, when encryption/key handling iteration starts:
zeroize = "1.8.2"
secrecy = "0.10.3"
# Check exact platform-store needs before adding:
keyring = "4.0.0"
```

Для bundled SQLite нужно отдельное закрепление через `libsqlite3-sys`, потому у Diesel нет публичной `sqlite-bundled` feature:

```toml
libsqlite3-sys = { version = "0.37.0", features = ["bundled"] }
```

Важно:

- текущий `Cargo.lock` уже содержит `libsqlite3-sys 0.37.0`, но после добавления Diesel надо проверить единое дерево через `cargo tree -i libsqlite3-sys`;
- текущий `rusqlite = { version = "0.39.0", features = ["bundled"] }` уже включает `libsqlite3-sys/bundled`; прямое workspace-закрепление нужно, чтобы bundled SQLite не исчезла после удаления `rusqlite`;
- текущий workspace уже имеет `uuid` with `serde/v4/v5`; v2 must add `v7` without dropping existing features;
- не смешивать одновременно `rusqlite` and Diesel SQLite native library без проверки linking/features;
- на переходный период `rusqlite` может остаться только в `legacy` модуле;
- в финале v2 new writes должны идти только через Diesel.
- migrations должны быть embedded через `diesel_migrations::embed_migrations!`, чтобы daemon/node package не зависел от внешней папки migrations at runtime.
- `cargo info diesel` confirms `32-column-tables`, `64-column-tables` and `128-column-tables`; current planned maximum table width is `27`, so `32-column-tables` is enough for the documented schema.
- if any table grows past 32 columns, choose explicitly between schema split and `64-column-tables`; do not silently add `huge-tables`.
- `returning_clauses_for_sqlite_3_35` можно использовать только если runtime SQLite version >= 3.35; startup check должен логировать фактический `SELECT sqlite_version()`.
- packaged Windows build should use bundled SQLite from `libsqlite3-sys 0.37.0`; docs.rs currently states this bundles SQLite `3.51.3`.
- reliable WAL history profile should fail closed or downgrade visibly if runtime SQLite is older than a known patched line for the WAL-reset bug (`3.51.3`, or documented backports such as `3.50.7` / `3.44.6`).
- startup diagnostics should persist/log `sqlite_version()`, compile options relevant to FTS/JSON/backup, `journal_mode`, `synchronous`, `wal_autocheckpoint` and whether the binary is using bundled SQLite.
- `keyring 4.0.0` currently has MSRV `1.88.0`; the workspace is `rust-version = "1.90"`, so MSRV is acceptable, but platform behavior still needs a Windows Credential Manager/DPAPI probe before enabling encryption by default.
- `zeroize`/`secrecy` can be added only in the encryption iteration; do not pull them into the journal MVP unless key material is actually handled.

Dependency gate before merge:

```text
cargo search diesel --limit 5
cargo search diesel_migrations --limit 5
cargo search libsqlite3-sys --limit 5
cargo search uuid --limit 5
cargo search schemars --limit 5
cargo search semver --limit 5
cargo search zstd --limit 5
cargo tree -i diesel
cargo tree -i diesel_migrations
cargo tree -i libsqlite3-sys
cargo tree -e features -i libsqlite3-sys@0.37.0
cargo test -p terminal-persistence
```

Column-budget gate:

```text
max planned table columns today: 27
current Diesel feature: 32-column-tables
CI must fail if any Diesel-managed table has > 32 columns without an explicit decision:
  split table / move payload to JSON with schema / enable 64-column-tables
```

Reference docs used for this decision:

- [Diesel latest docs](https://docs.rs/diesel/latest/diesel/)
- [Diesel crate metadata](https://docs.rs/crate/diesel/latest)
- [Diesel SqliteConnection](https://docs.diesel.rs/2.3.x/diesel/sqlite/struct.SqliteConnection.html)
- [diesel_migrations latest source/docs](https://docs.rs/crate/diesel_migrations/latest)
- [libsqlite3-sys crate metadata](https://docs.rs/libsqlite3-sys/0.37.0/libsqlite3_sys/)
- [BLAKE3 crate docs](https://docs.rs/blake3/latest/blake3/)
- [uuid crate docs](https://docs.rs/uuid/latest/uuid/)
- [schemars crate docs](https://docs.rs/schemars/latest/schemars/)
- [semver crate docs](https://docs.rs/semver/latest/semver/)
- [keyring crate docs](https://docs.rs/keyring/4.0.0/keyring/)
- [zeroize crate docs](https://docs.rs/zeroize/1.8.2/zeroize/)
- [secrecy crate docs](https://docs.rs/secrecy/0.10.3/secrecy/)
- [alacritty_terminal crate docs](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/)
- [Diesel embedded migrations](https://docs.diesel.rs/2.1.x/diesel_migrations/macro.embed_migrations.html)
- [SQLite PRAGMA reference](https://www.sqlite.org/pragma.html)
- [SQLite WAL](https://www.sqlite.org/wal.html)
- [SQLite Backup API](https://www.sqlite.org/backup.html)
- [SQLite result and error codes](https://www.sqlite.org/rescode.html)
- [SQLite STRICT tables](https://www.sqlite.org/stricttables.html)
- [SQLite FTS5](https://www.sqlite.org/fts5.html)
- [SQLite PRAGMA foreign_keys](https://www.sqlite.org/foreignkeys.html)
- [tmux man page](https://man7.org/linux/man-pages/man1/tmux.1.html)
- [MSYS2 tmux package](https://packages.msys2.org/packages/tmux)
- [Zellij documentation](https://zellij.dev/documentation/)
- [Zellij installation](https://zellij.dev/documentation/installation.html)
- [Zellij programmatic control](https://zellij.dev/documentation/programmatic-control.html)
- [Zellij subscribe](https://zellij.dev/documentation/zellij-subscribe.html)
- [Zellij CLI actions](https://zellij.dev/documentation/cli-actions.html)
- [Zellij DumpScreen action](https://zellij.dev/documentation/keybindings-possible-actions.html)
- [Microsoft ConPTY pseudoconsole session](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
- [Microsoft DPAPI CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata)
- [Zstandard format specification](https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md)
- [zstd Rust crate docs](https://docs.rs/zstd/latest/zstd/)
- [XTerm control sequences](https://www.x.org/docs/xterm/ctlseqs.pdf)
- [WezTerm shell integration](https://wezterm.org/shell-integration.html)

## Diesel schema generation workflow

Diesel gives value only if migrations, generated schema and Rust models stay synchronized.

Required files:

```text
crates/terminal-persistence/diesel.toml
crates/terminal-persistence/migrations/
crates/terminal-persistence/src/db/schema.rs
crates/terminal-persistence/src/db/models.rs
```

Example `diesel.toml`:

```toml
[print_schema]
file = "src/db/schema.rs"
with_docs = true
custom_type_derives = ["diesel::query_builder::QueryId"]
```

Workflow:

```text
edit migration
run migration on temp DB
run diesel print-schema
format Rust
run cargo test -p terminal-persistence
run schema diff check in CI
```

Rules:

- `schema.rs` is generated/checked, not a place for ad hoc hand schema drift;
- manual edits are allowed only when Diesel cannot infer a SQLite detail, and must be documented in a nearby comment;
- every table in Diesel-managed repositories must appear in `schema.rs`;
- derived/non-MVP tables may live behind feature modules, but their migrations and schema fragments still need fixtures;
- CI must fail if migrations changed and `schema.rs` was not regenerated;
- CI must run a column-budget check before accepting new Diesel table definitions.

Schema drift checks:

```text
cargo test -p terminal-persistence migration_fixtures
diesel print-schema --database-url <temp-db>
compare generated schema.rs with committed schema.rs
run PRAGMA foreign_key_check
run invariant SQL smoke suite
```

## SQLite connection policy

Каждое Diesel connection открытие должно проходить через один initializer.

Пример:

```rust
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;

const TERMINAL_PERSISTENCE_APP_ID: i32 = 0x54505632; // "TPV2"

pub enum DurabilityProfile {
    ReliableHistory,
    PerformanceHistory,
    TestFast,
}

pub fn establish_connection(
    path: &std::path::Path,
    durability: DurabilityProfile,
) -> Result<SqliteConnection, PersistenceError> {
    let database_url = sqlite_database_url(path)?;
    let mut conn = SqliteConnection::establish(database_url)?;

    // Diesel docs recommend applying SQLite PRAGMAs as separate statements.
    conn.batch_execute("PRAGMA busy_timeout = 5000;")?;
    conn.batch_execute("PRAGMA journal_mode = WAL;")?;
    match durability {
        DurabilityProfile::ReliableHistory => {
            conn.batch_execute("PRAGMA synchronous = FULL;")?;
        }
        DurabilityProfile::PerformanceHistory | DurabilityProfile::TestFast => {
            conn.batch_execute("PRAGMA synchronous = NORMAL;")?;
        }
    }
    conn.batch_execute("PRAGMA wal_autocheckpoint = 1000;")?;
    conn.batch_execute("PRAGMA foreign_keys = ON;")?;
    conn.batch_execute("PRAGMA temp_store = MEMORY;")?;

    verify_application_id_or_empty(&mut conn)?;
    run_embedded_migrations(&mut conn)?;
    ensure_terminal_database_identity(&mut conn)?;

    Ok(conn)
}

fn sqlite_database_url(path: &std::path::Path) -> Result<&str, PersistenceError> {
    path.to_str().ok_or_else(|| {
        PersistenceError::InvalidPath(
            "Diesel SQLite connection path must be representable as UTF-8; app data paths are required to pass this probe"
                .to_string(),
        )
    })
}
```

Notes:

- `foreign_keys = ON` must be set per connection.
- `busy_timeout` must be explicit.
- WAL is required for writer/readers, but WAL size needs checkpoint policy.
- Production v2 writer default is `reliable_history`, which uses `synchronous = FULL`.
- `synchronous = NORMAL` is only a named performance/test profile, not the reliability default.
- Future encrypted profile will add SQLCipher PRAGMAs here.
- Startup order must be: connection PRAGMAs, `application_id` guard, embedded migrations, then `terminal_db_identity` row check.
- `application_id = 0` is allowed only for empty/new or explicitly recognized legacy DB; any other mismatching non-zero value fails closed.
- After migrations, `ensure_terminal_database_identity` must set `PRAGMA application_id = TERMINAL_PERSISTENCE_APP_ID` if it was `0`.
- On startup, set/check `PRAGMA application_id`, `PRAGMA user_version` and one-row `terminal_db_identity`. This prevents accidentally opening an unrelated SQLite file as terminal history.
- Run `PRAGMA wal_checkpoint(TRUNCATE)` only on controlled startup/shutdown or maintenance, not on hot path.
- Run `PRAGMA optimize` from maintenance, not for every write transaction.
- For new DBs, initial migration should set `auto_vacuum = INCREMENTAL` before creating large tables. For legacy DBs, enabling it later requires an explicit VACUUM migration and should not be hidden in normal startup.
- Non-UTF Windows paths should fail early in v1. If we later need non-UTF support, add a tested path adapter instead of ad hoc lossy conversion.

## Durability and maintenance policy

Terminal history has two competing requirements:

```text
do not lose history
do not freeze the live terminal
```

Chosen policy:

- production writer default: `reliable_history`;
- performance profile exists but must be explicitly selected;
- command boundary, snapshot/save and session close force writer barriers;
- long-running output is still batched, but batches are bounded by bytes and time;
- if persistence cannot keep up, UI must show degraded/gap state instead of pretending history is complete.

Writer durability behavior:

```text
OutputBytes:
  append to in-memory segment
  flush on size/time boundary

CommandSubmitted or Shell PreExec:
  flush current segment
  commit command event + barrier

CommandFinished:
  flush current segment
  commit command finish + snapshot/outbox request

SessionSave/Close:
  flush all panes
  write topology snapshot
  write screen snapshots
  run lightweight restore drill if budget allows
```

Maintenance behavior:

```text
startup:
  app id guard
  embedded migrations
  identity check
  bounded PASSIVE WAL checkpoint only after writer lease/reader budget check
  sample restore drill for recently active sessions

idle:
  PRAGMA optimize
  bounded retention scan
  outbox drain
  storage pressure probe
  WAL size check
  bounded PASSIVE checkpoint under budget
  TRUNCATE checkpoint only in controlled maintenance window

shutdown:
  best-effort flush all panes
  write session snapshots
  do not block forever
```

Never:

- never silently delete canonical history under the default policy;
- never run heavy maintenance in the PTY reader path;
- never hide WAL/DB growth from diagnostics;
- never claim data is complete if a gap/tombstone exists.
- never copy only the `.db` file while WAL mode can be active;
- never silently prune raw history as the first response to disk pressure.

## Performance and storage budgets

Budgets are product guarantees. If the implementation cannot meet them, it must degrade visibly instead of freezing the terminal.

Initial budgets:

```text
writer queue:
  capacity: 1024 capture events
  warning: > 50% for 5s
  degraded: full or writer lag > 2000ms

segment flush:
  max hot segment: 32-128 KiB
  max flush interval: 100ms for active output
  command boundary: barrier flush

restore:
  small session visible restore target: < 500ms
  medium session restore target: < 2000ms
  fallback: hydrate latest snapshot first, replay remaining range progressively

maintenance:
  no heavy checkpoint/vacuum on capture hot path
  maintenance chunk target: < 50ms DB lock windows where practical

storage:
  warn when DB + WAL exceeds policy threshold
  warn when free space falls below configured floor
  never auto-delete canonical history under default policy
```

Actions when budgets are missed:

- writer lag: show `Writer lagging`, reduce derived work, keep capture first;
- queue full: create history gap, keep live terminal responsive;
- restore slow: show snapshot quickly and replay progressively with boundary marker;
- WAL bloat: schedule checkpoint and record maintenance result;
- DB growth: show storage pressure/remediation, not silent pruning.

Implementation rule:

- every budget above needs a metric or diagnostic row;
- CI should include at least one long-output fixture and one restore-time smoke test;
- acceptance criteria should be tied to measured budgets, not "seems fast".

Backup behavior:

```text
manual/dev backup request:
  create terminal_backup_records row
  use SQLite Online Backup API or VACUUM INTO
  write manifest + checksum
  run quick integrity check against backup target when budget allows

support/export bundle:
  default to redacted logical export
  require explicit approval for raw DB backup
```

## Diesel executor

Diesel is sync. We isolate it.

Target public shape:

```rust
use diesel::sqlite::SqliteConnection;
use tokio::sync::{mpsc, oneshot};

type BoxedPersistenceJob =
    Box<dyn FnOnce(&mut SqliteConnection) + Send + 'static>;

#[derive(Clone)]
pub struct PersistenceExecutor {
    tx: mpsc::Sender<BoxedPersistenceJob>,
}

impl PersistenceExecutor {
    pub async fn execute<T>(
        &self,
        run: impl FnOnce(&mut SqliteConnection) -> Result<T, PersistenceError> + Send + 'static,
    ) -> Result<T, PersistenceError>
    where
        T: Send + 'static,
    {
        let (reply_tx, reply_rx) = oneshot::channel::<Result<T, PersistenceError>>();
        let job: BoxedPersistenceJob = Box::new(move |conn| {
            let result = run(conn);
            let _ = reply_tx.send(result);
        });

        self.tx
            .send(job)
            .await
            .map_err(|_| PersistenceError::ExecutorClosed)?;

        reply_rx.await.map_err(|_| PersistenceError::ExecutorClosed)?
    }
}

pub fn spawn_persistence_executor(
    database_path: std::path::PathBuf,
) -> PersistenceExecutor {
    let (tx, mut rx) = mpsc::channel::<BoxedPersistenceJob>(128);

    std::thread::Builder::new()
        .name("terminal-persistence-writer".to_string())
        .spawn(move || {
            let mut conn = establish_connection(&database_path)
                .expect("terminal persistence connection should open");
            while let Some(job) = rx.blocking_recv() {
                job(&mut conn);
            }
        })
        .expect("terminal persistence thread should spawn");

    PersistenceExecutor { tx }
}
```

Implementation rule:

- one writer worker initially;
- all writes through executor;
- no random `SqliteConnection::establish` in services;
- every command is small and typed;
- batch writer owns transaction boundaries for journal appends.
- do not use `tokio::spawn_blocking` per write in v1; it is easier to accidentally create parallel SQLite writers. A single named worker thread is more predictable.
- add read pool only after write path and invariants are stable.

Potential later improvement:

```text
single writer thread
+ small read pool with read-only connections
+ explicit snapshot/read consistency rules
```

## Read path and WAL checkpoint policy

SQLite WAL allows readers and one writer to coexist, but long-running readers can keep old WAL pages alive and delay checkpoint progress. Restore/search/support reads must therefore be designed as budgeted, paged operations.

Chosen policy: 🎯 9   🛡️ 9   🧠 7

Read classes:

```text
hot UI read:
  session list, badges, recent command blocks
  target: small indexed queries
  transaction: none or very short

restore read:
  topology + snapshots + segment ranges
  target: progressive hydration
  transaction: page-sized read windows

search/support read:
  derived redacted projections first
  raw canonical reads only after explicit approval/scope

maintenance read:
  integrity/checkpoint/prune planning
  must yield between chunks
```

Rules:

- do not hold a SQLite statement iterator across `.await`;
- do not keep a read transaction open while streaming a large restore to the browser;
- page segment reads by `pane_id`, `stream_id`, `event_seq_low/event_seq_high`, `byte_low/byte_high` and byte budget;
- restore should load the latest snapshot first, then replay in chunks;
- long support/export reads should use explicit request rows and progress state;
- read pool can be added later, but every connection must run the same PRAGMA initializer and be identified as read/write/maintenance;
- if WAL grows while reads are active, diagnostics should identify the active read class where possible.

Example paged restore shape:

```rust
pub struct RestorePageRequest {
    pub session_id: SessionId,
    pub pane_id: PaneId,
    pub stream_id: String,
    pub event_seq_after: i64,
    pub event_seq_until: i64,
    pub byte_budget: usize,
    pub max_segments: usize,
    pub max_bytes: usize,
}

pub struct RestorePage {
    pub segments: Vec<StreamSegment>,
    pub next_event_seq_after: Option<i64>,
    pub next_byte_after: Option<i64>,
    pub known_gaps: Vec<HistoryGap>,
}
```

Invariant:

```text
restore page read closes DB cursor before sending data to websocket/client
```

## Sequence domains and range semantics

This section is a hard contract for migrations and code review. The plan must not use a generic `seq` when the domain matters.

Chosen policy: 🎯 10   🛡️ 10   🧠 6

Domains:

```text
commit_seq:
  session-local transaction/order number from terminal_commit_log
  used for multi-pane atomic restore boundaries

event_seq:
  pane/session scoped journal event order
  allocated by terminal_stream_cursors.next_event_seq
  used by terminal_journal_events, command lifecycle and restore replay ranges

byte_seq:
  byte offset/order inside a canonical captured payload stream
  allocated by terminal_stream_cursors.next_byte_seq
  used by stream segments, exact raw output extraction and gap byte accounting

frame_seq:
  derived visual frame order for screen/TUI projection only
  never canonical for raw replay
```

Rules:

- `event_seq_low/event_seq_high` ranges are inclusive.
- `byte_low/byte_high` ranges are half-open: `[byte_low, byte_high)`.
- `terminal_journal_events.event_seq` is an event counter, not a byte offset.
- `terminal_stream_segments.event_seq_low/event_seq_high` points to journal events represented by the segment.
- `terminal_stream_segments.byte_low/byte_high` points to canonical stored payload bytes before text decoding.
- `terminal_command_blocks.output_event_seq_low/output_event_seq_high` is the canonical command output range.
- `terminal_command_blocks.output_byte_low/output_byte_high` is optional and exists only when the backend can map command output to exact raw bytes.
- `terminal_screen_snapshots.high_water_event_seq` is pane-local parser high-water, while topology restore consistency uses `high_water_commit_seq` plus `pane_high_water_json`.
- Rendered mux surfaces may have `event_seq`, but their `byte_seq` is bytes of rendered evidence, not raw terminal process output.
- Search/export/AI read models must keep both event range and byte range when available, so redacted text can be traced back without pretending it is canonical raw history.

Naming rule:

```text
Allowed ambiguous name:
  seq only inside local code variables where the type name already includes EventSeq/CommitSeq/ByteSeq

Forbidden in DB/migration/public DTO names:
  seq_low
  seq_high
  next_seq
  last_seq
  high_water_seq
```

If the implementation needs a new range, choose one explicitly:

```text
event_seq_low / event_seq_high
byte_low / byte_high
commit_seq_low / commit_seq_high
frame_seq_low / frame_seq_high
```

## Core schema v2

Use Diesel migrations under:

```text
crates/terminal-persistence/migrations/
```

Initial migration:

```text
00000000000001_terminal_persistence_v2/up.sql
00000000000001_terminal_persistence_v2/down.sql
```

Implementation phases:

```text
phase 1 - core durable history:
  identity
  feature gates
  retention policy seed
  sessions/panes
  writer generations
  commit log
  stream/session cursors
  stream segments
  journal events
  capture receipts
  command blocks
  command history entries
  topology/screen snapshots
  history gaps
  restore drills

phase 2 - operational workflows:
  outbox
  idempotency
  clients/delivery offsets
  backup records
  storage pressure events
  support bundles
  delete/export requests

phase 3 - derived indexes and advanced tracks:
  search documents
  AI context projections
  compression metadata/jobs
  encryption/key rotation flows
  mux structured adapters
```

Rule:

- Phase 1 is the minimum for "history survives restart".
- Phase 2 is required before production MVP is called reliable.
- Phase 3 must not block the native Windows durable-history MVP unless a specific acceptance criterion depends on it.

Migration ordering note:

- SQL table definitions below are grouped for readability, not necessarily exact migration order;
- create referenced parent tables before child tables when practical;
- seed `terminal_retention_policies` before inserting sessions that reference `default_full_history`;
- if a forward reference is kept, add an explicit migration test with `PRAGMA foreign_key_check`;
- first migration should end by running invariant smoke checks on an empty DB.

Recommended first migration batches:

```text
batch 1 - identity and schema contracts:
  terminal_db_identity
  terminal_payload_schemas
  terminal_projection_versions
  terminal_feature_gates
  terminal_retention_policies

batch 2 - audit/ops parent tables:
  terminal_maintenance_runs
  terminal_integrity_checks
  terminal_data_health_records
  terminal_backup_records
  terminal_storage_pressure_events
  terminal_crypto_keys
  terminal_crypto_key_events

batch 3 - session topology parents:
  terminal_sessions
  terminal_panes
  terminal_backend_capability_reports
  terminal_writer_generations
  terminal_clock_anchors
  terminal_session_cursors
  terminal_commit_log
  terminal_stream_cursors

batch 4 - canonical history:
  terminal_topology_snapshots
  terminal_stream_segments
  terminal_journal_events
  terminal_capture_receipts
  terminal_command_blocks
  terminal_command_history_entries
  terminal_screen_snapshots

batch 5 - workflows and clients:
  terminal_outbox_messages + partial unique index
  terminal_idempotency_keys
  terminal_clients
  terminal_delivery_offsets
  terminal_history_gaps
  terminal_restore_drills
  terminal_delete_requests
  terminal_deletion_tombstones
  terminal_export_requests
  terminal_support_bundles
```

Rules:

- if FK ordering becomes cyclic, prefer nullable FK plus explicit invariant check over disabling FK checks broadly;
- each batch should have at least one empty-DB migration test and one invalid-row constraint test;
- after the full first migration, run `PRAGMA foreign_key_check` and a small invariant suite.

Migration rollout policy:

```text
expand:
  add new tables/nullable columns/indexes
  keep old read path alive

dual-write / shadow-read:
  write v1 and v2 where needed
  compare restore plans and command history projections
  record migration audit rows

switch:
  make v2 read path authoritative only after restore drills pass
  mark legacy sessions with explicit degraded semantics

contract:
  remove old columns/tables only in a later release
  require backup, restore drill and foreign_key_check before cleanup
```

Rules:

- never combine first v2 introduction with destructive legacy cleanup;
- if a migration changes canonical history shape, add old DB fixtures and post-migration restore drills;
- down migrations for production data can be limited, but recovery/backup path must be explicit;
- any irreversible step needs an audit row and a user-visible release note.

## Schema constraint policy

SQLite is permissive by default. For persistence v2, the schema should be defensive without making future product evolution painful.

Chosen policy: 🎯 9   🛡️ 9   🧠 6

1. **Stable internal domains use Rust enums + SQL `CHECK`**.
   Examples: `status`, `private_mode`, `gap_kind`, `backup_kind`, `durability_profile`, bounded command lifecycle states.

2. **Extension/product domains remain `TEXT`, but validation moves into typed Rust constructors**.
   Examples: future backend-specific `event_type`, mux capability names, payload schema ids and unknown extension event names.

3. **Consider `STRICT` tables only after runtime SQLite gate is enforced**.
   `STRICT` is useful for new canonical tables, but it requires SQLite `3.37.0+`; packaged v2 should already exceed that through bundled SQLite. Do not retrofit `STRICT` into legacy tables until migration/recovery tooling is ready.

Example constraints:

```sql
status TEXT NOT NULL
    CHECK(status IN ('pending', 'claimed', 'done', 'failed', 'quarantined')),

private_mode INTEGER NOT NULL DEFAULT 0
    CHECK(private_mode IN (0, 1)),

event_seq_low BIGINT NOT NULL
    CHECK(event_seq_low >= 1)
```

Rules:

- if a Rust enum has no `Unknown(String)` variant, add a DB `CHECK`;
- if a DB `CHECK` is added, tests must verify invalid values fail;
- never rely on UI validation as the only protection for canonical tables;
- do not make extension-heavy event names rigid too early, or migrations will become noisy and risky;
- schema comments in this plan are logical shape, final migrations should apply constraints consistently.

## ID and foreign-key policy

ID rules:

- all domain row IDs are app-generated UUIDv7 strings;
- SQLite `rowid` may be used only for internal FTS/search tables where it is required;
- IDs must be generated before entering repository functions so tests can use deterministic fixtures;
- never use command text, cwd, backend refs or file paths as IDs.

Foreign-key rules:

```text
canonical history:
  sessions, panes, commit log, journal, stream segments, snapshots, command blocks
  -> no accidental cascading through session/pane/commit refs
  -> deletion only through explicit user/delete/retention flow

derived data:
  search docs, redacted previews, temporary projections
  -> may cascade or rebuild

audit data:
  tombstones, maintenance runs, restore drills, export/delete requests
  -> preserve rows even when source rows are gone where privacy allows
```

Table-class matrix:

| Class | Examples | Source of truth | FK delete policy | Rebuildable | Test requirement |
| --- | --- | --- | --- | --- | --- |
| canonical | `terminal_commit_log`, `terminal_stream_segments`, `terminal_journal_events`, `terminal_command_blocks`, `terminal_screen_snapshots` | yes | `RESTRICT` / default no-action from session/pane/commit refs | no | direct parent delete fails |
| derived | `terminal_search_documents`, redacted previews, cached projections | no | `CASCADE` allowed when source deleted by service flow | yes | rebuild from canonical data |
| audit | `terminal_restore_drills`, `terminal_backup_records`, `terminal_export_requests`, `terminal_deletion_tombstones` | evidence | usually `SET NULL` plus copied non-sensitive identity | no, but append-only | survives source cleanup where privacy allows |
| workflow | `terminal_outbox_messages`, `terminal_delete_requests`, `terminal_idempotency_keys` | operational | depends on workflow owner, never deletes canonical rows implicitly | partially | stuck/expired state recovery |
| ephemeral | temp import buffers, transient locks | no | no long-lived FK promise | yes | can be discarded on restart |
| external artifact | future segment files, backup files | canonical only if referenced by manifest | DB row controls lifecycle, file delete via service | no | checksum/path identity verified before trust |

Important:

- `ON DELETE CASCADE` from `terminal_sessions` is not allowed on canonical tables in first v2 migrations.
- Direct `DELETE FROM terminal_sessions` / `terminal_panes` should fail when canonical history exists.
- Explicit full-session delete must be a service workflow: create delete request, write tombstone/export audit, delete child rows in chunks, then delete parent metadata last if policy allows.
- `terminal_commit_log` is append-only; child rows referencing it use `ON DELETE RESTRICT` or default no-action.
- Derived rows may cascade because they are rebuildable or privacy-scoped.
- Audit/tombstone rows usually use `ON DELETE SET NULL` and keep non-sensitive identity fields.
- Do not add DB triggers for destructive behavior in v1. Keep delete flow visible in Rust services and tests.

### Table: `terminal_db_identity`

Purpose: typed guard that the opened SQLite file belongs to this product/schema family.

```sql
CREATE TABLE terminal_db_identity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    product TEXT NOT NULL,
    schema_family TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    app_version TEXT,
    diesel_version TEXT,
    sqlite_version TEXT,
    notes TEXT
);
```

Rules:

- `PRAGMA application_id` is a fast SQLite-level guard;
- `terminal_db_identity` is the typed application-level guard;
- if the row is missing in an old DB, migrations may create it;
- if the row exists but mismatches product/schema family, fail closed.

### Table: `terminal_payload_schemas`

Purpose: versioned contract for JSON payload columns.

```sql
CREATE TABLE terminal_payload_schemas (
    id TEXT PRIMARY KEY,
    payload_kind TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    rust_type_name TEXT NOT NULL,
    json_schema TEXT NOT NULL,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    UNIQUE(payload_kind, schema_version)
);
```

Rules:

- every `payload_json`, `route_json`, `launch_json`, `manifest_json`, `metrics_json`, `error_json` shape must have a Rust struct first;
- derive/emit JSON schema with `schemars` for persisted payloads;
- store `schema_version` in rows that contain versioned JSON;
- migrations must include old payload fixtures and upgrade readers;
- never store unbounded `serde_json::Value` as canonical data without a versioned wrapper.

### Table: `terminal_projection_versions`

Purpose: parser/projection version registry for snapshots, search docs and command projections.

```sql
CREATE TABLE terminal_projection_versions (
    id TEXT PRIMARY KEY,
    projection_kind TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    projection_version TEXT NOT NULL,
    schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    created_at_ms BIGINT NOT NULL,
    UNIQUE(projection_kind, parser_version, projection_version)
);
```

Rules:

- snapshots/search docs/command projections record the parser/projection version that produced them;
- when parser behavior changes, mark affected derived rows stale through outbox jobs;
- canonical raw stream segments do not need rewriting for parser upgrades;
- restore should prefer latest valid projection but can fall back to replaying raw stream.

### Table: `terminal_sessions`

Purpose: durable session identity and restore metadata.

```sql
CREATE TABLE terminal_sessions (
    id TEXT PRIMARY KEY,
    route_json TEXT NOT NULL,
    backend_kind TEXT NOT NULL,
    title TEXT,
    launch_json TEXT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    closed_at_ms BIGINT,
    restore_generation BIGINT NOT NULL DEFAULT 0,
    persistence_version INTEGER NOT NULL,
    private_mode INTEGER NOT NULL DEFAULT 0,
    retention_policy_id TEXT NOT NULL DEFAULT 'default_full_history'
        REFERENCES terminal_retention_policies(id) ON DELETE RESTRICT
);
```

Indexes:

```sql
CREATE INDEX idx_terminal_sessions_updated_at
ON terminal_sessions(updated_at_ms DESC);

CREATE INDEX idx_terminal_sessions_backend
ON terminal_sessions(backend_kind, updated_at_ms DESC);
```

### Table: `terminal_retention_policies`

Purpose: explicit storage policy. "Full history" must not mean uncontrolled hidden behavior.

```sql
CREATE TABLE terminal_retention_policies (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    policy_kind TEXT NOT NULL,
    raw_output_policy TEXT NOT NULL,
    command_policy TEXT NOT NULL,
    snapshot_policy TEXT NOT NULL,
    max_raw_output_bytes BIGINT,
    max_age_ms BIGINT,
    delete_raw_on_private_mode INTEGER NOT NULL DEFAULT 1,
    warn_only_before_delete INTEGER NOT NULL DEFAULT 1,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL
);
```

Default policies:

```text
default_full_history:
  raw_output_policy = keep_until_user_deletes
  warn_only_before_delete = true
  no silent pruning of canonical journal/segments

private_ephemeral:
  raw_output_policy = do_not_persist_raw
  command_policy = minimal_metadata

performance_bounded:
  raw_output_policy = bounded_by_bytes_or_age
  must render visible retention gaps
```

Rules:

- the default product behavior must not silently delete raw history;
- storage pressure should produce user-visible diagnostics first;
- any policy that prunes canonical stream data must write `terminal_history_gaps` or deletion tombstones;
- derived data may be compacted/rebuilt more aggressively than canonical journal.

### Table: `terminal_maintenance_runs`

Purpose: durable audit for checkpoints, optimize, vacuum and retention.

```sql
CREATE TABLE terminal_maintenance_runs (
    id TEXT PRIMARY KEY,
    run_kind TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    status TEXT NOT NULL,
    selected_policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    wal_checkpoint_json TEXT,
    optimize_json TEXT,
    vacuum_json TEXT,
    reclaimed_bytes_estimate BIGINT,
    affected_sessions_json TEXT,
    error_json TEXT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_maintenance_runs_time
ON terminal_maintenance_runs(started_at_ms DESC);
```

Maintenance rules:

- never run heavy vacuum/checkpoint work on the PTY capture hot path;
- `wal_checkpoint(TRUNCATE)` belongs to startup/shutdown/maintenance windows;
- `PRAGMA optimize` is safe as scheduled maintenance, not per write;
- `auto_vacuum = INCREMENTAL` must be chosen before large production DBs exist, otherwise changing it later requires a VACUUM migration;
- retention deletes must be chunked and interruptible.

### Table: `terminal_integrity_checks`

Purpose: durable proof that DB, invariants and derived projections are internally consistent.

```sql
CREATE TABLE terminal_integrity_checks (
    id TEXT PRIMARY KEY,
    check_kind TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    status TEXT NOT NULL,
    sqlite_quick_check_json TEXT,
    foreign_key_check_json TEXT,
    invariant_results_json TEXT,
    projection_drift_json TEXT,
    sampled_session_ids_json TEXT,
    error_json TEXT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_integrity_checks_time
ON terminal_integrity_checks(started_at_ms DESC);
```

Rules:

- lightweight checks can run at startup/idle;
- full checks run on explicit user/dev command or scheduled maintenance;
- failed checks must not auto-delete data;
- support bundle can include redacted integrity reports by default.

### Table: `terminal_data_health_records`

Purpose: durable quarantine/degradation evidence when persisted data is suspicious or unusable.

```sql
CREATE TABLE terminal_data_health_records (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    row_kind TEXT NOT NULL,
    row_ref TEXT,
    detection_kind TEXT NOT NULL
        CHECK(detection_kind IN ('checksum_mismatch', 'decode_failed', 'parser_failed', 'projection_drift', 'missing_segment', 'migration_mismatch', 'manual')),
    severity TEXT NOT NULL
        CHECK(severity IN ('info', 'warning', 'degraded', 'critical')),
    status TEXT NOT NULL
        CHECK(status IN ('open', 'quarantined', 'ignored', 'resolved')),
    first_bad_event_seq BIGINT,
    expected_checksum TEXT,
    actual_checksum TEXT,
    action_taken TEXT NOT NULL
        CHECK(action_taken IN ('none', 'skip_snapshot', 'fallback_replay', 'mark_gap', 'quarantine_row', 'block_raw_export')),
    error_json TEXT,
    detected_at_ms BIGINT NOT NULL,
    resolved_at_ms BIGINT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_data_health_records_session
ON terminal_data_health_records(session_id, detected_at_ms DESC);
```

Recovery rules:

- corrupted derived snapshots can be skipped and rebuilt from canonical raw stream if checksums pass;
- corrupted canonical stream segments become degraded history and must render as gap/quarantine, not invisible omission;
- restore drill failures should link to health records where possible;
- raw export is blocked by default when critical health records are open for the requested scope;
- never auto-delete suspicious rows as a "repair"; mark, quarantine, rebuild derived projections, then let explicit recovery tooling decide.

### Table: `terminal_feature_gates`

Purpose: durable, inspectable rollout state for risky persistence capabilities.

```sql
CREATE TABLE terminal_feature_gates (
    id TEXT PRIMARY KEY,
    feature_name TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL
        CHECK(state IN ('disabled', 'shadow', 'enabled', 'force_disabled')),
    rollout_scope TEXT NOT NULL
        CHECK(rollout_scope IN ('global', 'session', 'backend', 'developer')),
    reason TEXT,
    enabled_at_ms BIGINT,
    disabled_at_ms BIGINT,
    updated_at_ms BIGINT NOT NULL,
    metadata_json TEXT
);
```

Gate rules:

- v2 persistence can run in `shadow` before becoming authoritative;
- mux capture, compression, raw export and encryption need separate gates;
- `force_disabled` must override config/env/UI toggles;
- gate changes should appear in diagnostics/support bundles;
- a gate must define downgrade behavior before it can be enabled.

Initial gates:

```text
terminal_persistence_v2_shadow
terminal_persistence_v2_capture
terminal_persistence_v2_authoritative
terminal_persistence_v2_authoritative_reads
mux_structured_capture
segment_compression_zstd
raw_history_export
encrypted_terminal_history
```

Gate state machine:

```text
disabled -> shadow -> enabled
disabled -> force_disabled
shadow -> force_disabled
enabled -> force_disabled
enabled -> disabled only through explicit rollback record
force_disabled -> disabled only through manual/admin recovery action
```

Initial gate matrix:

| Gate | Default | Enables | Required proof before enabled | Downgrade if disabled |
| --- | --- | --- | --- | --- |
| `terminal_persistence_v2_shadow` | `shadow` in dev/test, `disabled` in packaged build until migration tested | v2 writes beside legacy | migrations, identity check, legacy tests pass | no v2 writes |
| `terminal_persistence_v2_capture` | `disabled` | native capture events into writer | writer lease, cursor invariants, PTY nonblocking test | live terminal only, no durable stream claim |
| `terminal_persistence_v2_authoritative_reads` | `disabled` | saved session list/restore reads from v2 | restore drill, browser restart smoke, semantics v2 mapping | legacy visual snapshot reads |
| `terminal_persistence_v2_authoritative` | `disabled` | v2 is default persistence path | all MVP reliability gates pass | shadow/legacy fallback |
| `mux_structured_capture` | `disabled` | zellij/tmux structured adapters | fresh capability report and downgrade tests | mux shown as lower-fidelity/unsupported route |
| `segment_compression_zstd` | `disabled` | compressed cold segments | compressed restore drill and corruption tests | raw BLOB segments |
| `raw_history_export` | `disabled` | raw transcript export workflow | export request approval + redaction/health checks | redacted export only |
| `encrypted_terminal_history` | `disabled` | encrypted DB/artifact payloads | key-store probe, test vectors, recovery docs | plaintext v2 with clear diagnostics |

Gate invariants:

- `force_disabled` always wins over config/env/UI.
- `terminal_persistence_v2_authoritative_reads` cannot be enabled without `terminal_persistence_v2_capture` or a deliberate `legacy_visual_only` scope.
- `RichHistory` cannot be shown when the active gate state is `shadow` or `disabled`.
- gate changes must create maintenance/audit diagnostics so support can explain why history is degraded.

### Table: `terminal_crypto_keys`

Purpose: schema-ready key hierarchy for future encrypted terminal history.

```sql
CREATE TABLE terminal_crypto_keys (
    id TEXT PRIMARY KEY,
    key_kind TEXT NOT NULL
        CHECK(key_kind IN ('database_key', 'export_key', 'artifact_key')),
    key_ref TEXT NOT NULL,
    protection_kind TEXT NOT NULL
        CHECK(protection_kind IN ('windows_credential_manager', 'dpapi_user', 'dpapi_machine', 'macos_keychain', 'linux_secret_service', 'test_plaintext')),
    state TEXT NOT NULL
        CHECK(state IN ('active', 'rotating', 'disabled', 'destroyed', 'unavailable')),
    created_at_ms BIGINT NOT NULL,
    rotated_at_ms BIGINT,
    destroyed_at_ms BIGINT,
    capability_report_json TEXT,
    error_json TEXT
);

CREATE INDEX idx_terminal_crypto_keys_state
ON terminal_crypto_keys(state, created_at_ms DESC);
```

Key rules:

- `key_ref` is an opaque reference, not key material;
- `test_plaintext` is allowed only in tests and must fail production startup;
- Windows default should be user-scoped protection, not machine-scoped, unless product requirements explicitly need machine-wide access;
- key access should be serialized through a small service, because platform key stores can have threading/ordering quirks;
- support bundles may include key state and protection kind, never key bytes.

### Table: `terminal_crypto_key_events`

Purpose: auditable key lifecycle without exposing secrets.

```sql
CREATE TABLE terminal_crypto_key_events (
    id TEXT PRIMARY KEY,
    key_id TEXT REFERENCES terminal_crypto_keys(id) ON DELETE SET NULL,
    event_kind TEXT NOT NULL
        CHECK(event_kind IN ('created', 'unlocked', 'lock_failed', 'rotated', 'destroy_requested', 'destroyed', 'recovery_failed')),
    actor TEXT NOT NULL,
    occurred_at_ms BIGINT NOT NULL,
    status TEXT NOT NULL
        CHECK(status IN ('succeeded', 'failed', 'skipped')),
    error_json TEXT
);

CREATE INDEX idx_terminal_crypto_key_events_key_time
ON terminal_crypto_key_events(key_id, occurred_at_ms DESC);
```

### Table: `terminal_backup_records`

Purpose: durable record for safe database backup and restore artifacts.

```sql
CREATE TABLE terminal_backup_records (
    id TEXT PRIMARY KEY,
    backup_kind TEXT NOT NULL
        CHECK(backup_kind IN ('online_backup_api', 'vacuum_into', 'logical_redacted_export', 'manual_external')),
    requested_by TEXT NOT NULL,
    source_db_path_hash TEXT,
    destination_ref TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    status TEXT NOT NULL
        CHECK(status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')),
    sqlite_backup_pages_total INTEGER,
    sqlite_backup_pages_remaining INTEGER,
    checkpoint_before_backup INTEGER NOT NULL DEFAULT 0
        CHECK(checkpoint_before_backup IN (0, 1)),
    integrity_check_id TEXT REFERENCES terminal_integrity_checks(id) ON DELETE SET NULL,
    manifest_json TEXT,
    checksum_algorithm TEXT,
    checksum TEXT,
    error_json TEXT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_backup_records_time
ON terminal_backup_records(started_at_ms DESC);
```

Backup rules:

- default v1 DB backup path is `VACUUM INTO` through the persistence executor, because it is supported as a normal SQLite command and keeps repository writes on Diesel;
- SQLite Online Backup API is the v2 incremental/progress path, but implement it only through a tiny isolated `sqlite_backup` adapter over `libsqlite3-sys`, not inside repositories;
- normal domain reads/writes still use Diesel ORM/query builder; PRAGMAs, migrations, `VACUUM INTO` and backup FFI are operational database control, not application query logic;
- do not implement backup as a plain hot copy of only the `.db` file while WAL is enabled;
- if a file-level copy is ever added for admin/debug tooling, it must be quiesced or include a controlled checkpoint and all required WAL state;
- every raw DB backup should write a manifest with SQLite version, PRAGMAs, DB identity, checksum and source build metadata;
- post-backup `quick_check` should run when budget allows and link through `integrity_check_id`.

### Table: `terminal_storage_pressure_events`

Purpose: durable record that the persistence layer saw disk/quota/temp-space pressure and changed behavior visibly.

```sql
CREATE TABLE terminal_storage_pressure_events (
    id TEXT PRIMARY KEY,
    event_kind TEXT NOT NULL
        CHECK(event_kind IN ('disk_full', 'temp_full', 'wal_bloat', 'quota_near_limit', 'quota_exceeded', 'manual_probe')),
    db_file_bytes BIGINT,
    wal_file_bytes BIGINT,
    free_space_bytes BIGINT,
    writer_queue_depth INTEGER,
    affected_session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    started_at_ms BIGINT NOT NULL,
    resolved_at_ms BIGINT,
    status TEXT NOT NULL
        CHECK(status IN ('active', 'resolved', 'ignored', 'failed')),
    action_taken TEXT NOT NULL
        CHECK(action_taken IN ('warn_only', 'checkpoint_requested', 'writer_degraded', 'capture_gap', 'retention_requested', 'none')),
    error_json TEXT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_storage_pressure_active
ON terminal_storage_pressure_events(status, started_at_ms DESC);
```

Storage pressure rules:

- `SQLITE_FULL` and relevant `SQLITE_IOERR_*` variants become typed persistence errors, not generic query failures;
- if writer cannot persist complete history, mark the affected session as degraded and emit `terminal_history_gaps` when possible;
- do not auto-delete canonical history as the first reaction to pressure;
- offer user-visible remediation: free disk, run backup/export, change retention policy, or explicitly prune with tombstones;
- capture `db_file_bytes`, `wal_file_bytes` and `free_space_bytes` best-effort for diagnostics;
- temp-space full matters too, because SQLite can fail on temporary files even when the main DB volume has space.

### Table: `terminal_panes`

Purpose: pane identity and backend mapping.

```sql
CREATE TABLE terminal_panes (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    tab_id TEXT,
    backend_pane_ref TEXT,
    title TEXT,
    created_at_ms BIGINT NOT NULL,
    closed_at_ms BIGINT,
    last_event_seq BIGINT NOT NULL DEFAULT 0,
    rows INTEGER,
    cols INTEGER,
    shell_kind TEXT,
    execution_domain TEXT NOT NULL DEFAULT 'local_windows'
);

CREATE INDEX idx_terminal_panes_session
ON terminal_panes(session_id);
```

### Table: `terminal_backend_capability_reports`

Purpose: durable evidence for restore guarantees per backend/session/pane.

```sql
CREATE TABLE terminal_backend_capability_reports (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    backend_kind TEXT NOT NULL,
    backend_version TEXT,
    probe_kind TEXT NOT NULL,
    capture_strategy TEXT NOT NULL,
    can_preserve_process_when_live INTEGER NOT NULL DEFAULT 0
        CHECK(can_preserve_process_when_live IN (0, 1)),
    can_capture_raw_stream INTEGER NOT NULL DEFAULT 0
        CHECK(can_capture_raw_stream IN (0, 1)),
    can_capture_rendered_scrollback INTEGER NOT NULL DEFAULT 0
        CHECK(can_capture_rendered_scrollback IN (0, 1)),
    can_stream_rendered_updates INTEGER NOT NULL DEFAULT 0
        CHECK(can_stream_rendered_updates IN (0, 1)),
    can_query_layout INTEGER NOT NULL DEFAULT 0
        CHECK(can_query_layout IN (0, 1)),
    command_boundary_confidence TEXT NOT NULL
        CHECK(command_boundary_confidence IN ('high', 'medium', 'low', 'none')),
    output_fidelity TEXT NOT NULL
        CHECK(output_fidelity IN ('raw_replayable', 'rendered_ansi', 'rendered_plaintext', 'visual_only', 'unknown')),
    checked_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT,
    evidence_json TEXT,
    error_json TEXT
);

CREATE INDEX idx_terminal_backend_capability_reports_session
ON terminal_backend_capability_reports(session_id, checked_at_ms DESC);
```

Rules:

- saved session restore badges derive from capability reports plus restore drills, not backend name alone;
- backend upgrades should expire/reprobe old reports;
- `evidence_json` should store command names and summarized probe result, not raw transcript;
- if no report exists, UI falls back to conservative `visual_restore_only` / `unknown` semantics.

### Table: `terminal_writer_generations`

Purpose: process-level writer ownership, diagnostics and stale writer recovery.

```sql
CREATE TABLE terminal_writer_generations (
    id TEXT PRIMARY KEY,
    process_ref TEXT NOT NULL,
    host_ref TEXT NOT NULL,
    started_at_ms BIGINT NOT NULL,
    heartbeat_at_ms BIGINT NOT NULL,
    lease_until_ms BIGINT NOT NULL,
    state TEXT NOT NULL,
    app_version TEXT,
    pid INTEGER,
    notes TEXT
);

CREATE INDEX idx_terminal_writer_generations_state
ON terminal_writer_generations(state, lease_until_ms);
```

Rules:

- the active writer renews its lease periodically through the persistence executor;
- startup may take over only if previous lease expired or previous writer is marked closed;
- stale takeover writes a maintenance/audit row;
- this protects correctness, but live PTY ownership still belongs to runtime/session layer.

### Table: `terminal_clock_anchors`

Purpose: detect wall-clock jumps while preserving monotonic event ordering.

```sql
CREATE TABLE terminal_clock_anchors (
    id TEXT PRIMARY KEY,
    writer_generation TEXT NOT NULL REFERENCES terminal_writer_generations(id) ON DELETE RESTRICT,
    wall_time_ms BIGINT NOT NULL,
    monotonic_ms BIGINT NOT NULL,
    source TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_clock_anchors_writer
ON terminal_clock_anchors(writer_generation, created_at_ms);
```

Rules:

- event ordering uses `commit_seq` and pane `event_seq` first;
- wall time is for UX, search and diagnostics;
- if wall time jumps backwards/forwards, record anchor and expose diagnostic.

### Table: `terminal_session_cursors`

Purpose: durable session-level commit sequence allocator.

```sql
CREATE TABLE terminal_session_cursors (
    session_id TEXT PRIMARY KEY REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    next_commit_seq BIGINT NOT NULL DEFAULT 1,
    writer_generation TEXT,
    updated_at_ms BIGINT NOT NULL,
    CHECK(next_commit_seq >= 1)
);
```

Rules:

- `next_commit_seq` is allocated only inside the persistence writer transaction;
- commit sequence is session-local, not global across all sessions;
- this does not replace per-pane stream `event_seq`, it complements it.
- this table is allocator/workflow state, not canonical history; `ON DELETE CASCADE` is acceptable only because canonical commit/journal rows restrict direct parent deletion first.

### Table: `terminal_commit_log`

Purpose: atomic ordering and audit of multi-pane persistence transactions.

```sql
CREATE TABLE terminal_commit_log (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    commit_kind TEXT NOT NULL,
    writer_generation TEXT NOT NULL,
    pane_high_water_json TEXT NOT NULL,
    operation_id TEXT,
    idempotency_key_id TEXT REFERENCES terminal_idempotency_keys(id) ON DELETE SET NULL,
    created_at_ms BIGINT NOT NULL,
    UNIQUE(session_id, commit_seq)
);

CREATE INDEX idx_terminal_commit_log_session_seq
ON terminal_commit_log(session_id, commit_seq);
```

Rules:

- every write transaction that changes canonical history gets one commit row;
- `pane_high_water_json` maps pane/stream refs to the high-water `event_seq` after the commit;
- topology snapshots and restore plans should anchor to `commit_seq`, not only wall-clock time;
- operation idempotency is still handled by `terminal_idempotency_keys`, because SQLite nullable unique columns are not enough.

### Table: `terminal_deletion_tombstones`

Purpose: audit user deletion and retention-pruning decisions without keeping deleted payload.

```sql
CREATE TABLE terminal_deletion_tombstones (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    deletion_kind TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_ref TEXT,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    reason TEXT NOT NULL,
    policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    deleted_at_ms BIGINT NOT NULL,
    actor_kind TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX idx_terminal_deletion_tombstones_session
ON terminal_deletion_tombstones(session_id, deleted_at_ms);
```

Rules:

- tombstones must not store raw deleted output;
- user deletion and policy pruning are different `deletion_kind` values;
- restore UI can show that history was intentionally removed instead of treating it as corruption.

### Table: `terminal_delete_requests`

Purpose: explicit lifecycle for user delete, retention pruning and future crypto erase.

```sql
CREATE TABLE terminal_delete_requests (
    id TEXT PRIMARY KEY,
    request_kind TEXT NOT NULL,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    target_kind TEXT NOT NULL,
    target_ref TEXT,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    requested_by TEXT NOT NULL,
    requested_at_ms BIGINT NOT NULL,
    approved_at_ms BIGINT,
    completed_at_ms BIGINT,
    status TEXT NOT NULL,
    policy_id TEXT REFERENCES terminal_retention_policies(id) ON DELETE SET NULL,
    result_json TEXT,
    error_json TEXT,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX idx_terminal_delete_requests_session
ON terminal_delete_requests(session_id, requested_at_ms DESC);
```

Rules:

- destructive deletion flows create a request row first;
- service code deletes/prunes in chunks and writes tombstones;
- failed deletes must be resumable and visible;
- private mode may bypass raw persistence, but should still record minimal policy/audit metadata if configured.

### Table: `terminal_stream_cursors`

Purpose: durable event and byte sequence allocator per pane/stream.

```sql
CREATE TABLE terminal_stream_cursors (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    next_event_seq BIGINT NOT NULL DEFAULT 1,
    next_byte_seq BIGINT NOT NULL DEFAULT 0,
    updated_at_ms BIGINT NOT NULL,
    UNIQUE(pane_id, stream_id),
    CHECK(next_event_seq >= 1),
    CHECK(next_byte_seq >= 0)
);

CREATE INDEX idx_terminal_stream_cursors_session
ON terminal_stream_cursors(session_id);
```

Rules:

- `terminal_stream_cursors.next_event_seq` is allocator state for journal events;
- `terminal_stream_cursors.next_byte_seq` is allocator state for captured payload byte ranges;
- `terminal_panes.last_event_seq` is a denormalized read model for quick diagnostics;
- writer restart must load/create cursor in the same transaction that persists first new event or segment;
- never allocate event or byte sequence numbers in browser/client code.
- this table may cascade with the parent only after canonical stream/journal rows were removed by the audited delete service.

### Table: `terminal_topology_snapshots`

Purpose: durable layout tree, tabs, splits and focus state.

Why this table is mandatory:

- `terminal_panes` tells which panes exist, but not split tree shape;
- restore needs tab order, split direction, split ratio and focus;
- legacy `TopologySnapshot` already has the right product shape;
- topology is a snapshot/read model and should be versioned separately from stream events.

```sql
CREATE TABLE terminal_topology_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    topology_version INTEGER NOT NULL,
    source TEXT NOT NULL,
    base_commit_seq BIGINT,
    high_water_commit_seq BIGINT NOT NULL,
    pane_high_water_json TEXT NOT NULL,
    focused_tab_id TEXT,
    payload_json TEXT NOT NULL,
    encryption_state TEXT NOT NULL DEFAULT 'plaintext'
        CHECK(encryption_state IN ('plaintext', 'encrypted', 'pending_reencrypt', 'crypto_erased')),
    key_ref TEXT,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_topology_snapshots_session_seq
ON terminal_topology_snapshots(session_id, high_water_commit_seq DESC, created_at_ms DESC);
```

Rule:

- topology snapshots can be overwritten/compacted by policy;
- journal events still record topology mutations;
- latest valid topology snapshot is used for fast restore.
- `pane_high_water_json` is required because one scalar `event_seq` cannot describe a multi-pane session state.

### Table: `terminal_stream_segments`

Purpose: captured output chunks, canonical payload for native raw streams and lower-fidelity rendered mux streams.

```sql
CREATE TABLE terminal_stream_segments (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    stream_id TEXT NOT NULL,
    event_seq_low BIGINT NOT NULL,
    event_seq_high BIGINT NOT NULL,
    byte_low BIGINT NOT NULL,
    byte_high BIGINT NOT NULL,
    time_low_ms BIGINT NOT NULL,
    time_high_ms BIGINT NOT NULL,
    capture_layer TEXT NOT NULL,
    capture_semantics TEXT NOT NULL DEFAULT 'raw_vt_stream'
        CHECK(capture_semantics IN ('raw_vt_stream', 'rendered_ansi_stream', 'rendered_plaintext_snapshot', 'mux_structured_surface', 'imported_text')),
    payload_kind TEXT NOT NULL,
    codec TEXT NOT NULL DEFAULT 'raw',
    codec_level INTEGER,
    encryption_state TEXT NOT NULL DEFAULT 'plaintext'
        CHECK(encryption_state IN ('plaintext', 'encrypted', 'pending_reencrypt', 'crypto_erased')),
    key_ref TEXT,
    payload BLOB,
    artifact_ref TEXT,
    stored_byte_len INTEGER NOT NULL,
    uncompressed_byte_len INTEGER NOT NULL,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    redaction_state TEXT NOT NULL DEFAULT 'raw_unscanned',
    created_at_ms BIGINT NOT NULL,
    UNIQUE(pane_id, stream_id, event_seq_low),
    CHECK(event_seq_low <= event_seq_high),
    UNIQUE(pane_id, stream_id, byte_low),
    CHECK(byte_low < byte_high),
    CHECK(payload IS NOT NULL OR artifact_ref IS NOT NULL)
);

CREATE INDEX idx_terminal_stream_segments_pane_event_seq
ON terminal_stream_segments(pane_id, event_seq_low, event_seq_high);

CREATE INDEX idx_terminal_stream_segments_pane_byte_seq
ON terminal_stream_segments(pane_id, byte_low, byte_high);

CREATE INDEX idx_terminal_stream_segments_session_time
ON terminal_stream_segments(session_id, time_low_ms, time_high_ms);
```

Payload policy:

- v1 uses `codec = 'raw'` and SQLite `payload` BLOB for normal native terminal output;
- `artifact_ref` stays reserved for future large/external artifacts;
- `checksum_algorithm = 'blake3'` and `checksum` are computed over stored bytes;
- `stored_byte_len` is bytes stored in SQLite/artifact, `uncompressed_byte_len` is bytes after decompression;
- raw terminal bytes are not assumed to be valid UTF-8; UTF-8 decoding belongs to derived text projections only;
- only `capture_semantics = 'raw_vt_stream'` is replayable through a terminal emulator as canonical raw output;
- rendered mux surfaces are persisted as lower-fidelity visual evidence and must not be treated as raw VT history;
- `event_seq_*` ranges identify journal ordering, while `byte_*` ranges identify exact payload bytes;
- text projections must carry byte ranges so partial UTF-8 boundaries and binary output can be traced back to canonical bytes;
- future `codec = 'zstd'` must be opt-in through a compression iteration, not silently enabled in v1;
- after encryption, storage checksum is over ciphertext/artifact bytes; plaintext hashes must not leak unless stored inside encrypted metadata.

### Table: `terminal_journal_events`

Purpose: structured event envelope for output/input/resize/markers/snapshots.

```sql
CREATE TABLE terminal_journal_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    event_scope_kind TEXT NOT NULL,
    event_scope_id TEXT NOT NULL,
    stream_id TEXT NOT NULL,
    event_seq BIGINT NOT NULL,
    byte_low BIGINT,
    byte_high BIGINT,
    source_event_id_hash TEXT,
    event_kind TEXT NOT NULL,
    buffer_kind TEXT NOT NULL DEFAULT 'normal',
    capture_layer TEXT NOT NULL,
    occurred_at_ms BIGINT NOT NULL,
    monotonic_delta_us BIGINT,
    payload_json TEXT NOT NULL,
    encryption_state TEXT NOT NULL DEFAULT 'plaintext'
        CHECK(encryption_state IN ('plaintext', 'encrypted', 'pending_reencrypt', 'crypto_erased')),
    key_ref TEXT,
    payload_schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    segment_id TEXT REFERENCES terminal_stream_segments(id) ON DELETE SET NULL,
    trust_level TEXT NOT NULL DEFAULT 'terminal_observed',
    capture_semantics TEXT NOT NULL DEFAULT 'raw_vt_stream'
        CHECK(capture_semantics IN ('raw_vt_stream', 'rendered_ansi_stream', 'rendered_plaintext_snapshot', 'mux_structured_surface', 'imported_text')),
    schema_version INTEGER NOT NULL,
    UNIQUE(event_scope_kind, event_scope_id, stream_id, event_seq),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX idx_terminal_journal_events_pane_event_seq
ON terminal_journal_events(pane_id, event_seq);

CREATE INDEX idx_terminal_journal_events_scope_event_seq
ON terminal_journal_events(event_scope_kind, event_scope_id, event_seq);

CREATE INDEX idx_terminal_journal_events_kind_time
ON terminal_journal_events(event_kind, occurred_at_ms);

CREATE INDEX idx_terminal_journal_events_commit
ON terminal_journal_events(session_id, commit_seq);
```

Important:

- `event_scope_kind` is usually `pane` for output/input/resize and `session` for topology/session lifecycle events.
- `event_seq` is scoped by `event_scope_kind/event_scope_id/stream_id`; it is not a byte offset.
- `byte_low/byte_high` is optional and only present when the event can be mapped to exact payload bytes.
- Do not rely on `UNIQUE(pane_id, stream_id, event_seq)` because SQLite permits multiple `NULL` values in unique columns.
- `source_event_id_hash` is optional because not every backend can provide stable IDs; when available, it is enforced through `terminal_capture_receipts`.
- `schema_version` is the journal envelope version; payload-specific schema lives in `payload_schema_id`.

### Table: `terminal_capture_receipts`

Purpose: retry-safe ingestion of capture events/batches from backend/runtime.

```sql
CREATE TABLE terminal_capture_receipts (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    source_kind TEXT NOT NULL,
    source_event_id_hash TEXT NOT NULL,
    source_payload_hash TEXT NOT NULL,
    commit_id TEXT REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    state TEXT NOT NULL,
    first_seen_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    UNIQUE(session_id, source_kind, source_event_id_hash)
);

CREATE INDEX idx_terminal_capture_receipts_session
ON terminal_capture_receipts(session_id, last_seen_at_ms);
```

Rules:

- receipt check and commit insert happen in one `BEGIN IMMEDIATE` transaction;
- same source event id with different payload hash is a corruption/bug signal;
- generated source ids must be stable across retry, not random per attempt;
- if backend cannot provide stable source event ids, writer still remains ordered but duplicate prevention confidence is lower.

### Table: `terminal_command_blocks`

Purpose: user-visible command lifecycle.

```sql
CREATE TABLE terminal_command_blocks (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT,
    command_text TEXT,
    command_text_source TEXT NOT NULL,
    cwd TEXT,
    cwd_source TEXT,
    started_event_seq BIGINT,
    ended_event_seq BIGINT,
    output_event_seq_low BIGINT,
    output_event_seq_high BIGINT,
    output_byte_low BIGINT,
    output_byte_high BIGINT,
    started_at_ms BIGINT,
    finished_at_ms BIGINT,
    exit_code INTEGER,
    status TEXT NOT NULL,
    trust_level TEXT NOT NULL,
    shell_integration_protocol TEXT,
    sensitivity_class TEXT NOT NULL DEFAULT 'unknown',
    redaction_state TEXT NOT NULL DEFAULT 'unscanned',
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    CHECK((output_event_seq_low IS NULL AND output_event_seq_high IS NULL) OR (output_event_seq_low IS NOT NULL AND output_event_seq_high IS NOT NULL AND output_event_seq_low <= output_event_seq_high)),
    CHECK((output_byte_low IS NULL AND output_byte_high IS NULL) OR (output_byte_low IS NOT NULL AND output_byte_high IS NOT NULL AND output_byte_low < output_byte_high))
);

CREATE INDEX idx_terminal_command_blocks_pane_time
ON terminal_command_blocks(pane_id, started_at_ms);

CREATE INDEX idx_terminal_command_blocks_status
ON terminal_command_blocks(status, finished_at_ms);
```

Rules:

- `started_event_seq`/`ended_event_seq` follow shell markers or trusted UI submit/finish events.
- `output_event_seq_low/output_event_seq_high` is the canonical command output range for restore/search/export.
- `output_byte_low/output_byte_high` is optional and must be filled only when raw payload byte ownership is known.
- rendered mux output can create a visible command block only with lower trust; it should not invent exact byte ranges.

### Table: `terminal_command_history_entries`

Purpose: command dock/autocomplete/rerun history independent from terminal rendering.

```sql
CREATE TABLE terminal_command_history_entries (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    command_block_id TEXT REFERENCES terminal_command_blocks(id) ON DELETE SET NULL,
    scope_kind TEXT NOT NULL,
    command_text TEXT,
    display_text TEXT NOT NULL,
    redacted_text TEXT,
    command_hash_algorithm TEXT NOT NULL DEFAULT 'blake3',
    command_hash_scope TEXT NOT NULL DEFAULT 'local_keyed',
    command_hash TEXT NOT NULL,
    cwd TEXT,
    shell_kind TEXT,
    trust_level TEXT NOT NULL,
    source TEXT NOT NULL,
    sensitivity_class TEXT NOT NULL DEFAULT 'unknown',
    redaction_state TEXT NOT NULL DEFAULT 'unscanned',
    rerun_policy TEXT NOT NULL DEFAULT 'confirm',
    first_used_at_ms BIGINT NOT NULL,
    last_used_at_ms BIGINT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 1,
    disabled_for_rerun INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_terminal_command_history_scope
ON terminal_command_history_entries(scope_kind, last_used_at_ms DESC);

CREATE INDEX idx_terminal_command_history_session
ON terminal_command_history_entries(session_id, last_used_at_ms DESC);
```

Rules:

- canonical command lifecycle remains `terminal_command_blocks`;
- command history entries are derived/curated for UX and can be rebuilt from trusted blocks;
- command history is persisted in DB, not only browser localStorage;
- commands may contain secrets, so `display_text` and `redacted_text` are what autocomplete/search should use by default;
- `command_text` may be `NULL` if private mode or redaction policy forbids raw command persistence;
- `command_hash` is for local dedupe and should be keyed/peppered where practical; do not export it as a stable cross-machine fingerprint by default;
- `rerun_policy` can be `allow`, `confirm`, `disabled_sensitive`, `disabled_untrusted`;
- low-trust entries can be shown for history but default to `disabled_for_rerun = 1`.

### Table: `terminal_screen_snapshots`

Purpose: fast restore/hydration points.

```sql
CREATE TABLE terminal_screen_snapshots (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT NOT NULL REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT NOT NULL REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT NOT NULL,
    snapshot_kind TEXT NOT NULL,
    buffer_kind TEXT NOT NULL DEFAULT 'normal',
    base_event_seq BIGINT NOT NULL,
    high_water_event_seq BIGINT NOT NULL,
    high_water_byte_seq BIGINT,
    rows INTEGER NOT NULL,
    cols INTEGER NOT NULL,
    projection_source TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    encryption_state TEXT NOT NULL DEFAULT 'plaintext'
        CHECK(encryption_state IN ('plaintext', 'encrypted', 'pending_reencrypt', 'crypto_erased')),
    key_ref TEXT,
    payload_schema_id TEXT REFERENCES terminal_payload_schemas(id) ON DELETE RESTRICT,
    checksum_algorithm TEXT NOT NULL DEFAULT 'blake3',
    checksum TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    CHECK(base_event_seq <= high_water_event_seq),
    CHECK(high_water_byte_seq IS NULL OR high_water_byte_seq >= 0)
);

CREATE INDEX idx_terminal_screen_snapshots_pane_event_seq
ON terminal_screen_snapshots(pane_id, high_water_event_seq DESC);
```

Rule:

- screen snapshot is per-pane, so `high_water_event_seq` is pane-local;
- `high_water_byte_seq` is optional because rendered/imported sources may not map cleanly to raw byte offsets;
- session-level restore consistency comes from `commit_seq` and topology `pane_high_water_json`.

### Table: `terminal_outbox_messages`

Purpose: durable worker jobs committed with journal changes.

```sql
CREATE TABLE terminal_outbox_messages (
    id TEXT PRIMARY KEY,
    dedupe_key TEXT,
    aggregate_kind TEXT NOT NULL,
    aggregate_ref TEXT NOT NULL,
    event_type TEXT NOT NULL,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_attempt_at_ms BIGINT,
    claimed_by TEXT,
    claim_token TEXT,
    claimed_until_ms BIGINT,
    last_error_json TEXT,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high))
);

CREATE INDEX idx_terminal_outbox_pending
ON terminal_outbox_messages(status, next_attempt_at_ms, created_at_ms);

CREATE INDEX idx_terminal_outbox_claimed_until
ON terminal_outbox_messages(status, claimed_until_ms);

CREATE UNIQUE INDEX idx_terminal_outbox_dedupe_not_null
ON terminal_outbox_messages(dedupe_key)
WHERE dedupe_key IS NOT NULL;
```

### Table: `terminal_idempotency_keys`

Purpose: retry-safe user/runtime operations.

```sql
CREATE TABLE terminal_idempotency_keys (
    id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    scope_ref TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    idempotency_key_hash TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    result_json TEXT,
    state TEXT NOT NULL,
    first_seen_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL,
    UNIQUE(scope_kind, scope_ref, operation_kind, idempotency_key_hash)
);
```

### Table: `terminal_clients`

Purpose: durable local client identity for reconnect, ack and replay state.

```sql
CREATE TABLE terminal_clients (
    id TEXT PRIMARY KEY,
    client_kind TEXT NOT NULL,
    install_ref_hash TEXT,
    browser_profile_ref_hash TEXT,
    user_agent_hash TEXT,
    created_at_ms BIGINT NOT NULL,
    last_seen_at_ms BIGINT NOT NULL,
    trust_state TEXT NOT NULL DEFAULT 'local_unverified'
);

CREATE INDEX idx_terminal_clients_seen
ON terminal_clients(last_seen_at_ms DESC);
```

Rules:

- this is local reliability identity, not auth identity;
- never trust browser-provided client IDs as security proof;
- raw user-agent/browser profile values are not stored;
- if client identity is missing or reset, reconnect can still replay by explicit session/pane range but cannot rely on previous ack state.

### Table: `terminal_delivery_offsets`

Purpose: browser/client reconnect, ack and replay state.

```sql
CREATE TABLE terminal_delivery_offsets (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES terminal_clients(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE CASCADE,
    stream_id TEXT NOT NULL,
    last_sent_event_seq BIGINT NOT NULL DEFAULT 0,
    last_acked_event_seq BIGINT NOT NULL DEFAULT 0,
    last_persisted_event_seq BIGINT NOT NULL DEFAULT 0,
    replay_from_event_seq BIGINT,
    gap_state TEXT NOT NULL DEFAULT 'none',
    updated_at_ms BIGINT NOT NULL,
    UNIQUE(client_id, session_id, pane_id, stream_id)
);

CREATE INDEX idx_terminal_delivery_offsets_session_client
ON terminal_delivery_offsets(session_id, client_id);
```

Rule:

- delivery offsets are not canonical history;
- they only describe what a client saw/acked;
- replay is served from `terminal_journal_events` and `terminal_stream_segments` by event ranges, with segment byte ranges used for payload extraction.
- client identity is local reliability state, not authentication;
- do not store raw user-agent/browser profile values, store hashes or opaque local refs.

### Table: `terminal_history_gaps`

Purpose: durable visibility for known missing history.

```sql
CREATE TABLE terminal_history_gaps (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE RESTRICT,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE RESTRICT,
    commit_id TEXT REFERENCES terminal_commit_log(id) ON DELETE RESTRICT,
    commit_seq BIGINT,
    stream_id TEXT NOT NULL,
    gap_kind TEXT NOT NULL,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    started_at_ms BIGINT NOT NULL,
    ended_at_ms BIGINT,
    estimated_dropped_bytes BIGINT,
    estimated_dropped_events BIGINT,
    reason TEXT NOT NULL,
    writer_generation TEXT,
    created_at_ms BIGINT NOT NULL,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX idx_terminal_history_gaps_session
ON terminal_history_gaps(session_id, started_at_ms);

CREATE INDEX idx_terminal_history_gaps_pane_event_seq
ON terminal_history_gaps(pane_id, event_seq_low, event_seq_high);
```

Rules:

- gap rows are canonical diagnostics, not a replacement for journal events;
- writer should also append `history_gap_started` / `history_gap_ended` journal events when possible;
- restore UI must render gaps instead of silently joining surrounding output;
- event range is used for replay placement, byte range is used for storage/accounting diagnostics when known.

### Table: `terminal_restore_drills`

Purpose: durable verification record that a session can be restored from persisted data.

```sql
CREATE TABLE terminal_restore_drills (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    run_kind TEXT NOT NULL,
    source_snapshot_id TEXT REFERENCES terminal_screen_snapshots(id) ON DELETE SET NULL,
    started_at_ms BIGINT NOT NULL,
    finished_at_ms BIGINT,
    status TEXT NOT NULL,
    highest_checked_commit_seq BIGINT,
    pane_high_water_json TEXT,
    checker_version TEXT NOT NULL,
    error_json TEXT,
    metrics_json TEXT,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX idx_terminal_restore_drills_session_time
ON terminal_restore_drills(session_id, started_at_ms DESC);
```

Rules:

- every production restore should be able to emit a drill report shape, even if not persisted;
- scheduled/background drills persist rows;
- saved session list can show the latest drill status as evidence, not just optimism.

### Table: `terminal_export_requests`

Purpose: explicit safe export workflow.

```sql
CREATE TABLE terminal_export_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES terminal_sessions(id) ON DELETE SET NULL,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE SET NULL,
    export_kind TEXT NOT NULL,
    scope_json TEXT NOT NULL,
    redaction_profile_id TEXT,
    include_raw_output INTEGER NOT NULL DEFAULT 0,
    requested_by TEXT NOT NULL,
    requested_at_ms BIGINT NOT NULL,
    approved_at_ms BIGINT,
    completed_at_ms BIGINT,
    status TEXT NOT NULL,
    manifest_json TEXT,
    artifact_ref TEXT,
    checksum_algorithm TEXT,
    checksum TEXT,
    error_json TEXT
);

CREATE INDEX idx_terminal_export_requests_session
ON terminal_export_requests(session_id, requested_at_ms DESC);
```

Rules:

- export defaults to redacted/inert data;
- raw output export requires explicit policy/approval;
- export manifest must describe redaction profile, event/byte ranges, gaps and tombstones;
- export artifacts are never named from command text/session title.

### Table: `terminal_support_bundles`

Purpose: safe diagnostic package creation without raw transcript by default.

```sql
CREATE TABLE terminal_support_bundles (
    id TEXT PRIMARY KEY,
    scope_json TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    requested_at_ms BIGINT NOT NULL,
    completed_at_ms BIGINT,
    status TEXT NOT NULL,
    include_raw_output INTEGER NOT NULL DEFAULT 0,
    include_command_text INTEGER NOT NULL DEFAULT 0,
    redaction_profile_id TEXT,
    integrity_check_id TEXT REFERENCES terminal_integrity_checks(id) ON DELETE SET NULL,
    restore_drill_ids_json TEXT,
    manifest_json TEXT,
    artifact_ref TEXT,
    checksum_algorithm TEXT,
    checksum TEXT,
    error_json TEXT
);

CREATE INDEX idx_terminal_support_bundles_time
ON terminal_support_bundles(requested_at_ms DESC);
```

Rules:

- default bundle includes versions, metrics, schema ids, invariant results, gap/tombstone summaries and redacted errors;
- raw output and raw command text require explicit approval;
- support bundle should not include full DB copy by default;
- all bundle artifacts use generated storage names.

## Diesel schema and models example

Example `schema.rs` fragment:

```rust
diesel::table! {
    terminal_sessions (id) {
        id -> Text,
        route_json -> Text,
        backend_kind -> Text,
        title -> Nullable<Text>,
        launch_json -> Nullable<Text>,
        created_at_ms -> BigInt,
        updated_at_ms -> BigInt,
        closed_at_ms -> Nullable<BigInt>,
        restore_generation -> BigInt,
        persistence_version -> Integer,
        private_mode -> Integer,
        retention_policy_id -> Text,
    }
}

diesel::table! {
    terminal_commit_log (id) {
        id -> Text,
        session_id -> Text,
        commit_seq -> BigInt,
        commit_kind -> Text,
        writer_generation -> Text,
        pane_high_water_json -> Text,
        operation_id -> Nullable<Text>,
        idempotency_key_id -> Nullable<Text>,
        created_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_stream_cursors (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        stream_id -> Text,
        next_event_seq -> BigInt,
        next_byte_seq -> BigInt,
        updated_at_ms -> BigInt,
    }
}

diesel::table! {
    terminal_stream_segments (id) {
        id -> Text,
        session_id -> Text,
        pane_id -> Text,
        commit_id -> Text,
        commit_seq -> BigInt,
        stream_id -> Text,
        event_seq_low -> BigInt,
        event_seq_high -> BigInt,
        byte_low -> BigInt,
        byte_high -> BigInt,
        time_low_ms -> BigInt,
        time_high_ms -> BigInt,
        capture_layer -> Text,
        capture_semantics -> Text,
        payload_kind -> Text,
        codec -> Text,
        codec_level -> Nullable<Integer>,
        encryption_state -> Text,
        key_ref -> Nullable<Text>,
        payload -> Nullable<Binary>,
        artifact_ref -> Nullable<Text>,
        stored_byte_len -> Integer,
        uncompressed_byte_len -> Integer,
        checksum_algorithm -> Text,
        checksum -> Text,
        redaction_state -> Text,
        created_at_ms -> BigInt,
    }
}
```

Example model:

```rust
use diesel::{Identifiable, Insertable, Queryable, Selectable};

#[derive(Debug, Clone, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::db::schema::terminal_sessions)]
pub struct TerminalSessionRow {
    pub id: String,
    pub route_json: String,
    pub backend_kind: String,
    pub title: Option<String>,
    pub launch_json: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub closed_at_ms: Option<i64>,
    pub restore_generation: i64,
    pub persistence_version: i32,
    pub private_mode: i32,
    pub retention_policy_id: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::db::schema::terminal_sessions)]
pub struct NewTerminalSessionRow<'a> {
    pub id: &'a str,
    pub route_json: &'a str,
    pub backend_kind: &'a str,
    pub title: Option<&'a str>,
    pub launch_json: Option<&'a str>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub persistence_version: i32,
    pub retention_policy_id: &'a str,
}
```

Repository example:

```rust
use diesel::prelude::*;

pub fn insert_session(
    conn: &mut SqliteConnection,
    row: NewTerminalSessionRow<'_>,
) -> Result<(), PersistenceError> {
    diesel::insert_into(crate::db::schema::terminal_sessions::table)
        .values(row)
        .execute(conn)?;
    Ok(())
}
```

## Capture event contract

Backends should not write DB directly. They emit capture events.

```rust
#[derive(Debug, Clone)]
pub enum PersistenceCaptureEvent {
    SessionCreated {
        session_id: SessionId,
        route: SessionRoute,
        title: Option<String>,
        launch: Option<ShellLaunchSpec>,
        occurred_at_ms: i64,
    },
    PaneCreated {
        session_id: SessionId,
        pane_id: PaneId,
        tab_id: Option<TabId>,
        rows: u16,
        cols: u16,
        occurred_at_ms: i64,
    },
    OutputBytes {
        session_id: SessionId,
        pane_id: PaneId,
        stream_id: String,
        bytes: bytes::Bytes,
        capture_layer: CaptureLayer,
        occurred_at_ms: i64,
    },
    InputSubmitted {
        session_id: SessionId,
        pane_id: PaneId,
        input: String,
        source: InputSource,
        idempotency_key: Option<String>,
        occurred_at_ms: i64,
    },
    Resize {
        session_id: SessionId,
        pane_id: PaneId,
        rows: u16,
        cols: u16,
        occurred_at_ms: i64,
    },
    ShellMarker {
        session_id: SessionId,
        pane_id: PaneId,
        marker: ShellMarker,
        trust_level: TrustLevel,
        occurred_at_ms: i64,
    },
    ScreenSnapshot {
        session_id: SessionId,
        pane_id: PaneId,
        snapshot: ScreenSnapshot,
        occurred_at_ms: i64,
    },
}
```

Best practice:

- all event variants are typed;
- raw bytes stay bytes;
- JSON is only payload envelope where needed;
- every event carries session/pane IDs and time;
- writer assigns canonical sequence numbers, not backend/client.
- every backend event should be wrapped in an envelope when possible:

```rust
#[derive(Debug, Clone)]
pub struct CaptureEventEnvelope {
    pub source_kind: CaptureSourceKind,
    pub source_event_id: Option<String>,
    pub source_payload_hash: String,
    pub backend_sequence_hint: Option<String>,
    pub received_at_ms: i64,
    pub event: PersistenceCaptureEvent,
}
```

Envelope rules:

- `source_event_id` must be stable across retry if the backend can provide one;
- if the backend has no stable ID, derive `source_payload_hash` for diagnostics but do not pretend idempotency is perfect;
- `backend_sequence_hint` is evidence/debug only, not the canonical sequence allocator;
- `source_payload_hash` for secret-bearing data must be local/keyed where the value might be dictionary-attackable.

## Capture semantics and fidelity policy

Not every backend gives the same kind of history.

Capture semantics:

```text
raw_vt_stream:
  bytes emitted by native PTY/ConPTY or a raw mux output channel
  replayable through terminal emulator

rendered_ansi_stream:
  already-rendered lines with ANSI styling preserved
  useful for visual restore/search, not equivalent to raw terminal bytes

rendered_plaintext_snapshot:
  already-rendered text without ANSI/control sequences
  lowest output fidelity, never used for raw replay

mux_structured_surface:
  JSON/structured viewport/scrollback from zellij/tmux-like backend
  good layout/visual evidence, command boundaries still need trust source

imported_text:
  user/imported transcript
  visual/search only unless proven otherwise
```

Rules:

- `raw_vt_stream` can rebuild screen state through the terminal parser;
- rendered surfaces can hydrate visible history but must carry lower restore guarantee;
- command extraction from rendered surfaces is heuristic and disabled for rerun by default;
- periodic reconciliation snapshots should compare streamed capture against backend snapshot APIs when available;
- every restore guarantee must say what was restored: raw replay, rendered scrollback, visual snapshot, live process attach or command history.

Fidelity levels:

```text
fidelity_raw_replayable:
  capture_semantics = raw_vt_stream
  byte ranges known where applicable
  parser version known
  restore drill passed

fidelity_rendered_ansi:
  capture_semantics = rendered_ansi_stream or mux_structured_surface
  visual history can be restored
  raw replay and exact command boundaries are not promised

fidelity_snapshot_only:
  screen/topology snapshot exists
  no complete event stream exists
  legacy/imported rows usually land here

fidelity_degraded:
  known gaps, stale capability report, checksum failure, failed drill or unsupported backend route
```

Rules:

- `RichHistory` requires `fidelity_raw_replayable`;
- `BasicHistory` may use `fidelity_rendered_ansi` if gaps and trust levels are visible;
- `VisualRestoreOnly` is the default for legacy snapshots/imported text;
- `HistoryDegraded` overrides optimistic labels when any required evidence is missing.

## Journal writer design

The writer owns:

- per-pane in-memory event/byte sequence cache backed by `terminal_stream_cursors`;
- session-level commit cache backed by `terminal_session_cursors`;
- output batching;
- transaction boundaries;
- segment checksum;
- outbox emission;
- degraded state when DB is slow/unavailable.

Flush policy:

```text
flush segment when:
  bytes >= 32 KiB
  OR time since first byte >= 100 ms
  OR command boundary marker arrives
  OR snapshot/save/close requires barrier
```

Initial values:

- max segment payload: `32-128 KiB`;
- channel capacity: bounded, start `1024 events`;
- overflow policy: degrade and emit health warning, never unbounded memory.

Sequence allocation:

```text
on writer startup:
  no global sequence preload is required

on first event or flush for session:
  BEGIN IMMEDIATE
  load terminal_session_cursors row
  reserve commit_seq
  load terminal_stream_cursors row
  if missing:
    next_event_seq = max(existing journal event_seq for pane/stream) + 1
    next_byte_seq = max(existing segment byte_high for pane/stream)
    insert cursor
  reserve event_seq range for journal events
  reserve half-open byte range for output payload when payload exists
  insert terminal_commit_log row
  insert capture receipt if source id exists
  insert segment/events/gap rows linked to commit_id
  update cursor.next_event_seq = reserved_event_high + 1
  update cursor.next_byte_seq = reserved_byte_high + 1
  update session_cursor.next_commit_seq = commit_seq + 1
  update pane.last_event_seq = max(pane.last_event_seq, reserved_event_high)
  COMMIT
```

Why:

- restart cannot reuse event or byte sequence numbers;
- multi-pane restore can anchor to a single session commit sequence;
- `terminal_panes.last_event_seq` may be stale if a crash happens before transaction commit, cursor remains authoritative;
- sequence allocation and payload commit are atomic from the reader point of view.

Backpressure policy:

```text
PTY reader thread/task must never wait indefinitely for DB.
writer.try_record(event):
  if queue accepts:
    return ok
  if queue full:
    mark pane/session history_degraded
    enqueue lightweight gap marker if possible
    persist terminal_history_gaps row when writer recovers
    drop durable capture for the overflowing payload
    keep live terminal rendering running
  if executor closed:
    mark persistence_unavailable
    keep live terminal running
```

Why:

- blocking PTY output can deadlock or freeze the terminal;
- unbounded queues turn a DB stall into memory growth;
- a visible `history_gap` is more honest than pretending all output was persisted.

Minimum degraded events:

```text
history_writer_queue_full
history_writer_closed
history_segment_dropped
history_gap_started
history_gap_ended
```

Minimum durable gap row fields:

```text
session_id
pane_id
stream_id
gap_kind
event_seq_low/event_seq_high if known
byte_low/byte_high if known
estimated_dropped_bytes
estimated_dropped_events
reason
writer_generation
```

Pseudo-code:

```rust
pub struct TerminalJournalWriter {
    executor: PersistenceExecutor,
    pending: HashMap<PaneId, SegmentBuffer>,
    event_seq: HashMap<PaneId, EventSeq>,
    byte_seq: HashMap<PaneId, ByteSeq>,
}

impl TerminalJournalWriter {
    pub async fn ingest(&mut self, event: PersistenceCaptureEvent) -> Result<(), PersistenceError> {
        match event {
            PersistenceCaptureEvent::OutputBytes { pane_id, bytes, .. } => {
                self.append_output(pane_id, bytes)?;
                if self.should_flush(pane_id) {
                    self.flush_pane(pane_id).await?;
                }
            }
            PersistenceCaptureEvent::ShellMarker { .. } => {
                self.flush_related_pane(&event).await?;
                self.executor.execute(AppendJournalEvent::from(event)).await?;
            }
            _ => {
                self.executor.execute(AppendJournalEvent::from(event)).await?;
            }
        }
        Ok(())
    }
}
```

Transaction for output segment:

```rust
conn.immediate_transaction(|conn| {
    let commit = reserve_session_commit(conn, session_id, CommitKind::OutputFlush)?;
    let event_range = reserve_stream_event_range(conn, pane_id, stream_id, segment.event_count)?;
    let byte_range = reserve_stream_byte_range(conn, pane_id, stream_id, segment.byte_len)?;
    insert_capture_receipt_if_any(conn, &capture_receipt, commit.id)?;
    insert_stream_segment(conn, &segment.with_commit(&commit, event_range, byte_range))?;
    insert_journal_event(conn, &event.with_commit(&commit, event_range, byte_range))?;
    insert_history_gap_rows_if_any(conn, &gap_rows, commit.id)?;
    insert_outbox_message(conn, projection_job)?;
    update_stream_cursor(conn, pane_id, stream_id, event_range.next_after, byte_range.next_after)?;
    update_session_commit_cursor(conn, session_id, commit.commit_seq + 1)?;
    update_pane_last_event_seq(conn, pane_id, event_range.high)?;
    Ok(())
})
```

Invariant:

```text
terminal_stream_cursors.next_event_seq > max(journal event_seq for pane/stream)
terminal_stream_cursors.next_byte_seq > max(stream segment byte_high for pane/stream)
terminal_session_cursors.next_commit_seq > max(commit_seq for session)
every canonical journal/segment row references an existing commit
pane.last_event_seq >= max(terminal_journal_events.event_seq for pane)
every segment event_seq range has at least one journal event
no segment event_seq range overlaps another segment event_seq range for same pane/stream
no segment byte range overlaps another segment byte range for same pane/stream when byte ranges are known
every known dropped payload range has terminal_history_gaps row
topology snapshot high_water_commit_seq references existing commit
```

## Terminal emulator and TUI policy

Terminal history must handle more than line-oriented command output.

Buffer modes:

```text
normal:
  shell prompts, command output and scrollback

alternate:
  full-screen apps such as vim, less, top, htop, interactive TUIs

mux_surface:
  zellij/tmux visible UI surface when no better structured capture exists
```

Canonical rule:

- raw stream segments remain the source of truth;
- derived screen model stores normal-buffer scrollback and alternate-screen state separately;
- snapshots must include `buffer_kind`, dimensions, parser version and high-water `event_seq`;
- journal events must record buffer enter/leave events when parser detects them;
- command blocks can reference alternate-screen ranges, but search/export should treat them differently from plain output.

TUI frame policy:

```text
normal command output:
  preserve stream segments and line-oriented projections

alternate-screen bursts:
  preserve raw stream
  coalesce derived frames for UI/search/export
  write final snapshot on leave/close/save

mux surface fallback:
  mark lower fidelity
  avoid claiming high-confidence command history
```

Implementation guidance:

- reuse `alacritty_terminal` parser/emulator already in the workspace for derived screen state where possible;
- never try to parse alternate-screen apps as shell command lines by visible text only;
- restore can show latest alternate-screen visual state, but full frame-by-frame replay may be behind a fidelity flag;
- Playwright tests should include `cmd`, PowerShell, long output, `less`/alternate-screen equivalent where available, and resize during alternate screen.

## Command block state machine

Command blocks are first-class rows, not UI decoration.

States:

```text
pending_prompt
editing
submitted
running
finished
abandoned
unknown
```

Trusted sources:

1. `trusted_ui_submit`
2. `direct_pane_launch`
3. `osc633_with_nonce`
4. `osc133_with_known_shell_hook`
5. `shell_reported_no_nonce`
6. `terminal_observed`
7. `rendered_mux_surface`
8. `heuristic`

Trust rules:

```text
trusted_ui_submit:
  may create rerunnable command block

direct_pane_launch:
  high trust for command text and exit status when backend launches command as pane process

osc633_with_nonce / osc133_with_known_shell_hook:
  high/medium trust depending on nonce and installation path

shell_reported_no_nonce:
  usable for grouping output, rerun requires confirmation

terminal_observed / rendered_mux_surface / heuristic:
  may annotate history, cannot create auto-rerunnable command history by default
```

Rules:

- `copy command`, `rerun`, AI attachment and analytics must inspect `trust_level`;
- untrusted command text requires confirmation before rerun;
- background output must not be force-attributed to foreground command;
- raw input keystrokes are off by default.

Command history vs transcript:

```text
terminal transcript:
  canonical output/input evidence
  append-only journal + stream segments
  may contain secrets and control sequences

command block:
  lifecycle of one command
  ranges into transcript where known
  trust level and source metadata

command history entry:
  product read model for command dock/autocomplete/rerun
  scoped to session/pane/global policy
  may store redacted/display text without raw command_text

browser command cache:
  optional UI acceleration only
  never authoritative after restart
```

Command history write policy:

- create `terminal_command_history_entries` from high/medium trust command blocks only;
- low-trust rendered/heuristic rows may be shown as historical annotations but default to `rerun_policy = disabled` or `confirm`;
- private/sensitive commands may have `command_text = NULL` while keeping redacted/display text and audit metadata;
- dedupe uses local/keyed hash, not exportable stable fingerprint;
- each history entry records `scope_kind`, `session_id`, `pane_id`, `source`, `trust_level`, `sensitivity_class` and `rerun_policy`;
- deleting/pruning a command history read model must not delete transcript/journal evidence.

Command marker handler example:

```rust
pub fn apply_shell_marker(
    state: &mut CommandBlockState,
    marker: ShellMarker,
    event_seq: EventSeq,
    now_ms: i64,
) -> Vec<CommandBlockMutation> {
    match marker {
        ShellMarker::PromptStart => vec![CommandBlockMutation::StartPrompt { event_seq, now_ms }],
        ShellMarker::CommandLine { text, source } => {
            vec![CommandBlockMutation::SetCommandText { text, source }]
        }
        ShellMarker::PreExec => vec![CommandBlockMutation::MarkRunning { event_seq, now_ms }],
        ShellMarker::Finished { exit_code } => {
            vec![CommandBlockMutation::MarkFinished { event_seq, now_ms, exit_code }]
        }
        ShellMarker::Cwd { cwd, source } => {
            vec![CommandBlockMutation::SetCwd { cwd, source }]
        }
    }
}
```

## Snapshot and restore design

Snapshots are hydration aids.

Snapshot should store:

- pane ID;
- rows/cols;
- parser/projection version;
- base/high-water `event_seq` and optional high-water `byte_seq`;
- `commit_seq` and pane high-water vector for session-level restore points;
- screen surface JSON;
- checksum;
- created_at;
- source: native emulator, zellij pane, tmux capture, imported replay.

Restore flow:

```text
load session row
load latest valid topology snapshot anchored to commit_seq
create new native process/session if native restore
rebuild layout
load latest valid snapshot per pane at or before topology pane_high_water_json[pane]
hydrate visible history from snapshot
replay journal from pane snapshot high_water_event_seq + 1 up to topology pane_high_water_json[pane]
mark live boundary
show restore semantics badge
```

Important:

- restored historical output must be visually separated from new live process output;
- historical replay must suppress side effects like OSC52 clipboard;
- native restore must not auto-run old commands;
- zellij/tmux live attach can preserve process only if live mux session exists.

Atomic save rule:

```text
save_session_snapshot:
  flush all panes
  reserve one commit_seq
  write topology snapshot with pane_high_water_json
  write pane screen snapshots linked to same or previous commit
  emit restore_drill outbox job
```

Never choose latest screen snapshot independently per pane without checking the topology high-water vector.

Restore response semantics must be derived from evidence. Do not set `replays_saved_screen_buffers = true` just because a session row exists:

```rust
fn restore_semantics_from_evidence(evidence: &RestoreEvidence) -> SavedSessionRestoreSemantics {
    SavedSessionRestoreSemantics {
        restores_topology: evidence.verified_topology_snapshot,
        restores_focus_state: evidence.verified_topology_snapshot,
        restores_tab_titles: evidence.verified_topology_snapshot,
        uses_saved_launch_spec: evidence.has_launch_spec,
        replays_saved_screen_buffers: evidence.user_visible_history_hydrated
            && !evidence.only_legacy_snapshot,
        preserves_process_state: evidence.live_mux_attach_preserved_process,
    }
}
```

Native Windows restore after host restart normally has:

```text
preserves_process_state = false
replays_saved_screen_buffers = true only after v2 hydrate/replay actually ran
restore_guarantee_level = RichHistory/BasicHistory/VisualRestoreOnly/HistoryDegraded from evidence
```

Restore UI/read-model contract:

```text
phase 1:
  load session/topology metadata
  load nearest verified snapshot
  show historical region quickly
  show guarantee badge and gap markers

phase 2:
  page journal/history after snapshot high-water
  close SQLite cursor before sending page to browser
  append pages to historical region
  keep live process output in a separate live region

phase 3:
  if native restore starts a new process, mark live boundary
  never visually merge old prompt/output with the new process without a boundary
```

Acceptance:

- a user can see what was entered and what was output before restart;
- a user can distinguish restored history from new live output;
- a huge session restores first usable view quickly and progressively hydrates older/newer pages;
- a known gap is visible in the restored transcript and in the restore semantics payload.

## Replay sandbox policy

Historical replay is not the same as live terminal execution.

Replay modes:

```text
live_terminal:
  terminal output can affect live terminal state according to normal terminal rules

historical_restore:
  output is parsed into screen/history model only
  no host side effects are executed

restore_drill:
  output is parsed into an inert parser only
  no UI or host side effects are executed
```

Side-effect handling:

```text
OSC52 clipboard:
  store as data, suppress during historical replay

window/tab title changes:
  store title events, apply only to restored historical metadata when requested

OSC7 cwd and OSC133/633 shell markers:
  parse as metadata/events, never execute commands

hyperlinks:
  render as historical hyperlinks only, no automatic open

bell:
  store event, do not play during restore by default

prompt-injection-like text:
  always data, never instruction
```

Rules:

- restore renderer must call a side-effect-filtered parser path;
- test fixtures must include OSC52, title changes, OSC7 and hyperlink sequences;
- exported history should mark control-sequence-derived metadata separately from visible text.

## Restore drill design

Restore drill is the reliability proof that persisted data is actually usable.

Drill modes:

```text
lightweight_startup_sample
manual_user_requested
pre_release_full_scan
post_migration_full_scan
failure_reproduction_seeded
```

Minimum algorithm:

```text
load session
load latest topology snapshot
for each pane:
  load latest valid screen snapshot
  verify checksum
  load stream segments around snapshot high_water_event_seq
  verify non-overlap and checksum
  replay journal into inert terminal parser
  verify parser high_water_event_seq == expected
  verify known gaps are represented
persist terminal_restore_drills row
```

Important implementation rule:

- drill replay must be inert and side-effect-free;
- OSC52, hyperlinks, shell integration control sequences and prompt injection-like text are data only;
- drill result should include exact failure category, first bad `event_seq` and affected pane;
- failed drill must not delete data automatically.

## Outbox jobs

Outbox jobs are required from the start, even if only one worker exists.

Initial job kinds:

```text
projection_rebuild
search_index_update
snapshot_write
restore_drill
redaction_scan
compression_rewrite
```

SQLite claim discipline:

```rust
pub fn claim_next_outbox_message_sqlite(
    conn: &mut SqliteConnection,
    worker_id: &str,
    now_ms: i64,
) -> Result<Option<OutboxMessageRow>, PersistenceError> {
    conn.immediate_transaction(|conn| {
        let changed = claim_pending_message_sqlite(conn, worker_id, now_ms)?;
        if changed == 0 {
            return Ok(None);
        }

        load_claimed_message_for_worker(conn, worker_id, now_ms).map(Some)
    })
}
```

SQL shape for `claim_pending_message_sqlite`:

```sql
UPDATE terminal_outbox_messages
SET
    status = 'claimed',
    claimed_by = ?2,
    claim_token = ?3,
    claimed_until_ms = ?4,
    updated_at_ms = ?5
WHERE id = (
    SELECT id
    FROM terminal_outbox_messages
    WHERE status = 'pending'
      AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms <= ?1)
    ORDER BY created_at_ms ASC
    LIMIT 1
)
AND status = 'pending';
```

Rules:

- v1 can run one worker to avoid premature lock complexity;
- if more workers are added, use `immediate_transaction` and conditional update;
- `returning_clauses_for_sqlite_3_35` can simplify claim/load if SQLite runtime supports it;
- no side effect should happen before the claim transaction commits.
- claimed rows must heartbeat or complete before `claimed_until_ms`;
- stale claimed rows are reset to pending by maintenance with attempt_count/backoff update;
- worker completion must match `claim_token`, not just message id.
- worker actions are idempotent;
- crash after side effect must be safe;
- poison jobs move to `quarantined`;
- UI/diagnostics show outbox lag.

Compression worker rules:

- compression is derived rewrite of segment storage, not a semantic event;
- first implementation keeps `codec = 'raw'`;
- later worker may rewrite cold/large segments to `codec = 'zstd'`;
- rewrite must preserve `event_seq_low`, `event_seq_high`, `byte_low`, `byte_high`, `commit_seq`, payload checksum policy and restore drill compatibility;
- if decompression fails, mark segment corrupted/degraded and do not delete original until replacement passed verification.

## Redaction and privacy baseline

Default policy:

- raw output capture: on;
- raw input keystrokes: off;
- command text capture: on through UI submit or trusted shell integration;
- redaction scan: enabled for export/AI/search snippets;
- private mode: disables durable output by policy or stores only minimal metadata depending profile.

First implementation:

- define `terminal_redaction_rules` later;
- implement basic scanner as pure Rust service first;
- do not block journal writer on expensive redaction;
- redaction runs as outbox job and writes derived projection state.

Never:

- never send raw transcript to AI by default;
- never export raw transcript without explicit policy;
- never store private-mode data in browser localStorage.

## Privacy data classification

Terminal history is sensitive even when it looks like normal text. Classification is required before export/search/AI/support.

Data classes:

```text
class_public_diagnostic:
  app version, schema version, feature gate state, aggregate metrics
  default support bundle: allowed

class_local_metadata:
  session ids, pane ids, backend capability reports, timing, restore badges
  default support bundle: allowed if no raw paths

class_user_context:
  cwd, shell kind, command display text, tab titles, redacted snippets
  default support bundle: redacted only

class_sensitive_content:
  raw terminal output, raw command text, raw cwd/path, environment-like strings
  default export/support/AI: excluded

class_secret_material:
  keys, tokens, passwords, DPAPI/keyring payloads, encryption keys
  never exported, never logged
```

Default handling matrix:

```text
search:
  redacted snippets only

command dock:
  display/redacted text by default, raw command only when policy allows

support bundle:
  diagnostics + redacted summaries, no raw transcript

AI context:
  data-only structured context, redacted, explicit approval for actions

raw export:
  explicit request + approval + manifest + health check
```

Rules:

- every new table/column should be assigned a data class in code comments or schema metadata;
- raw command/output fields should never be added to support bundle by "just serialize row";
- hashes of commands/paths can still leak by dictionary attack, so treat stable hashes as sensitive unless keyed/local;
- private mode overrides default retention and search/indexing behavior.

## Search v1

Do not build large search first. Search is derived data, not canonical persistence.

Iteration goal:

- command block list;
- search command text and small redacted output snippets;
- chunk catalog schema ready;
- no global cold search yet.

V1 relational model:

```sql
CREATE TABLE terminal_search_documents (
    rowid INTEGER PRIMARY KEY AUTOINCREMENT,
    document_id TEXT NOT NULL UNIQUE,
    session_id TEXT NOT NULL REFERENCES terminal_sessions(id) ON DELETE CASCADE,
    pane_id TEXT REFERENCES terminal_panes(id) ON DELETE CASCADE,
    command_block_id TEXT REFERENCES terminal_command_blocks(id) ON DELETE SET NULL,
    document_kind TEXT NOT NULL,
    event_seq_low BIGINT,
    event_seq_high BIGINT,
    byte_low BIGINT,
    byte_high BIGINT,
    redaction_profile_id TEXT,
    redaction_state TEXT NOT NULL,
    source_hash_algorithm TEXT NOT NULL DEFAULT 'blake3',
    source_hash TEXT NOT NULL,
    text_preview TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    updated_at_ms BIGINT NOT NULL,
    CHECK((event_seq_low IS NULL AND event_seq_high IS NULL) OR (event_seq_low IS NOT NULL AND event_seq_high IS NOT NULL AND event_seq_low <= event_seq_high)),
    CHECK((byte_low IS NULL AND byte_high IS NULL) OR (byte_low IS NOT NULL AND byte_high IS NOT NULL AND byte_low < byte_high))
);

CREATE INDEX idx_terminal_search_documents_session
ON terminal_search_documents(session_id, updated_at_ms DESC);
```

Rules:

- `text_preview` is redacted derived text, never raw terminal stream by default;
- search docs are rebuilt by outbox workers;
- canonical restore must not depend on search docs;
- deletion/private mode must delete or invalidate search docs first.

Later:

- `terminal_chunk_catalog`;
- SQLite FTS5 hot index over `terminal_search_documents`, not raw stream;
- Tantivy warm shards;
- cold search job with budget.

FTS5 guidance for later:

- prefer contentless/contentless-delete style if runtime SQLite supports required version;
- otherwise use external-content FTS with explicit rebuild jobs and tests;
- do not rely on triggers over canonical stream writes in v1, because redaction and privacy policy must run before indexing;
- FTS index is allowed to be dropped and rebuilt without data loss.

## AI context v1

AI integration must use structured context package, not raw transcript paste.

Minimum data model later:

```text
terminal_ai_context_packages
terminal_ai_context_items
terminal_prompt_injection_findings
terminal_ai_action_approvals
```

First implementation rule:

- terminal output is `data_only`;
- AI-suggested `send_input`, `rerun`, `export`, `share`, `delete` require approval;
- context preview shows command blocks, event ranges, byte ranges where available and redaction state.

## zellij/tmux fidelity policy

Mux backends are not just "another shell". They need explicit adapters.

Guarantee matrix:

```text
native_conpty:
  process preservation after app restart: false
  visible history restore: true after v2
  command text reliability: high for UI submit, medium/low for raw typed until shell integration

zellij_windows_live_attach:
  availability: official Windows binary exists, but every install/session still needs runtime probe
  process preservation: true if the zellij session is still live and attach succeeds
  visible history restore: zellij rendered surface + our journal where captured
  command text reliability: backend/shell-integration dependent
  preferred capture: list-panes/list-tabs JSON for topology, subscribe JSON for rendered updates, dump-screen --full for reconciliation

zellij_unix_live_attach:
  process preservation: true if the zellij session is still live and attach succeeds
  visible history restore: mux-provided + our journal where captured
  command text reliability: backend/shell-integration dependent
  preferred capture: list-panes/list-tabs JSON for topology, subscribe JSON for rendered updates, dump-screen --full for reconciliation

tmux_wsl_or_msys2_live_attach:
  process preservation: true if tmux server/session is still live and attach succeeds
  visible history restore: tmux capture-pane + our journal where captured
  command text reliability: backend/shell-integration dependent
  preferred capture: control mode or pipe-pane for streaming, capture-pane for scrollback reconciliation
  Windows note: not native Windows baseline; WSL/MSYS2 route must be explicit in backend route metadata

mux_resurrected_or_dead:
  process preservation: false unless mux itself restored it
  visible history restore: our persisted journal/snapshots only
```

Adapter design:

```rust
pub enum CaptureLayer {
    NativeConpty,
    ZellijSubscribe,
    ZellijDumpScreen,
    ZellijListPanesJson,
    ZellijListTabsJson,
    TmuxControlMode,
    TmuxPipePane,
    TmuxCapturePane,
    OuterMuxPtyFallback,
}

pub struct BackendCapabilityReport {
    backend_kind: String,
    backend_version: Option<String>,
    can_preserve_process_when_live: bool,
    can_capture_scrollback: bool,
    can_emit_structured_pane_events: bool,
    command_boundary_confidence: TrustLevel,
    notes: Vec<String>,
}
```

Rules:

- do not parse the whole mux UI as one shell transcript unless marked `OuterMuxPtyFallback`;
- prefer structured zellij/tmux capabilities where available;
- zellij `subscribe --format json` and `dump-screen --full` are rendered surface capture, not raw VT stream by default;
- tmux `capture-pane` is scrollback snapshot/reconciliation; tmux control mode/pipe-pane can be a better streaming source when available;
- zellij on Windows is allowed as a first advanced mux target, but only after probe verifies CLI availability and JSON command behavior;
- tmux on Windows must declare route kind: `wsl`, `msys2`, `cygwin`, or `unsupported_native`; do not hide it behind generic `windows_tmux`;
- record backend version and capability probe result per session;
- store capability report rows and expire them after backend upgrade;
- show restore guarantee per backend in UI;
- command blocks still require UI submit or shell integration for high trust;
- if mux live attach preserves process, the persistence layer still owns durable history.

Source-of-truth rules:

```text
zellij list-tabs/list-panes:
  topology/layout evidence
  not output history

zellij subscribe:
  rendered/update evidence
  good for live UI sync and reconciliation
  raw replay only if a future probe proves the payload is raw terminal stream

zellij dump-screen:
  rendered scrollback/screen evidence
  good for hydration/reconciliation
  not command source of truth

tmux control mode:
  can be structured control/event evidence
  still needs per-command trust source

tmux pipe-pane:
  can be better stream evidence when configured early enough
  missing early output creates explicit gap

tmux capture-pane:
  rendered scrollback snapshot
  reconciliation/hydration only
```

Probe requirements before enabling a mux adapter:

```text
binary path and version
route kind: native_windows / wsl / msys2 / cygwin / unix
can list sessions/panes/tabs
can subscribe to updates in expected format
can dump screen/scrollback in expected format
can attach to live session
capture_semantics for each available channel
whether event IDs or stable sequence hints exist
whether command boundary markers are available
```

Capability cache invalidation:

- expire report when backend binary path changes;
- expire report when backend version changes;
- expire report when mux config path/hash changes if known;
- expire report after failed attach/subscribe/dump probe;
- downgrade guarantee to `HistoryDegraded` when the report is stale and no fresh probe can run.

Reconciliation loop:

```text
stream updates from best available mux channel
periodically capture full rendered scrollback/surface
compare high-water marker / visible tail hash
if mismatch:
  write terminal_data_health_records row
  mark affected capture range lower fidelity
  show restore guarantee downgrade
```

## Windows-specific rules

For first version:

- DB inside OS app data directory from `directories`;
- do not place artifact store in arbitrary user folder yet;
- use SQLite BLOB segments first to avoid path hazards;
- test PowerShell and cmd separately;
- record shell kind and execution domain.

When external artifacts are introduced:

- generated storage keys only;
- no command text as filename;
- reject ADS and reserved names;
- handle reparse points explicitly;
- verify final path/file identity for critical writes.

## Migration strategy from v1 rusqlite store

Current `native_saved_sessions` should be treated as legacy read model.

Migration approach:

1. Move the current `SqliteSessionStore` code behind a `legacy::rusqlite_v1_store` module or adapter.
2. Add Diesel dependencies and embedded migrations without deleting old tables.
3. Add `legacy::migrate_v1` command.
4. On startup, detect old rows not migrated.
5. Create `terminal_sessions`, `terminal_panes`, `terminal_topology_snapshots`, `terminal_screen_snapshots` and `terminal_legacy_migration_records` from old rows.
6. Mark migrated sessions with `source = legacy_snapshot_only`.
7. Do not invent stream journal that never existed.
8. Restore legacy sessions as `visual_restore_only` unless journal exists.
9. Keep v1 route registry `session_routes` readable until v2 route records are proven equivalent.
10. Keep v1 `prune_saved_sessions` behavior scoped to legacy snapshot rows only.
11. Never route v2 canonical journal retention through v1 count-based pruning.

Migration table:

```sql
CREATE TABLE terminal_legacy_migration_records (
    id TEXT PRIMARY KEY,
    legacy_table TEXT NOT NULL,
    legacy_session_id TEXT NOT NULL,
    new_session_id TEXT NOT NULL,
    migrated_at_ms BIGINT NOT NULL,
    migration_state TEXT NOT NULL,
    notes TEXT
);
```

Rule:

- never pretend old snapshots contain full output history;
- UI must show degraded historical fidelity for migrated rows.
- migrated legacy rows can become searchable only through snapshot-derived/redacted projections, not raw transcript replay.
- restoring a migrated row must not flip `replays_saved_screen_buffers` to true unless v2 hydration evidence exists.

Startup migration safety:

```text
open SQLite connection
set PRAGMAs
check application_id:
  0 -> new/legacy candidate
  TPV2 -> current family
  anything else -> fail closed
run embedded Diesel migrations
ensure terminal_db_identity
record terminal_maintenance_runs row for migration/check
do not delete legacy rows automatically
```

Production rules:

- Diesel `down.sql` is for development/test only; shipped app migrations are forward-only.
- Major migrations should create a DB backup or export manifest before destructive changes.
- Legacy migration must be idempotent through `terminal_legacy_migration_records`.
- Migration must tolerate the existing `rusqlite_migration` index and the ad hoc `manifest_json` upgrade path.
- V2 must not call migration checks from every operation like current `open_connection()` does; hot path opens already-initialized executor connections.
- Dual-read tests must compare v1 saved session summaries against v2 summaries before switching list/read APIs.
- If migration partially succeeds, resume from audit records rather than starting over blindly.
- Old `native_saved_sessions` rows are not full history; they become `visual_restore_only`.
- Any v2 retention/delete endpoint must reject legacy pruning parameters such as "keep latest N" unless explicitly operating on legacy snapshots.

## Production MVP cutline

The plan is intentionally broad, but the first production milestone must stay narrow enough to ship and verify.

Production MVP includes:

```text
Diesel schema and migrations
single writer executor
native Windows ConPTY durable output capture
command blocks from trusted UI submit
DB-backed command history
screen/topology snapshots
restore visible history after app restart
restore drills
history gaps
storage pressure diagnostics
safe backup via VACUUM INTO
redacted support diagnostics
Playwright restart smoke
```

Production MVP excludes:

```text
SQLCipher/encrypted history
zstd compression rewrite
external artifact store
full zellij/tmux structured capture as authoritative history
AI action automation
raw keystroke logging
automatic command rerun after restore
```

Reliability gate before calling MVP complete:

- native Windows command/output survives app restart;
- long output does not freeze UI or grow memory unbounded;
- restore drill passes on new v2 sessions;
- synthetic writer failure produces visible gap/degraded state;
- default support/export/AI paths do not include raw transcript;
- backup/restore smoke passes on temp DB;
- feature gates can disable v2 authoritative reads and fall back to legacy/visual behavior where applicable.

Advanced tracks can run after MVP:

```text
track A - zellij Windows structured adapter
track B - tmux WSL/MSYS2 adapter
track C - encryption/key management
track D - compression/cold segment maintenance
track E - richer search/AI context
```

## Hard gates before implementation PRs

These gates prevent partial implementation from being mistaken for a completed persistence feature.

Before PR 1 merge:

- `cargo tree -i libsqlite3-sys` shows one SQLite native binding family;
- Diesel and `rusqlite` do not fight over bundled SQLite features;
- v2 migrations run from embedded migrations on a temp DB;
- legacy `SqliteSessionStore` tests still pass;
- feature gate defaults are documented.

Before v2 authoritative writes:

- writer lease exists and rejects a second active writer;
- DB identity/application_id checks fail closed on wrong SQLite file;
- `terminal_stream_cursors` and `terminal_session_cursors` pass restart invariants;
- native capture events are persisted without blocking PTY reader indefinitely;
- `raw_output_stream` remains false/degraded until this proof exists.

Before v2 authoritative reads/UI badge:

- restore semantics v2 are exposed next to legacy bools;
- evidence refs point to real rows or explicit gap/degraded rows;
- latest restore drill for the session/backend/parser version passed or produced a visible degraded state;
- browser restart smoke proves history comes from DB, not local browser storage.

Before mux support is called Windows-supported:

- zellij/tmux adapter has fresh capability report;
- route kind is explicit: native Windows, WSL, MSYS2, Cygwin or unsupported;
- rendered sources are labeled rendered and cannot produce `RichHistory`;
- attach/process preservation is shown separately from persisted history.

Before retention/delete is enabled:

- canonical parent deletes fail closed;
- delete request/tombstone flow is resumable;
- support/export defaults exclude raw transcript and raw commands;
- no count-based v1 pruning code path touches v2 canonical tables.

## Iteration plan

### Iteration 0 - Architecture and dependency foundation

Оценка: 🎯 10   🛡️ 9   🧠 5
Объем: `500-1200` строк.

Tasks:

- Add this plan to docs.
- Add Diesel dependencies after version verification.
- Create `migrations/` directory.
- Add `diesel.toml` and generated/checked `src/db/schema.rs` workflow.
- Add `db::connection`, `db::executor`, `db::migrations`.
- Add writer generation lease acquisition/release.
- Keep old `SqliteSessionStore` public API intact.
- Add connection PRAGMA initializer.
- Add `DurabilityProfile` with `ReliableHistory` as production writer default.
- Add smoke test that opens DB, runs migrations, checks `foreign_keys`.
- Add CI/helper check for Diesel table column budget.

Done when:

- `cargo test -p terminal-persistence` passes;
- Diesel migrations run on temp DB;
- `schema.rs` is generated/checked from migrations;
- column-budget check passes with current max table width <= 32;
- reliable profile sets `synchronous = FULL`;
- performance/test profile sets `synchronous = NORMAL`;
- only one active writer lease exists for the DB;
- old saved session tests still pass;
- no runtime API behavior changed.

### Iteration 1 - Core Diesel schema and dual store boundary

Оценка: 🎯 9   🛡️ 9   🧠 7
Объем: `1500-3000` строк.

Tasks:

- Add tables:
  - `terminal_db_identity`
  - `terminal_payload_schemas`
  - `terminal_projection_versions`
  - `terminal_retention_policies`
  - `terminal_maintenance_runs`
  - `terminal_integrity_checks`
  - `terminal_data_health_records`
  - `terminal_feature_gates`
  - `terminal_crypto_keys`
  - `terminal_crypto_key_events`
  - `terminal_backup_records`
  - `terminal_storage_pressure_events`
  - `terminal_sessions`
  - `terminal_panes`
  - `terminal_backend_capability_reports`
  - `terminal_writer_generations`
  - `terminal_clock_anchors`
  - `terminal_deletion_tombstones`
  - `terminal_delete_requests`
  - `terminal_session_cursors`
  - `terminal_commit_log`
  - `terminal_stream_cursors`
  - `terminal_topology_snapshots`
  - `terminal_stream_segments`
  - `terminal_journal_events`
  - `terminal_capture_receipts`
  - `terminal_command_blocks`
  - `terminal_command_history_entries`
  - `terminal_screen_snapshots`
  - `terminal_outbox_messages`
  - `terminal_idempotency_keys`
  - `terminal_clients`
  - `terminal_delivery_offsets`
  - `terminal_history_gaps`
  - `terminal_restore_drills`
  - `terminal_export_requests`
  - `terminal_support_bundles`
- Add Diesel schema/models.
- Add schema generation diff test so migrations and `schema.rs` cannot drift.
- Add typed payload wrappers and schema registry.
- Add repository functions for session/pane creation.
- Add FK-class tests:
  - direct parent delete fails for canonical history;
  - derived rows can be deleted/rebuilt;
  - audit rows survive with nullable source refs.
- Add range constraint tests for inclusive `event_seq` and half-open `byte` ranges.
- Add v2 store facade:

```rust
pub struct TerminalPersistenceV2 {
    executor: PersistenceExecutor,
}
```

- Add legacy migration detection, but do not auto-migrate yet.

Done when:

- insert/load session works through Diesel;
- insert/load pane works through Diesel;
- old rusqlite tests pass;
- schema has foreign keys and indexes.
- direct `DELETE FROM terminal_sessions` cannot silently remove canonical journal/segments/snapshots.
- JSON payload fixtures validate against registered schemas.
- integrity check rows can record quick_check/foreign_key_check results.
- stable status/kind columns have Rust enums and DB `CHECK` constraints where appropriate.
- capture semantics columns exist on canonical stream/journal rows.
- backend capability report rows can be inserted for native/mux probes.
- encryption metadata columns exist on payload-bearing tables with default `plaintext`.
- risky capabilities are represented in `terminal_feature_gates`.

### Iteration 2 - Capture events and journal writer skeleton

Оценка: 🎯 9   🛡️ 9   🧠 8
Объем: `1800-3600` строк.

Tasks:

- Add `PersistenceCaptureEvent`.
- Add bounded writer channel.
- Add session-level commit sequence assignment.
- Add per-pane sequence assignment.
- Add durable `terminal_stream_cursors`.
- Add segment batching.
- Add normal/alternate buffer event classification.
- Add capture semantics classification:
  - raw native ConPTY output as `raw_vt_stream`
  - rendered mux snapshots as lower-fidelity semantics
- Add output segment insert transaction.
- Add command-boundary and save/close writer barriers.
- Add basic health/degraded state if writer queue closes/fails.
- Add persisted gap rows for known dropped history.
- Native backend emits output bytes into writer.
- Native backend sets or reports `raw_output_stream` only after durable capture events are wired and tested.

Current replacement target:

- `terminal-backend-native::TranscriptBuffer` is ephemeral and limited to 256 KiB.
- Keep it only as UI helper if needed, but durable history must go through journal writer.
- Do not read durable history from `TranscriptBuffer`; it is allowed only as a short-lived rendering/diagnostic helper.

Done when:

- native output bytes are persisted as stream segments;
- every canonical write has `terminal_commit_log` row;
- sequence ordering invariant passes;
- stream cursor restart invariant passes;
- duplicate capture receipt test passes;
- command-boundary flush is tested;
- alternate-screen enter/leave events are persisted;
- restore drill can read persisted segments from DB;
- rendered mux captures are not replayed through raw terminal parser;
- writer does not block PTY reader indefinitely.

### Iteration 3 - Command blocks

Оценка: 🎯 9   🛡️ 8   🧠 8
Объем: `1800-3600` строк.

Tasks:

- Add command block state machine.
- Capture UI-submitted commands with idempotency key.
- Add command sensitivity classification baseline.
- Add shell marker event types for future OSC 133/633.
- For first native Windows baseline:
  - UI submit source gets high trust.
  - typed directly inside terminal gets lower confidence until shell integration is ready.
- Add command block list read API.
- Add command block ranges linked to stream `event_seq` and optional byte ranges.
- Add DB-backed command history entries for command dock/autocomplete.
- Add browser-cache-independent command history read path.
- Add local/keyed command hash strategy and no-export policy for hashes.

Done when:

- commands submitted from input composer persist per session/pane;
- command history survives browser/app restart without localStorage dependency;
- command history survives with browser storage disabled/cleared;
- sensitive commands default to confirm/disabled rerun policy;
- output ranges attach to running command where possible;
- unknown/background output stays unassigned or lower confidence;
- rerun API refuses untrusted commands without confirmation.

### Iteration 4 - Snapshot write and restore visible history

Оценка: 🎯 10   🛡️ 9   🧠 8
Объем: `2200-4200` строк.

Tasks:

- Write snapshots with `base_event_seq`, `high_water_event_seq` and optional `high_water_byte_seq`.
- Write topology snapshots with `pane_high_water_json`.
- Write snapshot `buffer_kind`.
- Write `checksum_algorithm` and checksum for snapshots/topology/segments.
- Save topology plus pane snapshots into v2 tables.
- Implement restore read path:
  - topology;
  - snapshot;
  - journal replay range;
  - paged historical reads after snapshot high-water;
  - SQLite cursor closed before async/browser page delivery.
- Add protocol v2 restore semantics fields with backward-compatible defaults.
- Add evidence refs in restore semantics payload.
- Change `replays_saved_screen_buffers` only when replay/hydration works and restore drill evidence exists.
- Mark restored historical content vs live boundary.

Done when:

- after app restart, user sees previous visible history;
- multi-pane restore uses one consistent topology high-water vector;
- alternate-screen restore is marked with explicit fidelity;
- latest `terminal_restore_drills` row is successful for a v2 session;
- `replays_saved_screen_buffers = true` only for v2 sessions with valid snapshots/journal;
- `restore_guarantee_level` and `history_replay_state` match the taxonomy/evidence rows;
- native process state is still honestly `preserves_process_state = false`;
- large history restore shows first snapshot quickly and progressively hydrates pages;
- historical region and new live process output cannot visually merge without boundary;
- Playwright/browser smoke confirms history visible after restart.

### Iteration 5 - Idempotency and reconnect delivery state

Оценка: 🎯 9   🛡️ 9   🧠 8
Объем: `1600-3200` строк.

Tasks:

- Add idempotency key around:
  - send input;
  - paste;
  - save session;
  - export later.
- Add delivery offsets for browser stream:
  - durable client row;
  - last sent `event_seq`;
  - last acked `event_seq`;
  - replay from `event_seq`.
- Add duplicate submit tests.
- Add reconnect replay tests.
- Add gap replay tests.

Done when:

- sending same command twice with same key does not duplicate action;
- browser reconnect can ask for missed `event_seq` range;
- unrecoverable gaps become visible degraded state.

### Iteration 6 - Outbox and derived workers

Оценка: 🎯 9   🛡️ 9   🧠 8
Объем: `1600-3400` строк.

Tasks:

- Add outbox claim/retry/quarantine.
- Add SQLite conditional claim using `BEGIN IMMEDIATE`.
- Add outbox claim token and lease expiry.
- Add projection rebuild job.
- Add snapshot write job.
- Add restore drill job.
- Add redaction scan placeholder job.
- Add maintenance job skeleton:
  - `PRAGMA optimize`
  - WAL checkpoint policy
  - retention scan in warn-only mode
  - stale writer/outbox lease recovery
- Add safe backup job skeleton:
  - `VACUUM INTO` first
  - optional SQLite Online Backup API adapter later
  - backup manifest
  - checksum
  - post-backup quick_check when budget allows
- Add storage pressure detector:
  - DB/WAL size probe
  - free-space probe
  - typed `SQLITE_FULL` handling
  - visible degraded state
- Add integrity check job skeleton:
  - `PRAGMA quick_check`
  - `PRAGMA foreign_key_check`
  - invariant SQL subset
  - projection drift scan
- Add compression rewrite placeholder job but keep raw codec default.
- Add outbox lag diagnostics.

Done when:

- journal transaction emits outbox rows;
- worker crash/retry is idempotent;
- poison jobs do not block all workers;
- maintenance never runs on capture hot path;
- integrity failures are persisted and visible, not auto-repaired silently;
- backup records are auditable and never rely on plain hot `.db` copy;
- synthetic `SQLITE_FULL` produces storage-pressure diagnostics and does not silently delete canonical history;
- compression job is disabled by default until restore drills cover zstd segments;
- outbox state appears in diagnostics.

### Iteration 7 - Redaction, private mode and export safety

Оценка: 🎯 9   🛡️ 9   🧠 8
Объем: `2200-4600` строк.

Tasks:

- Add redaction profile model.
- Add safe linear regex/exact matching scanner.
- Add redacted projection for command/output snippets.
- Add `terminal_search_documents` as redacted derived search source.
- Add private mode policy.
- Add delete request service and tombstone writer.
- Add export manifest as inert data.
- Add no-raw-export default.
- Add redacted support bundle request flow.
- Add privacy data-class mapping for v2 rows/fields.

Done when:

- export does not include raw secrets by default;
- private mode avoids durable raw output by policy;
- delete requests are auditable and resumable;
- support bundles omit raw output/commands by default;
- raw command/output columns are classified as sensitive and excluded from default support/AI/export;
- redaction profile changes invalidate derived snippets;
- search docs never index raw stream by default;
- prompt-injection-like output is still treated as data.

### Iteration 8 - Windows hardening

Оценка: 🎯 9   🛡️ 10   🧠 8
Объем: `1800-3600` строк.

Tasks:

- Add Windows command/cwd/shell metadata profiles.
- Test PowerShell and cmd separately.
- Add ConPTY resize/reprint tests.
- Add suspend/resume writer flush/recover test.
- Prepare path safety module for future external artifacts.
- Add restore/read paging tests so large history restore does not hold long SQLite read transactions.

Done when:

- Windows native sessions persist and restore reliably;
- PowerShell/cmd command capture fidelity is documented;
- no test assumes Linux shell semantics.
- restore page read closes DB cursor before streaming page data to browser.

### Iteration 9 - Zellij/tmux backend integration

Оценка: 🎯 8   🛡️ 8   🧠 8
Объем: `2200-5200` строк.

Tasks:

- Keep backend abstraction same.
- Add mux-specific capture layer.
- Do not parse outer mux UI as one shell transcript.
- Prefer zellij/tmux structured APIs where available.
- Treat zellij Windows as probed advanced adapter, not native baseline.
- Treat tmux Windows as WSL/MSYS2 route unless a separately verified native-compatible mux is added.
- Add zellij probes:
  - `list-panes --json`
  - `list-tabs --json`
  - `subscribe --format json`
  - `dump-screen --full`
- Add tmux probes:
  - control mode support
  - `capture-pane`
  - optional `pipe-pane`
- Store mux session route and attach semantics.
- Store backend capability report.
- Add capability report expiry on backend version/path/config/probe drift.
- Store per-channel capture semantics for subscribe/dump/control/pipe/capture channels.
- Show restore guarantee matrix per backend.

Done when:

- zellij live attach preserves process when mux session is live;
- history still persists through our journal where capture is reliable;
- UI distinguishes native restore vs mux attach/resurrection.
- outer mux UI fallback is visibly marked lower fidelity.
- capability report and capture semantics drive the displayed guarantee.
- stale mux capability report downgrades guarantee until a fresh probe passes.

### Iteration 10 - Reliability proof skeleton

Оценка: 🎯 9   🛡️ 10   🧠 9
Объем: `2600-5600` строк.

Tasks:

- Add invariant registry in test code first.
- Add failpoints around:
  - DB transaction;
  - segment flush;
  - outbox claim;
  - restore drill;
  - backup step;
  - disk-full/storage-pressure path.
  - checksum mismatch/quarantine path.
- Add seeded simulation for:
  - output chunks;
  - reconnect;
  - duplicate input;
  - crash before/after transaction;
  - `SQLITE_FULL` during segment flush;
  - long reader blocking checkpoint/WAL shrink.
- Add failure artifact capture for test failures.

Done when:

- deterministic seed can reproduce at least one injected failure;
- invariants detect corrupted/missing segment;
- restore drill runs in CI for temp DB.
- backup roundtrip can be verified from a temp DB;
- disk-full simulation leaves visible degraded state and durable diagnostics.
- corrupted derived snapshot falls back to raw replay when possible.
- corrupted canonical segment creates health record and visible restore gap.

### Iteration 11 - Encryption foundation

Оценка: 🎯 8   🛡️ 10   🧠 9
Объем: `2800-6200` строк.

Tasks:

- Add key hierarchy tables.
- Add OS key store capability profile.
- Add SQLCipher profile decision.
- Add Windows DPAPI/keyring capability probe.
- Add `zeroize`/`secrecy` wrappers for key material.
- Add encryption feature gate with fail-closed startup behavior.
- Add encrypted external artifact metadata for future external store.
- Add crypto erase records.
- Add encrypted export verifier.

Done when:

- encryption architecture has test vectors;
- key state is explicit;
- no plaintext fallback happens silently;
- secure deletion limitations are documented in UI/diagnostics.
- key refs are opaque and support bundles never include key material.

## Public APIs to add

Persistence crate facade:

```rust
pub struct TerminalPersistence {
    executor: PersistenceExecutor,
    journal: TerminalJournalHandle,
}

impl TerminalPersistence {
    pub async fn record(&self, event: PersistenceCaptureEvent) -> Result<(), PersistenceError>;
    pub async fn save_session_snapshot(&self, session_id: SessionId) -> Result<(), PersistenceError>;
    pub async fn restore_plan(&self, session_id: SessionId) -> Result<RestorePlan, PersistenceError>;
    pub async fn command_blocks(&self, pane_id: PaneId) -> Result<Vec<CommandBlock>, PersistenceError>;
    pub async fn run_restore_drill(&self, session_id: SessionId) -> Result<RestoreDrillReport, PersistenceError>;
    pub async fn create_safe_backup(&self, request: BackupRequest) -> Result<BackupRecord, PersistenceError>;
    pub async fn storage_health(&self) -> Result<StorageHealthReport, PersistenceError>;
    pub async fn backend_capability_report(&self, session_id: SessionId) -> Result<BackendCapabilityReport, PersistenceError>;
    pub async fn data_health(&self, session_id: SessionId) -> Result<Vec<DataHealthRecord>, PersistenceError>;
}
```

Runtime integration:

```rust
pub struct TerminalRuntime {
    sessions: SessionService,
    persistence: TerminalPersistence,
}
```

Backend integration:

```rust
pub trait PersistenceCaptureSink: Send + Sync {
    fn try_record(&self, event: PersistenceCaptureEvent) -> Result<(), CaptureBackpressure>;
}
```

Do not let backend call repositories directly.

## Error model

Add typed errors:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("database connection failed: {0}")]
    Connection(String),
    #[error("migration failed: {0}")]
    Migration(String),
    #[error("database query failed: {0}")]
    Query(#[from] diesel::result::Error),
    #[error("executor closed")]
    ExecutorClosed,
    #[error("writer queue full")]
    WriterQueueFull,
    #[error("storage pressure: {0}")]
    StoragePressure(String),
    #[error("database backup failed: {0}")]
    Backup(String),
    #[error("invalid persisted data: {0}")]
    InvalidData(String),
    #[error("restore degraded: {0}")]
    RestoreDegraded(String),
}
```

Rules:

- user-facing errors need reason code;
- internal DB errors should not expose raw secrets/path payloads;
- degraded state should be persisted when relevant.
- `SQLITE_FULL` must map to `StoragePressure`, not a generic query error.
- backup errors must preserve enough metadata for retry/support without exposing raw paths by default.

## Metrics and diagnostics

Minimum metrics:

```text
history_writer_queue_depth
history_writer_lag_ms
history_durability_profile
history_writer_generation
history_writer_lease_remaining_ms
history_db_file_bytes
history_wal_file_bytes
history_disk_free_bytes
history_storage_pressure_state
history_restore_visible_latency_ms
history_restore_page_count
history_restore_page_bytes
history_writer_commit_latency_ms
history_segment_flush_bytes
history_segments_written_total
history_segment_bytes_total
history_journal_last_event_seq
history_commit_last_commit_seq
history_cursor_lag_total
history_gaps_total
history_capture_receipt_conflicts_total
history_data_health_open_total
history_maintenance_last_result
history_backup_last_result
history_backup_last_age_ms
history_backend_capability_last_probe_result
history_feature_gate_state
history_encryption_capability_state
history_active_read_class
history_outbox_pending
history_outbox_oldest_age_ms
history_outbox_stale_claims_total
history_restore_drill_last_result
history_redaction_findings_total
history_dropped_events_total
```

User-visible badges:

```text
Rich history
Basic history
Visual restore only
History degraded
Private mode
Writer lagging
Restore gap
Storage pressure
Lower-fidelity mux capture
Data health warning
Maintenance pending
```

## Test plan

### Unit tests

- Diesel model insert/select.
- Command block state machine.
- Segment sequence assignment.
- Session commit sequence assignment.
- Idempotency key conflict.
- Capture receipt duplicate/conflict handling.
- Redaction scanner.
- Retention policy decision function.
- Backend capability report mapping.
- Command sensitivity classifier baseline.
- Writer lease state machine.
- Outbox lease expiry/reset policy.
- JSON payload schema validation.
- Alternate-screen parser/projection behavior.
- Integrity check runner.
- Data health/quarantine decision function.
- Support bundle redaction policy.
- Backup manifest/checksum builder.
- Storage pressure state machine.
- SQL domain constraint enum conversions.
- Capture semantics classifier.
- Backend capability report mapper.
- Privacy data-class classifier.
- Feature gate resolution logic.
- Key-ref model rejects accidental key material serialization.
- Restore guarantee derivation from evidence matrix.
- Capability report expiry rules.
- Command confidence/rerun policy for UI, shell marker, raw typed and rendered mux sources.
- Table-class FK/delete policy classifier.
- Restore page cursor boundary builder for inclusive event ranges and half-open byte ranges.
- Diesel table column-budget parser/checker.

### Integration tests

- Open temp DB, migrate, insert session/pane/segment/event.
- Run `PRAGMA foreign_key_check` on migrated DB.
- Generate/compare Diesel `schema.rs` from migrated temp DB.
- Verify reliable profile PRAGMAs on writer connection.
- Restart store, load restore plan.
- Save native session, kill runtime, restore visible history.
- Duplicate input with same idempotency key.
- Outbox worker crash/retry.
- Multi-pane snapshot restore uses one commit/high-water vector.
- Replay sandbox suppresses OSC52/title/bell side effects.
- Alternate-screen fixture restores with explicit fidelity label.
- Maintenance run does not delete canonical history under default policy.
- Delete request writes tombstone and is resumable.
- Export request defaults to redacted/inert manifest.
- Support bundle defaults to redacted diagnostics.
- Second writer cannot acquire active lease before expiry.
- Stale outbox claim returns to pending with backoff.
- Online backup/VACUUM backup record is written and target passes quick_check.
- Plain `.db` hot-copy helper does not exist in production backup API.
- Invalid stable status/kind values fail DB constraints.
- Rendered mux surface cannot be loaded as `raw_vt_stream`.
- Corrupted snapshot checksum writes health record and falls back when possible.
- Feature gate `force_disabled` overrides config/env enablement.
- Payload-bearing rows default to `encryption_state = plaintext` until encryption gate is enabled.
- Direct session/pane delete fails while canonical history exists, but derived rows can be rebuilt.
- Capability report stale state downgrades restore guarantee.
- Large restore reads pages without holding a SQLite statement across an async boundary.
- Restore semantics evidence refs point to real rows or explicit degraded/gap rows.

### Protocol compatibility tests

- Old JSON fixture without `restore_semantics_v2` deserializes and keeps conservative booleans.
- New JSON fixture with `restore_semantics_v2` deserializes in new clients and legacy booleans remain present.
- Legacy v1 snapshot-only row maps to `restore_semantics.replays_saved_screen_buffers = false`.
- V2 hydrated row maps to `restore_semantics.replays_saved_screen_buffers = true` only after evidence-backed restore path ran.
- `restore_semantics_v2.restore_guarantee_level` downgrades when gaps/health records/stale capability report exist.
- `evidence_refs` cannot reference missing rows unless the ref is an explicit degraded/gap pseudo-ref.
- Protocol minor compatibility test covers `ListSavedSessions`, `SavedSessionResponse` and `RestoreSavedSessionResponse`.
- Unknown future `restore_guarantee_level` is handled as degraded/unknown in UI mapping, not as rich history.
- Feature gate `force_disabled` removes/downgrades v2 semantics from outward responses without corrupting DB rows.

### Windows tests

- PowerShell command capture.
- cmd command capture.
- ConPTY output burst.
- resize storm.
- suspend/resume simulation where possible.
- long output restore.
- Windows path and app-data DB location smoke.
- disk-free probe handles Windows volumes and permission errors without panic.
- ConPTY close semantics are documented/tested: native restore does not promise process preservation after host restart.
- restored historical region is visually separated from the new native process after restart.
- `VACUUM INTO` backup path works on Windows app-data DB and the backup passes quick_check.
- zellij Windows probe records unsupported/degraded state cleanly when binary/JSON behavior is missing.

### Browser/Playwright tests

- type command, output appears, restart host, output still visible;
- input composer command persists as command block;
- clear browser storage/localStorage, restart app, command dock still loads from DB;
- browser reconnect replays missed output;
- duplicate submit does not duplicate command;
- saved session shows restore guarantee badge.
- visible warning appears when a synthetic history gap is restored.
- storage pressure warning appears for synthetic disk-full/degraded state.
- restore badge distinguishes raw replay from rendered scrollback hydration.
- UI exposes `Rich history` only when taxonomy conditions are met.
- large restore hydrates first snapshot quickly and streams remaining pages without blocking live output.
- historical output and new live prompt are separated by a visible live boundary.
- low-trust rendered/heuristic command history is not shown as one-click rerunnable without confirmation.
- stale mux capability report downgrades the badge after simulated backend version change.

### Reliability tests

Initial invariants:

```text
no overlapping stream segment event ranges per pane
no overlapping stream segment byte ranges per pane when byte ranges are known
journal event_seq is strictly increasing per pane
stream cursor next_event_seq is above persisted max event_seq
stream cursor next_byte_seq is above persisted max byte_high
commit_seq is strictly increasing per session
every canonical stream/journal row references an existing commit
capture receipt duplicate does not duplicate journal rows
command block output ranges refer to existing events
snapshot high_water_event_seq <= pane.last_event_seq
topology pane_high_water_json points to persisted pane event ranges
history gaps are visible as rows and/or journal events
default retention policy does not prune raw canonical history
storage pressure never silently prunes canonical history
search documents are redacted derived rows only
versioned JSON payload rows have known schema ids
integrity check failures persist diagnostic rows
data health records exist for corrupt/quarantined rows
raw content is not included in default support/AI/export projections
support bundle default excludes raw transcript and raw commands
commit-log-linked canonical rows are not cascade-deleted by commit changes
command history raw text may be null under privacy policy
only one active writer generation is valid at a time
stale outbox claims are recoverable
outbox rows are done/quarantined after worker drain
restore drill can hydrate all v2 sessions
backup record succeeded implies backup target quick_check passed or diagnostic explains why skipped
stable status/kind columns reject invalid enum values
rendered capture semantics are never treated as raw replayable stream
feature-gated capabilities can be disabled without corrupting canonical history
large restore/search reads are paged and do not keep SQLite read cursors open across async boundaries
restore guarantee badge is derived from evidence rows, not backend kind
stale backend capability report cannot produce RichHistory
rendered mux sources cannot be promoted to raw_vt_stream without probe evidence
historical restore pages never merge with live process output without a live boundary
production backup API has no plain hot `.db` copy path
table-class FK policy matches canonical/derived/audit/workflow matrix
```

Example invariant SQL checks:

```sql
-- overlapping segment event ranges per pane/stream
SELECT a.id, b.id
FROM terminal_stream_segments a
JOIN terminal_stream_segments b
  ON a.pane_id = b.pane_id
 AND a.stream_id = b.stream_id
 AND a.id < b.id
 AND a.event_seq_low <= b.event_seq_high
 AND b.event_seq_low <= a.event_seq_high;

-- overlapping segment byte ranges per pane/stream
SELECT a.id, b.id
FROM terminal_stream_segments a
JOIN terminal_stream_segments b
  ON a.pane_id = b.pane_id
 AND a.stream_id = b.stream_id
 AND a.id < b.id
 AND a.byte_low < b.byte_high
 AND b.byte_low < a.byte_high;

-- cursor below persisted journal
SELECT c.id, c.next_event_seq, MAX(e.event_seq) AS max_event_seq
FROM terminal_stream_cursors c
JOIN terminal_journal_events e
  ON e.event_scope_kind = 'pane'
 AND e.event_scope_id = c.pane_id
 AND e.stream_id = c.stream_id
GROUP BY c.id, c.next_event_seq
HAVING c.next_event_seq <= MAX(e.event_seq);

-- byte cursor below persisted segments
SELECT c.id, c.next_byte_seq, MAX(s.byte_high) AS max_byte_high
FROM terminal_stream_cursors c
JOIN terminal_stream_segments s
  ON s.pane_id = c.pane_id
 AND s.stream_id = c.stream_id
GROUP BY c.id, c.next_byte_seq
HAVING c.next_byte_seq < MAX(s.byte_high);

-- screen snapshots past pane high-water
SELECT s.id, s.high_water_event_seq, p.last_event_seq
FROM terminal_screen_snapshots s
JOIN terminal_panes p ON p.id = s.pane_id
WHERE s.high_water_event_seq > p.last_event_seq;

-- canonical rows without commit
SELECT e.id
FROM terminal_journal_events e
LEFT JOIN terminal_commit_log c ON c.id = e.commit_id
WHERE c.id IS NULL;

-- duplicate capture receipt payload mismatch
SELECT session_id, source_kind, source_event_id_hash, COUNT(DISTINCT source_payload_hash)
FROM terminal_capture_receipts
GROUP BY session_id, source_kind, source_event_id_hash
HAVING COUNT(DISTINCT source_payload_hash) > 1;
```

### Fuzz/property tests

- random output chunk boundaries;
- partial UTF-8 boundaries;
- event ranges are inclusive and byte ranges are half-open `[low, high)`;
- command output event range maps to the same bytes after segment splitting/merging;
- resize interleavings;
- command marker ordering;
- idempotency retry order.

## Acceptance criteria for first complete v2

Functional:

- user runs commands in Windows native backend;
- output is persisted;
- command blocks are persisted where source is reliable;
- command dock/history reloads from DB after browser storage is cleared;
- app restart restores visible history;
- saved sessions list shows v2 sessions;
- old v1 saved sessions still load as legacy/degraded;
- native restore does not claim process preservation.

Reliability:

- DB migrations pass from empty and legacy DB;
- production writer uses reliable durability profile;
- every canonical write is tied to session commit sequence;
- restore drill passes;
- writer can recover after injected transaction failure;
- duplicate submit test passes;
- duplicate capture receipt test passes;
- long output does not freeze UI or grow memory unbounded.
- default retention policy never silently prunes canonical history.
- safe backup path uses SQLite backup/VACUUM semantics, not plain hot `.db` copy.
- synthetic disk-full/storage-pressure path produces visible degraded state and durable diagnostics.
- stable domain constraints are tested in DB, not only in Rust.
- rendered mux history is labeled lower fidelity and is never claimed as raw replay.
- corrupt snapshot/segment tests produce data-health records and visible degraded restore.
- failed restore drill or stale capability report downgrades the API/UI guarantee instead of showing `Rich history`.

Security/privacy:

- raw input keystroke capture is off by default;
- export/AI context does not use raw transcript by default;
- search docs are redacted derived data;
- command history respects sensitivity/redaction policy;
- raw export requires explicit export request approval;
- historical replay suppresses terminal side effects;
- private mode behavior is explicit;
- prompt-injection-like output is data-only.

Windows:

- PowerShell smoke passes;
- cmd smoke passes;
- ConPTY resize/output smoke passes;
- no path/artifact external store assumptions in v1.

## Non-goals for the first implementation

Do not implement in first cut:

- full E2EE sync;
- cold object-store search;
- external artifact store for normal output;
- process checkpointing;
- automatic rerun of restored commands;
- full zellij resurrection semantics;
- AI autonomous terminal actions;
- raw keystroke logging.

## Most important implementation warnings

1. ⚠️ Do not parse command blocks from visible text only. Use UI submit and shell markers.
2. ⚠️ Do not let snapshots become source of truth. They are caches.
3. ⚠️ Do not block PTY reader on Diesel writes. Use bounded queue and writer worker.
4. ⚠️ Do not promise process restore for native backend.
5. ⚠️ Do not expand raw input logging by default.
6. ⚠️ Do not put external artifact paths into first implementation unless path safety is ready.
7. ⚠️ Do not expose terminal output to AI as instructions.
8. ⚠️ Do not claim "no data loss" without explicit tested scope.
9. ⚠️ Do not silently prune canonical history under the default policy.
10. ⚠️ Do not index raw terminal stream in FTS/search by default.
11. ⚠️ Do not treat zellij/tmux outer UI output as high-confidence command history.
12. ⚠️ Do not use `synchronous = NORMAL` while claiming reliable history durability.
13. ⚠️ Do not restore a multi-pane session from independently selected latest pane snapshots.
14. ⚠️ Do not replay OSC52/title/bell/hyperlink side effects during historical restore.
15. ⚠️ Do not treat command autocomplete history as raw transcript parsing; derive it from trusted command blocks.
16. ⚠️ Do not enable compression until restore drills verify compressed segments.
17. ⚠️ Do not use `ON DELETE CASCADE` from sessions, panes or commit log into canonical history rows.
18. ⚠️ Do not store command history raw text when privacy/redaction policy says not to.
19. ⚠️ Do not treat browser-provided client IDs as trusted identity.
20. ⚠️ Do not export raw history without an auditable export request and explicit approval.
21. ⚠️ Do not assume process-local single writer protects the DB from a second app process.
22. ⚠️ Do not let claimed outbox jobs stay claimed forever after worker crash.
23. ⚠️ Do not store unversioned canonical JSON payloads.
24. ⚠️ Do not treat alternate-screen/TUI output as ordinary shell scrollback.
25. ⚠️ Do not use inline nullable `UNIQUE(dedupe_key)` when a partial unique index says the intent better.
26. ⚠️ Do not export command hashes or support bundles as stable secret fingerprints.
27. ⚠️ Do not auto-repair or delete data after integrity check failure without explicit recovery flow.
28. ⚠️ Do not decode raw terminal bytes as UTF-8 in the canonical stream layer.
29. ⚠️ Do not back up a WAL-mode database by copying only the `.db` file while the app can be running.
30. ⚠️ Do not treat `SQLITE_FULL` as a normal transient query failure; it is a storage-pressure state.
31. ⚠️ Do not leave stable canonical status/kind fields as unconstrained free-form strings.
32. ⚠️ Do not claim reliable WAL history on an unknown/old SQLite runtime without version diagnostics and a downgrade/fail-closed path.
33. ⚠️ Do not replay rendered zellij/tmux surface as if it were raw VT stream.
34. ⚠️ Do not calculate restore guarantees from backend name without capability evidence and restore drill status.
35. ⚠️ Do not auto-delete corrupt snapshots/segments; record health/quarantine state and prefer explicit recovery.
36. ⚠️ Do not make heuristic or rendered-surface command text auto-rerunnable.
37. ⚠️ Do not show `Rich history` unless taxonomy conditions are satisfied by persisted evidence.
38. ⚠️ Do not add raw terminal content to support/AI/export by serializing DB rows wholesale.
39. ⚠️ Do not store encryption keys, DPAPI blobs, or keyring payloads in support bundles/logs.
40. ⚠️ Do not enable encryption/compression/mux capture without a feature gate and downgrade behavior.
41. ⚠️ Do not treat performance budgets as optional; missed budgets must produce diagnostics or visible degradation.
42. ⚠️ Do not stream large restore/search results while holding a SQLite cursor or transaction open.
43. ⚠️ Do not ship zellij/tmux as "Windows supported" without backend-specific runtime probes and route metadata.
44. ⚠️ Do not let advanced tracks block the native Windows durable-history MVP.

## Recommended first PR breakdown

### PR 1 - Diesel foundation

Scope:

- dependencies;
- connection initializer;
- durability profile;
- migrations runner;
- executor;
- `terminal_db_identity`;
- retention/maintenance baseline tables;
- backup/storage-pressure baseline tables;
- data-health and backend-capability baseline tables;
- stable enum/check-constraint policy;
- first schema tables;
- temp DB tests.

### PR 2 - v2 session/pane rows

Scope:

- create session/pane v2 rows;
- create session commit cursor rows;
- create stream cursor rows;
- legacy remains;
- session route preserved;
- tests.

### PR 3 - journal writer

Scope:

- capture events;
- capture receipts;
- commit log;
- writer queue;
- segment insertion;
- cursor-backed sequence allocation;
- history gap rows;
- reliable writer barriers;
- native output capture;
- capture semantics tagging;
- sequence invariants.

### PR 4 - snapshot restore

Scope:

- snapshots with `event_seq` and optional `byte_seq` metadata;
- topology snapshots with checksum;
- topology high-water vectors;
- restore plan;
- hydrate/replay visible history;
- two-phase restore page API;
- evidence-backed guarantee derivation;
- replay sandbox;
- restore drill row;
- data health fallback for invalid snapshots;
- protocol semantics update.

### PR 5 - command blocks

Scope:

- UI submit capture;
- command block state machine;
- command block list API;
- rerun trust guard.

### PR 6 - reliability gate

Scope:

- restore drill;
- idempotency tests;
- fault injection skeleton;
- invariant SQL checks;
- maintenance safety checks;
- safe backup checks;
- storage pressure checks;
- synthetic gap restore check;
- browser restart smoke.

## Final architecture statement

The correct architecture for this repo is:

```text
  Diesel-backed Terminal Persistence v2
  local-first SQLite
  bundled patched SQLite on Windows
  single durable writer
  reliable history durability profile
  measured performance budgets
  paged restore/read path
  explicit capture semantics
  backend capability evidence
  session-level commit log
  append-only journal
  UUIDv7 domain IDs
  durable stream cursors
  capture receipts for retry safety
  bounded stream segments
  versioned JSON payloads
  explicit terminal buffer modes
  explicit history gaps
  first-class command blocks
  DB-backed command history
  audited export/delete flows
  snapshots as cache
  replay sandbox
  redacted derived search
  integrity checks
  data health and quarantine records
  online backup records
  storage pressure diagnostics
  privacy data classification
  feature-gated rollout
  clear MVP cutline
  encryption-ready key hierarchy
  redacted support bundles
  outbox for derived work
  restore drills as proof
  retention/maintenance as explicit policy
  explicit restore semantics
  Windows-first testing
  reliability proof gates
```

This is the best balance of confidence, reliability and implementation complexity for the current codebase.
