# Deep Dive - Terminal History, Session Journal, Restore

**Проверено**: 2026-04-29  
**Фокус**: как лучшие терминалы сохраняют команды, вывод, scrollback, сессии и процессное состояние

## Короткий вывод

Надежная история терминала должна быть не "списком команд" и не "снимком экрана", а отдельным durable runtime layer:

- `Command Blocks` - структурированные команды пользователя с cwd, exit code, duration и связью с output.
- `Terminal Journal` - append-only поток фактов: input, output, resize, markers, title, clear, process exit.
- `Screen Snapshots` - быстрые точки восстановления viewport/scrollback.
- `Shell Integration` - semantic markers, чтобы не угадывать границы команд по тексту.
- `Restore Semantics` - честное разделение "восстановили историю" и "живой процесс продолжает работать".
- `Retention / Privacy` - лимиты, очистка, приватный режим, redaction.

Оценка целевой архитектуры: 🎯 10   🛡️ 10   🧠 9

## Сводка v2 после дополнительного ресерча

После второго раунда ресерча вывод стал жестче:

1. **Нужно строить не "history", а `Terminal Persistence v2`**  
   История команд, output, scrollback, session restore, zellij/tmux attach и AI-context - это разные read models поверх одного durable journal. Если смешать их в один JSON/blob, архитектура быстро станет хрупкой.

2. **Command Block - это продуктовая сущность, а не renderer decoration**  
   Warp и JetBrains явно идут к block-structured terminal. JetBrains даже exposes `TerminalCommandBlock` в embedded terminal API. Значит command block надо хранить в БД и отдавать через runtime API, а не вычислять на фронте из текста.

3. **Shell integration - обязательный слой качества, но не источник полной истины**  
   `OSC 633`/`OSC 133` дают границы команд, cwd и exit code, но markers может напечатать любая программа внутри терминала. Поэтому нужен `trust/integration_quality`, nonce, spoofing tests и fallback режимы.

4. **Полный output-history лучше хранить как segmented journal, а не row-per-line**  
   xterm/terminal stream не line-based: есть overwrite, cursor moves, alt-screen, resize, partial UTF-8, progress redraw. Значит durable stream должен быть chunk/segment-based с sequence ranges.

5. **Snapshots нужны для speed, journal нужен для correctness**  
   xterm.js serialize/addon-style snapshot полезен для быстрой hydration, но сам по себе не дает command boundaries, semantic search, raw replay и точную историю output.

6. **Raw input нельзя включать по умолчанию**  
   `script(1)` и xterm.js security docs подтверждают: terminal input легко содержит passwords/tokens, а web terminal вообще проводит keystrokes через JS. По умолчанию пишем command text из trusted sources, а не все keypress.

7. **SQLite подходит, но только с writer discipline**  
   SQLite WAL хорош для local-first history, но нужен single writer, batching, checkpoint policy, backup через SQLite API, и контроль WAL growth. Нельзя просто "включить WAL и забыть".

8. **Zellij/tmux - отдельная ветка semantics**  
   Native restore восстанавливает историю, но не живой процесс. Zellij/tmux могут сохранить живой процесс при attach, но raw outer PTY stream уже не равен одному shell transcript.

9. **Нельзя обещать "никогда не потеряется" без policy**  
   Реально надежная формулировка: сохраняем максимально полно в рамках quota/retention/private policy, показываем degraded state при сбое записи, делаем crash-safe batching и recoverable partial journal.

10. **Windows должен быть first-class path, не afterthought**  
    PowerShell/PSReadLine/cmd.exe/ConPTY/OSC 9;9 имеют свои ограничения. Нужно проектировать markers, cwd, history и restore сразу под Windows, иначе история будет "почти работает".

## Сводка v3 - что добавил третий проход

1. **Audit/session recording системы подтверждают journal-first подход**  
   `tlog` и Apache Guacamole пишут terminal I/O как запись с replay, timing и отдельным storage. Это ближе к нашей цели "ничего не терять", чем обычная shell history.

2. **Запись должна быть replayable при другой UI-среде**  
   Guacamole и tlog отделяют запись от playback UI. Значит наш journal должен уметь проигрываться не только в текущем React/xterm view, но и в debug/export/player режиме.

3. **Размер терминала является частью записи**  
   tlog playback прямо указывает на важность matching terminal size; asciinema/Guacamole тоже хранят resize/timing. Значит rows/cols/resize - не метаданные, а обязательные events.

4. **Shell history policies надо уважать**  
   Bash `HISTCONTROL=ignorespace`, fish private mode, Nushell history isolation, PSReadLine history settings - это user privacy intent. Наша persistence не должна молча обходить эти правила.

5. **Terminal escape sequences - активный контент**  
   OSC 8 hyperlinks, OSC 52 clipboard, window/title controls, bracketed paste, alt buffer - это не "текст". Restored replay должен отключать side effects и маркировать restored content.

6. **Recording feedback loop - реальная грабля**  
   Red Hat tlog docs описывают loop, когда просмотр логов генерирует новые логи, которые снова записываются. Для нашего history viewer нельзя писать replay/export обратно в active journal.

7. **Storage должен иметь maintenance API**  
   Нужны DB backup через SQLite API, integrity diagnostics, WAL checkpoints, compaction/vacuum policy и recovery из частично поврежденных segments.

8. **Достоверность command output должна быть доказуемой sequence ranges**  
   "Этот output принадлежит этой команде" должно опираться на stream sequence/event boundary, а не на позицию в scrollback.

## Сводка v4 - что добавил четвертый проход

1. **Есть два разных продукта: recording и restore**  
   Recording должен воспроизводить terminal session как historical artifact. Restore должен вернуть пользователя в рабочий контекст. Эти цели похожи, но не одинаковы: recording допускает player, restore требует live boundary и process semantics.

2. **Emulator state persistence не заменяет transcript**  
   VTE/xterm.js/WezTerm-style state snapshot сохраняет screen/cursor/modes, но не говорит "какая команда это создала" и не всегда сохраняет raw provenance. Transcript journal нужен отдельно.

3. **Line rewrap ломает naive text history**  
   GNOME/VTE и terminal emulators могут rewrap старые строки при resize. Если search/index строить по visible wrapped lines, результаты меняются от размера окна. Search index должен строиться из parser-derived logical text chunks, с recorded cols как контекстом.

4. **"Unlimited scrollback" не равен durable history**  
   GNOME Terminal разрешает unlimited scrollback, но предупреждает, что большой буфер замедляет resize. Это UI buffer, не надежный DB journal.

5. **GNU screen подтверждает старый паттерн: scrollback/log/hardcopy разные команды**  
   Scrollback, logging и hardcopy решают разные задачи. У нас тоже должны быть разные действия: view history, record journal, export transcript.

6. **Full-text search надо проектировать как derived index**  
   SQLite FTS5 подходит для searchable transcript, но FTS index не должен становиться source of truth. Source of truth - raw segments + events + snapshots.

7. **Encryption at rest - отдельное решение, не бесплатная настройка SQLite**  
   Обычный SQLite не шифрует БД. SEE/SQLCipher существуют, но добавляют dependency/build/key-management риски. Поэтому redaction/private mode нужны уже сейчас, даже до encryption.

8. **Deletion без compaction не всегда значит "данные физически исчезли"**  
   Для sensitive transcripts нужно отдельно проектировать secure delete expectations, VACUUM/compaction policy и backups. Просто `DELETE FROM` недостаточно для приватности на уровне диска.

## Сводка v5 - что добавил пятый проход

1. **sudo/sudoreplay подтверждает split между event logs и session recordings**  
   Sudo отдельно развивает structured logs, sub-command logging и full I/O session recordings. Это ровно наш split: command blocks/searchable events для навигации и raw transcript для replay.

2. **Password redaction нельзя откладывать**  
   Sudo 1.9.10 добавил hiding passwords in session recordings. Значит даже mature audit systems признают: raw I/O recording без redaction опасен.

3. **Unicode width - часть replay correctness**  
   CJK, emoji, variation selectors и ambiguous-width characters ломают cursor position и wrapping. В journal/snapshot metadata нужно хранить Unicode/cell-width policy или хотя бы parser/render version.

4. **Terminal output может содержать media, а не только text**  
   iTerm2 inline images, Kitty graphics protocol, Sixel и WezTerm imgcat показывают, что "вывод команды" может быть binary/media protocol. Поэтому output segment нельзя моделировать как plain UTF-8 lines.

5. **Paste is not typing**  
   Bracketed paste mode отделяет pasted text от typed input. Command capture и raw input policy должны хранить source: typed, pasted, UI submit, programmatic paste.

6. **Replay должен быть mode-aware и side-effect-safe**  
   Historical replay не должен менять clipboard, открывать links, скачивать файлы, рисовать images без user action или менять window state.

7. **Нужен tamper-evident option**  
   Для обычного developer history хватит checksum на segments. Для audit/compliance режима нужен hash chain или signed segments, чтобы detect-ить изменение transcript.

8. **"Чистый transcript" и "точный transcript" конфликтуют**  
   Чистый markdown/plain text удобен пользователю, но теряет control sequences, colors, cursor movement и media. Точный raw replay неудобен для чтения. Надо хранить оба как разные read models.

## Сводка v6 - что добавил шестой проход

1. **Input protocol тоже должен быть версионирован**  
   Kitty keyboard protocol, xterm `modifyOtherKeys`, CSI-u и WezTerm key encoding показывают: "какую клавишу нажали" не всегда равно bytes, которые ушли в PTY. Для replay/debug нужно хранить source и encoding policy.

2. **Mouse reporting - terminal input, не UI interaction**  
   Full-screen apps получают mouse events как escape sequences. История должна отличать user mouse reporting внутри terminal от кликов по нашей UI.

3. **Parser conformance надо тестировать отдельно**  
   vttest, libtsm и alacritty/vte показывают, что terminal parser - отдельная сложная система. Restore/replay нельзя считать надежным без fixtures и conformance tests.

4. **PowerShell Start-Transcript не заменяет наш journal**  
   Start-Transcript пишет input/output PowerShell session в текстовый файл, но это shell-level transcript, не terminal-level raw replay, не layout/session store и не zellij/native unified history.

5. **Export из terminal UI уже существует, но это не restore**  
   GNOME Terminal умеет save contents. Это полезный UX, но это text export из tab/window, а не durable semantic journal.

6. **Accessibility выигрывает от command blocks**  
   VS Code использует shell integration и accessible terminal commands для навигации между командами. Значит command blocks должны быть доступны screen reader и keyboard navigation, не только визуальные cards.

7. **Нужен terminal capability profile**  
   `TERM`, terminfo, keyboard protocol, shell integration, ConPTY/mux mode, Unicode version, graphics support - все это влияет на то, насколько replay/search/blocks точны.

8. **Replay tests должны быть golden-file based**  
   Для raw bytes + resize + snapshots нужны golden fixtures: input stream, expected screen, expected command blocks, expected search text.

## Сводка v7 - что добавил седьмой проход

1. **PTY/ConPTY boundary - это hard boundary правды**  
   Unix PTY и Windows ConPTY дают byte-stream boundary между terminal host и process. То, что выше этого уровня, может быть UI metadata; то, что ниже - shell/app semantics. История должна явно хранить, на каком уровне событие поймано.

2. **termios ECHO/ICANON важнее эвристик пароля**  
   На Unix shells/apps могут выключать ECHO для password input. Если мы видим termios state, это лучший signal для raw input redaction. На Windows/ConPTY такого простого универсального signal может не быть.

3. **pam_tty_audit подтверждает риск keystroke logging**  
   Linux audit может писать TTY keystrokes, но даже системные docs выделяют password logging как отдельную опасную опцию. Это еще раз подтверждает: raw input off by default.

4. **Mosh показывает альтернативу raw byte replay: state synchronization**  
   Mosh не просто пересылает byte stream; он синхронизирует terminal state. Для нашей restore-системы это важный паттерн: journal нужен для истории, а state snapshots нужны для fast convergence.

5. **dtach показывает минимальный detach без screen history**  
   dtach позволяет reattach process, но не хранит screen contents. Значит live process persistence и visual/history persistence - независимые features.

6. **Eternal Terminal/reconnect tools не решают transcript**  
   Они удерживают remote session при network drop, но не дают полноценную local command/output history model, search, block semantics или redaction.

7. **Нужно хранить capture layer**  
   Событие может быть поймано из UI submit, shell marker, PTY bytes, projection delta, mux pane API, audit layer. Без поля `capture_layer` нельзя оценить надежность.

8. **Process persistence надо описывать как matrix**  
   `native child process`, `daemon-managed native`, `zellij/tmux attach`, `mosh-like remote`, `dtach-like detach` имеют разные гарантии. Один bool `preserves_process_state` слишком груб для UI.

## Сводка v8 - что добавил восьмой проход

1. **tmux control mode лучше screen scraping для mux integration**  
   iTerm2 использует tmux control mode, чтобы показывать tmux panes как native UI. Это важный паттерн: если mux дает structured API/control mode, лучше использовать его, чем парсить внешний raw terminal stream.

2. **Escape passthrough внутри tmux/zellij не гарантирован**  
   OSC 7/52/133/633, images, user vars и shell integration markers могут быть swallowed, wrapped, changed or blocked mux-слоем. Поэтому shell integration quality должна быть per pane and per mux.

3. **OSC 7 cwd - это URI, не просто path**  
   WezTerm/iTerm2 используют current working directory tracking. Это может включать host/path, remote context и sensitive directory names. CWD events требуют parsing, redaction and trust.

4. **Zellij structured APIs дают лучший путь для pane output**  
   Zellij features include pane output streaming, JSON list-panes/list-tabs/current-tab-info, remote sessions and read-only tokens. Для zellij backend правильнее читать pane-level state/output, чем внешний raw mux stream.

5. **Multi-client sessions ломают простую модель viewport**  
   tmux/zellij/remote sessions могут иметь несколько clients с разным размером, read-only viewers, browser viewers. Надо различать pane content, client viewport и local UI viewport.

6. **Clipboard side effects are per-client, not global history**  
   OSC 52 через tmux/zellij/remote session может скопировать в clipboard конкретного клиента. Это не "output history" и не должно replay-иться глобально.

7. **Remote/session sharing требует access model**  
   Read-only viewers и pair-programming mode должны иметь отдельную запись: кто видел историю, кто вводил команды, кто мог копировать output.

8. **Mux-level restore может быть лучше native, но хуже semantic**  
   zellij/tmux могут сохранить live process, но могут скрыть shell markers. Native может дать лучший raw PTY transcript, но хуже process persistence. Это tradeoff, не линейная шкала.

## Сводка v9 - что добавил девятый проход

1. **Terminal history is event sourcing with projections**  
   Azure Event Sourcing pattern и Fowler подтверждают: append-only events - source of truth, snapshots/projections - read models. Это точно совпадает с нашей моделью: raw stream/events - truth, screen/search/blocks - projections.

2. **Проекции должны быть пересобираемыми**  
   Search chunks, command blocks, screen snapshots and AI context views могут стать stale после parser/redaction/schema upgrade. Поэтому каждая проекция должна иметь version and rebuild path.

3. **Tamper-evident audit - не просто checksum**  
   AWS CloudTrail digest files и Sigstore/Rekor показывают pattern: digest/hash chain plus signed checkpoints/transparency. Для developer mode хватит checksum, для strict/audit - hash chain + signed checkpoint.

4. **ANSI/Unicode output can be hostile log content**  
   OWASP injection guidance, CWE control-sequence injection и Unicode UTR36/UTS39 показывают: logs/transcripts are untrusted data. History viewer должен sanitize display, mark bidi/confusables and avoid terminal side effects.

5. **Log lifecycle is a product requirement**  
   NIST SP 800-92 описывает generation, transmission, storage, analysis and disposal of logs. Значит retention/deletion/backup/integrity are not polish. Это core.

6. **Replay must be deterministic enough to debug**  
   Для forensic mode нужно хранить parser version, environment profile, resize stream, event schema version and redaction version. Иначе "replay не совпадает" невозможно объяснить.

7. **Schema evolution is unavoidable**  
   Terminal history will live longer than parser/schema releases. Нужны migrations, event versioning, upcasters and derived view rebuilders.

8. **Viewer safety is separate from recording safety**  
   Даже если запись raw output корректна, просмотр этой записи может быть опасен из-за ANSI/OSC/Unicode deception. Viewer должен быть inert/sanitizing by design.

## Сводка v10 - что добавил десятый проход

1. **REPL/subshells are command domains**  
   Python/IPython/Node/psql and other REPLs have their own history, prompts and execution lifecycle. A shell-level command block like `python` or `psql` does not explain inner commands. Нужно моделировать nested command domains.

2. **Application history is not terminal history**  
   IPython stores history in SQLite, Node REPL has persistent history, psql stores history, Python readline can save history. These are useful signals/import sources, but they do not include terminal output, resize, panes, shell integration trust, or replay.

3. **Structured event schema should borrow from OpenTelemetry**  
   OpenTelemetry logs use timestamps, observed timestamps, event names, attributes, trace/span correlation and semantic conventions. Terminal journal events need similar structure: event_name, attributes, resource/session/pane identity, correlation IDs.

4. **Process creation audit can enrich, not replace, command blocks**  
   Windows Event 4688 and Sysmon process creation capture process command lines, but miss shell built-ins, aliases, functions and REPL commands. Process audit is optional correlation, not terminal truth.

5. **Large output storage needs BLOB strategy**  
   SQLite documents internal vs external BLOB tradeoffs and incremental BLOB I/O. Stream segments should have bounded size; huge media/log artifacts may need external content-addressed blobs later.

6. **Output chunks need content addressing and dedupe option**  
   Repeated media/images or huge generated logs should not bloat DB blindly. Hash-addressed artifacts with DB metadata are more scalable for cold/large data.

7. **Command identity should be correlation-based**  
   A command block should have correlation IDs that connect UI submit, shell markers, process creation, output segments, snapshots, search chunks, and export artifacts.

8. **Nested domain fidelity must be honest**  
   Shell command blocks can be high-fidelity while inner REPL commands are unknown unless app integration exists. UI should show "inside Python REPL - terminal transcript only" rather than fake command blocks.

## Сводка v11 - что добавил одиннадцатый проход

1. **SQLite can be corrupted by operational mistakes, not just bugs**  
   SQLite docs explicitly list corruption causes: broken filesystem locks, network filesystems, renaming/unlinking DB files while open, backup mistakes, file descriptor reuse, and external writes. History storage must have operational rules, not just schema.

2. **Recovery is a required feature, not disaster folklore**  
   SQLite has recovery guidance. Our history DB should support partial recovery: quarantine bad segments, rebuild projections, recover intact sessions, and report what was lost.

3. **Secret redaction must be multi-stage**  
   OWASP Logging and GitHub secret scanning show both static patterns and policy exclusions. Terminal output is worse than app logs: secrets can appear in commands, output, cwd, env, URLs, screenshots, media and exports.

4. **Redaction findings are data, not only transformations**  
   We need records of what was redacted, where, with what rule/profile/version, without storing the secret itself. This makes re-redaction and user deletion explainable.

5. **Backups must be history-aware**  
   WAL, external artifact store, search indexes, snapshots and deletion tombstones mean backup is a transaction across multiple storage surfaces. Simple file copy is not reliable enough.

6. **Storage health should be visible in UI and tests**  
   Users need to know when history is no longer guaranteed. Tests need disk-full/DB-locked/corruption/recovery scenarios. Storage health is part of product behavior.

7. **Network/cloud folders are dangerous default locations**  
   SQLite warns about unreliable locking on network filesystems. Project/workspace history DB should prefer local app data, not OneDrive/Dropbox/SMB/NFS folders unless explicitly configured and tested.

8. **Secret scanning has false positives and false negatives**  
   Redaction cannot be perfect. UI and policy must acknowledge `redacted`, `possibly_sensitive`, `not_scanned`, `scan_failed`, not pretend all history is safe.

## Сводка v12 - что добавил двенадцатый проход

1. **SQLite PRAGMA choices are product semantics**  
   `journal_mode=WAL`, `synchronous=NORMAL/FULL`, `busy_timeout`, `foreign_keys`, `wal_autocheckpoint`, `journal_size_limit` are not tuning trivia. They define durability, lock behavior, checkpoint cost and corruption risk.

2. **Writer transactions should be explicit**  
   SQLite `BEGIN DEFERRED/IMMEDIATE/EXCLUSIVE` matters. For a single history writer, `BEGIN IMMEDIATE` can fail early on writer contention instead of failing mid-batch. This makes degraded state easier to reason about.

3. **Durability should be profile-based**  
   Developer mode may choose WAL + `synchronous=NORMAL` for performance with visible best-effort semantics. Strict/audit mode should prefer stronger durability and be honest about latency.

4. **Diesel migrations are deployable, but need discipline**  
   `embed_migrations!` lets us ship migrations inside the binary, but docs note proc-macro limitations around rerunning when migration files change. Build/test must explicitly depend on migration files and verify DB schema.

5. **Compression must preserve random access**  
   Zstd is strong for terminal logs, but the standard zstd frame format is not random-access by itself. Segment-level compression or seekable-frame design is safer than compressing one giant stream.

6. **Batch boundaries are data-model boundaries**  
   Compression, checksums, redaction, pruning and replay all operate on segments. Segment size/time thresholds must be stable enough for performance and small enough for recovery.

7. **Foreign keys and cascading deletes need tests**  
   SQLite needs `PRAGMA foreign_keys=ON` per connection. If we rely on cascades for deleting sessions/history/artifacts, tests must prove it on every connection path.

8. **Maintenance is a runtime service**  
   `PRAGMA optimize`, WAL checkpoints, integrity checks, projection rebuilds, artifact GC and retention pruning should be scheduled and observable, not manually run in emergencies.

## Сводка v13 - что добавил тринадцатый проход

1. **OS storage/key APIs define the safe default**  
   Windows DPAPI/Credential Manager, Known Folders and XDG Base Directory spec show: history DB, keys and transient cache should live in different OS-appropriate places. Нельзя класть encrypted DB key рядом с самой DB как plain text.

2. **History writer needs SLI/SLO, not only unit tests**  
   SRE/OTel metrics guidance applies: writer lag, dropped segments, failed commits, checkpoint latency, WAL size, recovery success and redaction failures must be measurable. Иначе "история надежная" невозможно доказать.

3. **Backpressure exists at every layer**  
   PTY output, WebSocket bufferedAmount, browser render queue, SQLite writer queue and compression job all can backlog. Нужно явно хранить queue depth/dropped/degraded state.

4. **Fuzzing is mandatory for terminal parser/replay**  
   cargo-fuzz/libFuzzer and proptest are a good Rust fit. Terminal parser takes hostile byte streams by design, so fuzz tests should target parser, journal segment decoder, redaction, replay and export sanitizer.

5. **Property tests should encode invariants**  
   Examples: append+replay is deterministic, projections rebuild from raw, delete removes raw+derived, redaction never increases secret exposure, sequence ranges remain monotonic.

6. **DPAPI/OS keychain is not equal to encrypted history**  
   DPAPI can protect encryption keys/secrets; SQLCipher or app-level encryption protects DB contents. These are different layers.

7. **Cache/state/data separation matters**  
   Durable session history belongs to state/data, derived search cache can be rebuildable, exports belong to user-selected locations, runtime sockets/temp files belong to runtime/cache.

8. **Testing needs chaos scenarios**  
   Disk full, DB locked, crash mid-segment, corrupt artifact, stale projection, bad Unicode, malicious ANSI, WebSocket disconnect, zellij/tmux passthrough failure and Windows ConPTY resize must be tested as first-class scenarios.

9. **Serialize projection is a fast restore path, not the source of truth**  
   xterm.js serialize-style buffer export is useful for quick viewport/scrollback hydration after reconnect. Но canonical truth все равно должен быть server-side journal + snapshots, иначе нельзя доказать completeness, redaction and audit integrity.

10. **Backpressure must be end-to-end**  
    Browser WebSocket, gateway queue, Rust channel, DB writer batch and projection cache each need bounded queues and visible health metrics. Если хотя бы один слой "безлимитный", он станет скрытым местом потери истории или memory blowup.

## Сводка v14 - что добавил четырнадцатый проход

1. **asciicast v3 confirms append-friendly export semantics**  
   v3 keeps NDJSON-style header + event stream, adds exit events and clearer terminal metadata. Для нас это хороший внешний формат экспорта/импорта, но не внутренняя модель DB: internal journal can be richer.

2. **Event envelope should be standardized**  
   CloudEvents and W3C Trace Context show a mature shape for event identity/correlation: `id`, `source`, `type`, `time`, schema/version, trace/span-like IDs. Terminal events need similar envelope, not ad-hoc blobs.

3. **SQLite hardening is a separate persistence task**  
   `SQLITE_DBCONFIG_DEFENSIVE`, `TRUSTED_SCHEMA=OFF`, STRICT tables and runtime limits matter because terminal history stores hostile text and potentially imported DB/export files. Durable history must be typed and resource-bounded.

4. **Search/index inputs must be resource-limited**  
   SQLite limits docs explicitly recommend lower limits for attacker-influenced data. Terminal search queries, import files and huge output chunks should have caps, otherwise one paste/import can create local DoS.

5. **Windows command identity is not argv identity**  
   PowerShell parsing modes, `$PSNativeCommandArgumentPassing` and `CommandLineToArgvW` show that stored command text, displayed command, rerun command and argv are different things on Windows. Rerun must preserve shell/domain context.

6. **Backup/PITR is not session restore**  
   SQLite Session Extension and Litestream-like WAL replication are useful future patterns for sync/backup/point-in-time restore, but they do not replace terminal command/output journal semantics.

7. **External artifacts need authenticated chunk encryption**  
   For large artifact files outside DB, libsodium secretstream-style AEAD is a better pattern than unauthenticated encryption: it detects truncation, reorder, modification and supports chunked streams.

8. **Export/import needs schema validation**  
   JSON Schema 2020-12 and RFC 7464 JSON Text Sequences provide a robust pattern: every exported event stream should declare schema version, validate each event, and ignore/forward unknown compatible fields intentionally.

## Сводка v15 - что добавил пятнадцатый проход

1. **Secure deletion is not one SQL statement**  
   `DELETE` removes logical rows, but sensitive bytes can remain in free pages, WAL, backups, temp files, exports and derived indexes. Нужно проектировать deletion workflow: tombstone, purge, rebuild derived layers, checkpoint, compaction and backup retention.

2. **Temporary files are part of the threat model**  
   SQLite temp files, export bundles, decoded images and debug archives can hold terminal secrets. Temp/cache paths need the same OS storage policy, quotas and cleanup as the main DB.

3. **Key material needs lifecycle, not just storage**  
   `keyring`, `secrecy` and `zeroize` show the shape: keys/tokens should live in OS stores, secret access should be explicit, debug output should redact, and memory cleanup should be best-effort with documented limits.

4. **Accessibility must model terminal history as a log, not a bitmap**  
   WCAG status messages and ARIA live regions imply that history restore/degraded state/search results/output changes must be programmatically announced. Command blocks give an accessible timeline; canvas-only terminal output does not.

5. **Keyboard escape from terminal focus is product-critical**  
   Web terminal panes can trap focus because the terminal wants all keys. WCAG no-keyboard-trap guidance means we need documented keyboard exits, focus rings and command/history navigation outside raw terminal mode.

6. **Terminal media and file-transfer protocols are active artifacts**  
   iTerm2 OSC 1337 file transfer/images, SIXEL and Kitty graphics can carry large or sensitive binary payloads. They need size caps, quarantine, lazy decode, safe preview and redaction/export policy.

7. **Retention policy should be explainable to users**  
   ICO/NIST privacy guidance reinforces: retention periods must be justified and reviewable. For terminal history this means per-session policy, private mode, bookmarked exceptions, backup retention and deletion audit.

8. **Metrics should be actionable, not decorative**  
   Prometheus histogram guidance is useful for writer latency, replay latency, checkpoint duration and redaction/export duration. Buckets must answer product questions: "is history being durably saved fast enough?"

## Сводка v16 - что добавил шестнадцатый проход

1. **Search needs a migration path, not one fixed engine**  
   SQLite FTS5 is the right local-first baseline, especially with external/contentless-delete patterns and trigram search. Но для very large histories Rust-native Tantivy can become a future cold/global search projection. Search index must stay derived either way.

2. **Background work should be transactional**  
   Projection rebuilds, redaction scans, artifact GC, export preparation, sync checkpoints and compaction should be durable jobs/outbox rows committed with the DB state that requires them. In-memory `tokio::spawn` is not enough for history reliability.

3. **CRDTs are for collaborative metadata, not raw terminal truth**  
   Automerge/Yjs are useful patterns for concurrent bookmarks, annotations, layout notes or shared command docs. Raw terminal output should remain append-only event journal; CRDT delete/undo semantics do not map cleanly to privacy deletion and forensic transcript.

4. **Time model needs wall time and monotonic time**  
   W3C High Resolution Time and Rust `Instant` docs show why durations should use monotonic clocks, while user-visible timestamps need wall-clock time. Journal events need both or replay/debug timing becomes unreliable under clock skew.

5. **AI/RAG context is another export surface**  
   OWASP LLM Top 10 highlights prompt injection and sensitive disclosure. Terminal history attached to AI must be redacted, provenance-preserving, trust-scored, bounded and treated as untrusted data, not blindly pasted context.

6. **DB file identity should be explicit**  
   SQLite `application_id` and `user_version` are small but useful file-format guardrails. History DB should identify itself, schema version and compatibility before migrations/import/recovery touch user data.

7. **Job queues have library options, but core semantics should be ours**  
   `apalis-sqlite` shows a current Rust SQLite-backed job queue with heartbeat/orphan re-enqueueing. We can borrow the pattern, but the first version should keep persistence jobs in our own Diesel schema to avoid coupling history correctness to an RC dependency.

8. **AI-ready command history should cite exact ranges**  
   If AI receives command/output, it should get session/pane/block IDs, stream sequence ranges, redaction state and trust level. Otherwise answers become impossible to audit and prompt-injection boundaries blur.

## Сводка v17 - что добавил семнадцатый проход

1. **Windows process lifecycle needs Job Objects and console-control semantics**  
   Windows does not behave like Unix process groups. Job Objects, `AssignProcessToJobObject`, `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, `GenerateConsoleCtrlEvent` and ConPTY close semantics must be first-class if native Windows sessions should stop/restart reliably.

2. **Backup consistency has levels**  
   SQLite Online Backup API, snapshot isolation, sqlite3 snapshot API and Windows VSS all solve different consistency scopes. App-level history backup should prefer SQLite backup/export APIs; VSS is OS-level snapshot help, not semantic session restore.

3. **File watchers are hints, not integrity proof**  
   `notify`, ReadDirectoryChangesW and inotify can detect changes to artifact directories, but watchers can drop/duplicate/coalesce events. Integrity still requires DB manifest, checksums and periodic scan.

4. **Content addressing is useful for artifacts, but privacy deletion becomes harder**  
   IPFS/CID and BLAKE3 patterns show how immutable content-addressed blobs can make artifact integrity and dedupe strong. But if secret-containing blobs are content-addressed, deletion/GC/backup retention must be designed carefully.

5. **Process checkpointing is not the baseline product**  
   CRIU proves real process checkpoint/restore is possible in some Linux contexts, but it is privileged/platform-specific and not a Windows/native-terminal baseline. Our honest baseline remains: restore history/context, not arbitrary live process memory.

6. **Modern terminals are moving toward durable blocks and session protection**  
   Wave Durable Sessions explicitly preserves terminal history across reconnects and ties it to terminal blocks. WindTerm/iTerm2/Termius show users expect session restore, but mobile/background/sync limits force honest semantics.

7. **OTel semantic events help avoid naming chaos**  
   OpenTelemetry event semantic convention guidance says event names and attributes must be documented. Terminal events should have stable names like `terminal.output.segment`, `command.started`, `history.redaction.applied`, not random per-feature strings.

8. **AI terminal environments are adversarial**  
   Recent terminal-agent benchmark/reward-hacking work reinforces the same rule: terminal output/history passed to AI must be treated as untrusted, provenance-tagged content. The history layer should not become an invisible prompt-injection amplifier.

## Сводка v18 - что добавил восемнадцатый проход

1. **WebSocket is transport, not delivery guarantee**  
   RFC 6455 gives frames, ping/pong and close behavior, but missed terminal updates after reconnect need application-level sequence numbers, offsets, acknowledgements and replay. Socket.IO docs explicitly say extra delivery guarantees must be implemented at app level.

2. **Client restore needs server-side replay window**  
   Connection-state recovery patterns store session ID, last processed offset and missed packets for a bounded duration. Terminal stream clients need the same: per pane stream seq, last client ack, replay from DB/window, and degraded state if gap cannot be filled.

3. **Offline buffers can create reconnect storms**  
   Socket.IO offline behavior warns about buffered client events creating a spike on reconnect. Terminal UI should rate-limit queued input/actions after reconnect and never replay stale user input blindly.

4. **Remote/SSH/WSL are separate execution domains**  
   VS Code Remote SSH, WezTerm SSH domains, OpenSSH ControlMaster and WSL docs show: local Windows path, WSL path, remote path, ssh-mux domain and shell integration quality are different. History must store execution domain and path translation policy.

5. **WSL path/interop policy affects command provenance**  
   WSL can run Windows `.exe`, translate env vars via `WSLENV`, append Windows PATH and work across Linux/Windows filesystems with different performance/case rules. A command block must know whether it was native Windows, WSL Linux, Win32-from-WSL or WSL-from-Win32.

6. **Feature flags are required for persistence rollout**  
   Durable history changes are high-risk. OpenFeature/Fowler/LaunchDarkly/Unleash patterns suggest rollout flags, ops kill switches, migration flags and stale-flag cleanup. Persistence v2 should be progressively enabled by backend/profile/session, not flipped globally.

7. **Network chaos tests should be explicit**  
   Toxiproxy and `tc netem` provide repeatable latency, reset, timeout, loss and bandwidth-collapse scenarios. Reconnect/history tests need these, not only happy-path Playwright.

8. **Remote context privacy is bigger than cwd**  
   Remote host, user, SSH config alias, forwarded ports, agent forwarding, proxy settings, WSL distro name and path mappings can be sensitive. They should be redacted/classified like command/output, especially in exports and AI context.

## Сводка v19 - что добавил девятнадцатый проход

1. **Event payload format must be evolvable by design**  
   Protobuf, FlatBuffers, CBOR and MessagePack all show the same lesson: binary formats are fine only if schema/version/unknown-field/deprecated-field rules are explicit. For terminal history, JSON is easiest for early debug, but long-term event payloads need codec registry and upcasters.

2. **SQLite performance must be tested as schema contract**  
   Partial indexes, covering indexes, `EXPLAIN QUERY PLAN`, `ANALYZE` and `PRAGMA optimize` are not optional tuning. Large history UX depends on query-plan baselines for "latest blocks", "search context", "session restore", "delete/prune" and "artifact GC".

3. **Crash reports and telemetry can leak terminal secrets**  
   OpenTelemetry sensitive-data docs, Sentry scrubbing hooks, Crashpad and Windows Error Reporting privacy docs all point to the same risk: diagnostics may contain command text, paths, tokens and memory. Crash/telemetry export needs its own redaction gate.

4. **Legal hold changes deletion semantics**  
   Microsoft Purview eDiscovery holds and SEC electronic recordkeeping guidance show that "delete/prune" cannot always win. Enterprise/audit mode needs legal hold/retention override states, separate from normal user deletion.

5. **Quota and disk pressure are product behavior**  
   Browser storage quotas, OPFS quota behavior and Windows Storage Sense show storage can disappear or fail under pressure. History must have preflight, quota samples, soft/hard limits, backpressure and clear user warnings before data loss.

6. **Telemetry context propagation is privacy-sensitive**  
   OTel Baggage can propagate arbitrary key/value context and warns that sensitive baggage may leak to unintended resources. Terminal session IDs, cwd, remote host and user identifiers must not be propagated blindly as tracing baggage.

7. **Schema evolution requires old fixture corpus**  
   Every event/schema/payload codec change needs old DB/export fixtures and upcaster tests. Otherwise future releases will silently lose history, break search or misinterpret old command blocks.

8. **Enterprise mode must separate audit integrity from privacy deletion**  
   Developer mode optimizes for local privacy and cleanup. Audit/legal mode may require hold, immutable/audit-trail retention and reproducible records. The product must not pretend one policy satisfies both.

## Сводка v20 - что добавил двадцатый проход

1. **Browser tab lifecycle can break live delivery assumptions**  
   Page Lifecycle/Visibility docs warn that hidden/frozen/discarded tabs and unreliable unload events make "send final state on close" unsafe. Browser terminal client state must be resumable from server DB, not dependent on `beforeunload`.

2. **Multi-tab coordination needs explicit ownership**  
   BroadcastChannel and Web Locks give browser-side coordination primitives, but they are advisory and origin-scoped. A web terminal should track active viewer/input owner per session and prevent two tabs from accidentally sending conflicting input.

3. **Clipboard and OSC52 are side-effect channels**  
   Clipboard API requires secure context/permissions/user activation; OSC52 can write local clipboard from remote output. Historical replay must never trigger clipboard writes, and live OSC52 should be policy/consent-gated.

4. **Container terminals are separate domains**  
   Docker `exec`, `attach`, `logs`, logging drivers and Kubernetes `exec`/TTY semantics differ. Container logs are not the same as interactive TTY transcript, and `docker attach` can send input to the main process. History must distinguish container exec/attach/logs.

5. **Windows encoding/codepage is part of replay correctness**  
   Windows console code pages, PowerShell encoding differences and UTF-8/UTF-16 behavior can create mojibake in commands/output. Journal metadata needs encoding/codepage/profile data for Windows native/cmd/PowerShell sessions.

6. **Sleep/resume is a durability event**  
   Windows `WM_POWERBROADCAST`, `SetThreadExecutionState` and systemd inhibitors show suspend/shutdown can interrupt writers and transports. History writer should flush/checkpoint on power events where possible and mark resume gaps/reconnect state.

7. **Frontend storage locks are not persistence locks**  
   Web Locks can coordinate tabs, but canonical history lock/writer ownership remains server/runtime-side. Browser locks are useful for UI leader election, not DB correctness.

8. **Container/remote clipboard and file side effects need the same policy as terminal media**  
   Docker/Kubernetes/SSH sessions can bridge local/remote/container boundaries. Copy, paste, file transfer, hyperlinks and terminal media all need domain-aware side-effect controls.

## Сводка v21 - что добавил двадцать первый проход

1. **Local gateway is a high-risk API surface**  
   A localhost WebSocket/HTTP gateway that can write to a terminal is effectively a local command-control API. It needs loopback binding, per-launch tokens, Origin validation, message authorization, rate limits and audit logs. "It is only localhost" is not a security model.

2. **WebSocket handshake needs CSWSH defenses**  
   OWASP WebSocket guidance is direct: validate `Origin` during handshake, authenticate/authorize messages, validate payloads and log security violations. Terminal gateway should reject unknown origins and never rely on cookies alone.

3. **Private Network Access changes local web assumptions**  
   Chrome PNA/local-network preflights show browsers are actively tightening public-to-local/private access. Our local gateway should support explicit allowed origins and PNA preflight behavior where HTTP endpoints exist, while still requiring tokens.

4. **DNS rebinding and Host validation matter for local tools**  
   DNS rebinding can make an attacker-controlled origin point at `127.0.0.1`. Mitigations: bind to loopback, reject unexpected `Host`/`Origin`, require unguessable local token, avoid wildcard CORS, and do not expose unauthenticated command APIs.

5. **Named pipes are not automatically safe**  
   Windows named pipes need explicit security descriptors/access rights. If we move local control from TCP to named pipes, we still need ACLs, client identity, impersonation policy and auditability.

6. **Archive import/export is a path traversal surface**  
   Terminal debug/export bundles may contain files, artifacts and manifests. OWASP/Python zipfile warnings confirm archive extraction must normalize paths, reject absolute/parent traversal, size-limit entries and quarantine before import.

7. **HTML transcript/export viewer needs sandbox and CSP**  
   Terminal output can include hostile HTML-like text, links and escape-rendered content. Any HTML viewer/export must be inert by default: CSP, sandboxed iframe, no inline script, no `object/embed`, no local file access, no service-worker cache surprises.

8. **Desktop wrappers have their own webview security requirements**  
   Electron/Tauri docs emphasize navigation allowlists, CSP, disabled dangerous APIs and least privilege. If terminal-platform later ships desktop shell/webview, gateway tokens and webview security must be designed together, not as an afterthought.

## Сводка v22 - что добавил двадцать второй проход

1. **External artifacts need real crash-safe writes**  
   Atomic rename is not enough by itself. For external artifacts/manifests, safe write means same-directory temp file, write all bytes, `fsync` file, atomic replace/rename, then `fsync` directory where supported. On Windows, open-file replacement and sharing semantics need explicit tests.

2. **File locks are coordination hints, not a data model**  
   Windows `LockFileEx`, Unix `flock`/`fcntl` and Rust wrappers help prevent concurrent writers, but locks can be advisory, handle-scoped, released on process death, or behave differently on network filesystems. The DB/job state remains the source of truth.

3. **Redaction rules can become a denial-of-service vector**  
   OWASP ReDoS and Rust regex docs show why secret scanning/redaction should prefer linear-time engines, exact multi-pattern matching and bounded input. A single bad regex over huge terminal output can freeze history/export.

4. **Redaction needs rule lifecycle and performance budgets**  
   Rules need IDs, versions, test fixtures, match counts, runtime metrics and rollback. Secret scanning is not just a list of regexes in code.

5. **Share/export authorization needs policy engine boundaries**  
   OWASP Authorization, NIST ABAC, OPA and Cedar point to the same architecture: deny-by-default, server-side checks, policies over subject/object/action/environment, and tests. History share/export/rerun/delete should not be scattered `if` statements.

6. **Capability tokens need attenuation and revocation**  
   Biscuit/Macaroons/OAuth Token Exchange patterns show scoped delegated access: time bounds, caveats, audience/resource, purpose and revocation records. Sharing a session should create a narrow grant, not expose the whole history DB.

7. **Policy decisions should be audited with inputs**  
   When export/delete/rerun/share is allowed or denied, store which policy version, subject, resource, action and environment attributes were evaluated. Otherwise support and audit cannot explain why sensitive history left the machine.

8. **Multi-user saved sessions require object-level authorization**  
   Session, pane, command block, artifact, export bundle and AI context are different resources. Permissions must be checked per object and action, not only "can open workspace".

## Сводка v23 - что добавил двадцать третий проход

1. **Windows path string is not a stable artifact identity**  
   Microsoft docs around file IDs, final paths, reparse points and file streams show that `C:\path\file` is only a name, not proof that the same object stayed inside our store. For critical artifacts we need canonical/final path, root policy, volume serial and file ID checks.

2. **Artifact store must be hostile-path safe by design**  
   Windows reserved names, long paths, alternate data streams, trailing spaces/dots and `\\?\` namespaces can break naive export/import/storage. Export filenames must be generated/sanitized, not derived directly from command text or user output.

3. **Reparse points and symlinks are a Windows sandbox escape surface**  
   Junctions, mount points, symlinks and other reparse points can redirect operations outside the intended directory. Path validation before open is not enough; critical writes should verify after opening the handle.

4. **CreateFile sharing modes are part of reliability**  
   Open handles can block replace/delete/rename on Windows depending on access and share mode. History artifact writes need tests for readers, antivirus/indexers, parallel export and stale process handles.

5. **Filesystem watchers and USN are hints, not correctness**  
   ReadDirectoryChanges, notify, inotify and USN change journal help detect changes, but they can miss, overflow or only say that a change happened. Source of truth remains DB manifest + verifier scan.

6. **Case sensitivity is not uniform on Windows anymore**  
   WSL and per-directory case sensitivity mean `foo` and `FOO` can collide in one place and differ in another. Artifact IDs must be normalized/generated, and display names must not become storage keys.

7. **Rust path handling must keep OS strings opaque**  
   `PathBuf`/`OsStr` should be used for operational paths. Converting paths through UTF-8 `String` is acceptable only for display/logging with loss/escape markers, not for canonical storage identity.

8. **Windows path safety needs its own test matrix**  
   Long path, reserved name, ADS, reparse point, case sensitivity, open handle sharing, antivirus-like reader and cross-volume replace should be explicit regression fixtures.

## Сводка v24 - что добавил двадцать четвертый проход

1. **Exactly-once is not a transport feature, it is a product contract**  
   Kafka, Stripe and AWS idempotency patterns all point to the same answer: retries are normal, duplicates are normal, reconnect is normal. Reliable terminal history needs event IDs, per-stream sequence, idempotency keys, deduplication and replay windows.

2. **Transactional outbox is the right boundary between journal and workers**  
   Projection rebuilds, search indexing, export, sync and AI-context packaging should be committed as durable outbox jobs in the same DB transaction as state changes. Otherwise we get history row exists, but search/export/sync silently missed it.

3. **Snapshot is a manifest over event/artifact lineage, not a copy of reality**  
   Delta Lake, Iceberg and Hudi reinforce the pattern: snapshots are metadata pointing at immutable data files/log ranges. Terminal restore snapshots should record base seq, high-water seq, artifact refs, schema/projection versions and parent lineage.

4. **Object storage is not a filesystem**  
   S3/GCS/Azure object stores have versions, ETags, lifecycle policies, retention/legal hold and different overwrite/delete semantics. If terminal history sync later uses object storage, manifests must be immutable and version-aware.

5. **Backup quality is proven only by restore drills**  
   restic, Kopia and Borg patterns emphasize content-addressed chunks, manifests/indexes, verification and pruning. A terminal history backup is not "done" until a temp restore can rebuild journal, snapshots, search and redaction state.

6. **Sync conflicts must be visible branches, not silent merges**  
   Syncthing-style conflict files and file sync tools show that conflicts are a UX feature. Terminal byte streams from two writers must not be merged as text. Divergent histories should become branch/conflict records with user-visible resolution.

7. **CRDT is useful for metadata, not raw terminal transcript**  
   Collaborative notes/layout/favorites can use CRDT ideas later. Raw PTY/mux streams are ordered side-effect logs; merging them like editable text creates false history.

8. **Retention/deletion must understand backup and sync lineage**  
   Deleting local DB rows is not enough if chunks, object versions, exports, backups and remote manifests still reference the same transcript data. Tombstones and reachability analysis are part of privacy.

## Сводка v25 - что добавил двадцать пятый проход

1. **Searchable history needs a log-storage architecture, not one FTS table**  
   Loki, ClickHouse, Elasticsearch/OpenSearch, Quickwit and OpenObserve all split ingestion, chunk/segment storage, metadata indexes, query planning and lifecycle. Terminal history should do the same: canonical journal first, searchable projections second.

2. **Low-cardinality labels are the only safe indexed labels**  
   Loki docs repeatedly warn that labels should describe source with low cardinality. For terminal history, safe labels are `backend_kind`, `execution_domain`, `shell_kind`, `exit_status_class`, `trust_level`, `workspace_id`. Unsafe labels are full command text, cwd, hostname, token, PID and every git branch.

3. **Chunk catalog is the heart of scalable search**  
   Each terminal chunk should have min/max seq, min/max time, pane/session refs, byte ranges, checksum, compression, redaction profile, token/bloom summaries and artifact refs. Query should first choose chunks, then scan/index them.

4. **Hot/warm/cold tiers fit terminal history better than one store**  
   Hot recent sessions need fast local SQLite/FTS/projection. Warm history can use compressed chunks and FTS/Tantivy. Cold history can be object-store/searchable snapshot style with slower explicit UX and query budgets.

5. **Bloom/token/minmax indexes are prefilters, not answers**  
   ClickHouse and Parquet show the pattern: skipping indexes reduce reads only when query/selectivity fits. Terminal search must verify matches against real text/projection after a prefilter says "maybe".

6. **Full-text index lifecycle needs merge/optimize/repair jobs**  
   Lucene/Tantivy/SQLite FTS5 all have segment/merge/optimize behavior. Search index health must be observable, rebuildable and versioned by tokenizer/redaction/parser profile.

7. **Search authorization and redaction must happen before result display/export**  
   Search indexes can duplicate sensitive text. Every search path must know whether it uses raw, redacted, contentless or derived token indexes and must re-check policy before showing snippets.

8. **Query budget is a product feature**  
   Long cold-history searches need timeout, scanned bytes/chunks, partial result state and "continue search" UX. Without budgets, one broad search can freeze local history or hammer object storage.

## Сводка v26 - что добавил двадцать шестой проход

1. **Terminal output is indirect prompt-injection input**  
   OWASP, Microsoft and OpenAI all describe indirect prompt injection as hostile instructions embedded in data the agent later reads. Terminal output is exactly that: untrusted data produced by arbitrary programs, remote hosts, build logs, tests, web pages, package managers and attackers.

2. **AI context must be structured, not pasted transcript text**  
   Command, output, exit code, cwd, trust level, redaction state and provenance ranges must be separate fields. Raw transcript pasted into a prompt loses the boundary between user intent and hostile terminal data.

3. **The model should not be the security boundary**  
   Microsoft agent safety guidance and OWASP agent guidance point to the same pattern: least privilege, tool scoping, human approval, output validation and runtime monitoring. The LLM can propose, but deterministic policy must approve actions.

4. **Rerun/share/delete/export from AI needs action gates**  
   An injected line in terminal output must not be able to cause command rerun, file write, history export, share link creation or deletion. Every AI-originated action needs resource scope, policy check, user intent evidence and audit.

5. **Prompt-injection detection is a risk signal, not a proof of safety**  
   Prompt shields, classifiers, Giskard, garak and PyRIT help find and test attacks, but no filter fully solves prompt injection. Detection should downgrade trust, require confirmation or strip context, not declare content safe forever.

6. **Context budgets are privacy/security controls**  
   The safest AI context is minimal: only selected command blocks, redacted snippets, explicit provenance and no raw secrets. Token budget, byte budget, chunk count and sensitive findings should be visible before sending context.

7. **AI red-team fixtures should be part of terminal persistence tests**  
   The history layer needs fixtures where command output contains prompt injection, hidden Unicode, ANSI hyperlinks, encoded instructions, fake approval text and malicious shell suggestions. Tests should assert that action gates still block.

8. **MCP/tool ecosystems add supply-chain risk**  
   Microsoft MCP guidance highlights indirect injection and tool-change risk. Terminal history exposed through MCP/resources/tools must version schemas, pin tool permissions, audit prompts and detect tool capability drift.

## Сводка v27 - что добавил двадцать седьмой проход

1. **Надежность нужно доказывать исполняемыми инвариантами**  
   SQLite, FoundationDB, TigerBeetle and Jepsen show the same pattern: critical storage systems are not validated by happy-path tests. They define invariants, inject faults, replay seeds and check resulting histories.

2. **Deterministic simulation is the highest-leverage test layer**  
   FoundationDB and TigerBeetle-style simulation suggests a path for Terminal Persistence: simulate PTY/mux output, browser reconnect, writer batching, DB errors, filesystem faults, clock jumps and power loss under one seed.

3. **Fault injection belongs at every persistence boundary**  
   SQLite tests OOM, I/O errors and crash/power-loss cases; Chaos Mesh and Linux fault injection cover network and filesystem faults. Our writer should inject failures around `write`, `flush`, `rename`, DB commit, checkpoint, outbox claim and browser ack.

4. **Formal/model checking is realistic for the small protocols**  
   We do not need to model an entire terminal emulator in TLA+. But seq/ack/replay, idempotency keys, single-writer locks, outbox/inbox, snapshot lineage and deletion/tombstones are small enough for TLA+/Apalache/Stateright-style checking.

5. **Concurrency testing must target the Rust async edges**  
   Loom/Shuttle-style schedule exploration is appropriate for the writer queue, outbox worker claim/retry, lock manager, delivery ack state and shutdown/flush races. Normal stress tests rarely find those deterministically.

6. **Chaos experiments need steady-state hypotheses**  
   Randomly breaking the app is weak. Each chaos run should define expected user-visible behavior: no committed journal corruption, restore indicates gap, outbox retries, no duplicate command submit, no raw data leak, recovery finishes within budget.

7. **Every failure needs a replay artifact**  
   Failed reliability tests should store seed, schedule, fault plan, DB copy/export bundle, event trace, runtime versions and screenshot/log references. Otherwise "rare bug" remains rare.

8. **Release readiness should include a persistence checklist**  
   SQLite-style release checklists are a practical pattern. Terminal persistence changes should require passing crash, fault, migration, restore, redaction, Windows path, search, AI-context and backup drill gates before release.

## Сводка v28 - что добавил двадцать восьмой проход

1. **Encryption-at-rest не заменяет redaction/private mode**  
   OWASP and NIST guidance is clear: encryption reduces damage after storage theft, but it does not protect data while the app is unlocked, during export, in AI context, screenshots, logs or compromised runtime. Redaction and policy remain required.

2. **Нужна envelope encryption hierarchy, а не один пароль от БД**  
   Best pattern: root wrapping key in OS key store or passphrase-derived key, KEKs for scopes, DEKs per database/artifact stream/search shard/export. Rotate KEK by rewrapping DEKs, not by rewriting all transcript chunks.

3. **SQLCipher protects SQLite, not external artifacts by magic**  
   SQLCipher/SEE can encrypt DB pages. But compressed stream chunks, media artifacts, search shards, backups and export bundles outside SQLite need separate AEAD/secretstream encryption and authenticated manifests.

4. **Associated data is part of integrity**  
   Encrypted chunks should authenticate metadata: session, pane, artifact ID, seq range, codec, schema version, redaction profile and key version. Otherwise a valid ciphertext can be replayed in the wrong context.

5. **Key rotation/rekey is a crash-sensitive workflow**  
   `PRAGMA rekey`, KEK rotation, DEK rewrap and export key rotation need job states, verification and rollback/quarantine. A crash during rekey must never leave "maybe encrypted, maybe lost" state.

6. **Cryptographic erase is powerful only if keys are granular**  
   NIST 800-88 supports cryptographic erase as a sanitization approach, but it only works if the target data is encrypted and the relevant key can be destroyed. One global key makes selective deletion impossible.

7. **OS key stores have different guarantees and failure modes**  
   Windows DPAPI user vs machine scope, macOS Keychain and Linux Secret Service differ in UI prompts, headless support, roaming, backups and multi-user behavior. Terminal-platform needs a capability profile and fallback policy.

8. **Recovery UX is a security architecture decision**  
   If local-only key is lost, history is unrecoverable. If recovery keys exist, they become sensitive secrets. Product must choose between local-only privacy, recovery phrase, OS keychain sync and enterprise escrow per profile.

## Источники и что забираем

| Система | Что делает | Что берем |
| --- | --- | --- |
| [Warp Blocks](https://docs.warp.dev/terminal/blocks) | Команда и output живут как один Block. Есть copy command, copy output, re-input, share, bookmark. | Command Block должен быть first-class сущностью, а не UI-эффектом поверх scrollback. |
| [Warp Command History](https://docs.warp.dev/features/entry/command-history) | История изолирована по shell session, позже объединяется. Есть exit code, directory, thread, duration, last run. | История команд должна быть per session / per pane, с rich metadata. |
| [Warp Session Restoration](https://docs.warp.dev/features/sessions/session-restoration) | Восстанавливает windows/tabs/panes и последние Blocks, хранит в SQLite. Есть clear для sensitive blocks. | SQLite нормален для local-first терминала, но нужна явная очистка sensitive history. |
| [Warp Background Blocks](https://docs.warp.dev/features/blocks/background-blocks) | Background output получает отдельные blocks, но attribution ограничен. | Нужно отдельное состояние `background/unknown`, нельзя притворяться, что любой output точно принадлежит foreground command. |
| [Warp How Warp Works](https://www.warp.dev/blog/how-warp-works) | Традиционная grid модель недостаточна для Blocks, потому input/output разных команд могут конфликтовать в одной grid line. | Нельзя строить Blocks только из final terminal grid. Нужен semantic journal. |
| [VS Code Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration) | `OSC 633` дает prompt start/end, pre-exec, finish, command line, cwd и nonce против spoofing. | Поддержать `OSC 633` как rich protocol, включая nonce. |
| [VS Code Terminal Advanced](https://code.visualstudio.com/docs/terminal/advanced) | Есть process reconnection и process revive; scrollback restore настраивается отдельно. ConPTY может reprint screen. | Restore должен разделять live-process attach и relaunch с восстановленной историей. На Windows нужны ConPTY guards. |
| [Windows Terminal Shell Integration](https://learn.microsoft.com/en-us/windows/terminal/tutorials/shell-integration) | `OSC 133` marks, select command/output, scroll between prompts, PowerShell example с exit code. | Базовый protocol - `OSC 133`, особенно для Windows shells. |
| [Windows Terminal Profile Advanced](https://learn.microsoft.com/en-us/windows/terminal/customize-settings/profile-advanced) | Профиль задает `historySize` для scrollback. | Даже modern terminal ограничивает scrollback, durable history должен быть отдельным disk-backed слоем. |
| [iTerm2 Shell Integration](https://iterm2.com/documentation-shell-integration.html) | Трекает command history, cwd, hostname, даже over ssh. | Shell integration нужна как first-class capability, не как optional decoration. |
| [iTerm2 Session Restoration](https://iterm2.com/3.4/documentation-restoration.html) | Живые job сохраняются через long-lived servers; reboot убивает jobs, но window content может восстановиться. | Честно показывать пользователю: process state и visual/history state - разные вещи. |
| [Zellij Session Resurrection](https://zellij.dev/documentation/session-resurrection.html) | Сериализует session layout, commands, опционально viewport/scrollback; пишет раз в 1 секунду; команды не auto-run, ждет ENTER. | Restore команд без подтверждения опасен. Для zellij путь отличается от native. |
| [tmux man page](https://man7.org/linux/man-pages/man1/tmux.1.html) | Server переживает detach, есть history-limit, capture-pane, alt-screen capture. | Для настоящей живой persistence нужен внешний server/mux. Scrollback всегда лимитирован. |
| [tmux-resurrect](https://github.com/tmux-plugins/tmux-resurrect) | Восстанавливает sessions/windows/panes/cwd/layout/focus, optional pane contents, conservative restore programs. | Program restore должен быть allowlist/idempotent, не "перезапускаем всё". |
| [WezTerm Shell Integration](https://wezterm.org/shell-integration.html) | Поддерживает `OSC 7`, `OSC 133`, `OSC 1337`, user vars, command zones. | Нужно принять несколько protocols, а не один vendor escape. |
| [WezTerm scrollback_lines](https://wezterm.org/config/lua/config/scrollback_lines.html) | Scrollback ограничен строками. | UI scrollback и durable journal имеют разные лимиты и lifecycle. |
| [Kitty Shell Integration](https://sw.kovidgoyal.net/kitty/shell-integration/) | Prompt marks, command output browsing, cwd, clone shell; много shell-hook edge cases. | Нужен quality indicator для shell integration, потому hooks ломаются темами и plugins. |
| [Kitty scrollback_lines](https://sw.kovidgoyal.net/kitty/conf/#opt-kitty.scrollback_lines) | Очень большой scrollback не рекомендуют из-за RAM/perf, предлагают pager history. | Не делать infinite in-memory scrollback. Durable history должна быть disk-backed. |
| [Ghostty Shell Integration](https://ghostty.org/docs/features/shell-integration) | Auto-inject для shells, но при switching shell integration теряется. Есть manual setup. | Auto-injection недостаточна. Нужны detect/degraded/manual paths. |
| [asciicast v2](https://docs.asciinema.org/manual/asciicast/v2/) | NDJSON event stream: output, input, marker, resize. Incremental writing survives crash better. Input off by default. | Journal должен быть append-friendly и crash-friendly. Input capture - opt-in/filtered. |
| [Guacamole terminal recording](https://guacamole.apache.org/doc/gug/configuring-guacamole.html#graphical-recording) | Guacamole умеет session recording и playback через отдельный viewer. | Нужен replay/export/player path, независимый от live terminal UI. |
| [tlog](https://github.com/Scribery/tlog) | tlog пишет terminal I/O как JSON, поддерживает playback, follow, partial/corrupt replay и rate limits. | Journal writer должен иметь latency budget, buffering, storage policy и partial replay. |
| [Red Hat session recording](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/8/html-single/recording_sessions/recording_sessions) | Session recording имеет notice, playback, SSSD scope и предупреждает про feedback loops. | Для enterprise-grade истории нужны notice/private policy и запрет replay self-recording. |
| [Teleport Audit and Recorded Sessions](https://goteleport.com/docs/reference/deployment/monitoring/audit/) | Recorded SSH sessions можно replay-ить; recording modes бывают `best_effort` и `strict`. | Нужна policy: при сбое записи продолжать с degraded warning или блокировать сессию. |
| [script(1)](https://man7.org/linux/man-pages/man1/script.1.html) | Логирует raw terminal data и timing; предупреждает, что input log пишет passwords даже без echo. | Нельзя включать raw input logging без privacy model. |
| [xterm.js Buffer API](https://xtermjs.org/docs/api/terminal/interfaces/ibuffernamespace/) | Есть normal/alternate buffers. Active buffer может меняться. | Restore должен понимать normal vs alt screen, иначе TUI restore будет ложным. |
| [Atuin Shell Integration](https://docs.atuin.sh/cli/guide/shell-integration/) | Preexec/precmd hooks пишут command, cwd, timestamp, exit code, duration; есть session id и filters. | Command history лучше строить из shell lifecycle hooks, с filters и ignorespace. |
| [SQLite WAL](https://www.sqlite.org/wal.html) | WAL дает concurrent readers и single writer, но требует checkpointing; WAL file часть durable state. | Journal writer должен быть single-writer service с batching и checkpoint policy. |
| [JetBrains TerminalCommandBlock API](https://plugins.jetbrains.com/docs/intellij/embedded-terminal.html) | IDE Terminal API exposes `TerminalCommandBlock`, command, output text и lifecycle hooks. | Command block становится platform API, не только UI decoration. |
| [JetBrains New Terminal](https://blog.jetbrains.com/idea/2024/02/the-new-terminal-beta-is-now-in-jetbrains-ides/) | Новый terminal отделяет prompt/editor-like command entry и output blocks, но rollout требует compatibility fallback. | Block UX полезен, но надо оставить classic/degraded path для shells/TUI, где block semantics неточные. |
| [Nushell configuration](https://www.nushell.sh/book/configuration) | History может быть `plaintext` или `sqlite`; есть `sync_on_enter` и лимиты. | Даже shell-level history уже требует policy: формат, синхронизация, изоляция, лимит. |
| [Atuin Sync](https://docs.atuin.sh/guide/sync/) | Shell history sync шифруется end-to-end; local-first история может синхронизироваться. | Если позже делать cloud/shared history, encryption/redaction должны быть заложены заранее. |
| [Atuin Deleting History](https://docs.atuin.sh/cli/guide/delete-history/) | Удаление истории должно учитывать sync и local/global deletion semantics. | Clear history - это не один DELETE. Нужны scopes и tombstones/sync-aware deletion. |
| [PowerShell PSReadLine about](https://learn.microsoft.com/en-us/powershell/module/psreadline/about/about_psreadline) | PSReadLine управляет command history, prediction, key handling и history save behavior. | На Windows command capture должен учитывать PSReadLine, а не только PTY text. |
| [PowerShell about_History](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_history) | PowerShell session history и persistent PSReadLine history - разные вещи. | Нельзя путать shell history, our command blocks и persisted transcript. |
| [doskey command history](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/doskey) | cmd.exe history управляется `doskey`, поддерживает macros и command recall. | Для cmd.exe reliable command history ограничена, нужен lower fidelity mode. |
| [Microsoft VT sequences](https://learn.microsoft.com/en-us/windows/console/console-virtual-terminal-sequences) | Windows Console/ConPTY поддерживает VT sequences, включая alternate buffer и cursor/state controls. | Windows replay должен учитывать VT modes, alt buffer, cursor state и resize. |
| [Microsoft Command-Line OSC 9;9](https://devblogs.microsoft.com/commandline/shell-integration-in-the-windows-terminal/) | `OSC 9;9` используется Windows Terminal для current working directory notifications. | Cwd source matrix должна включать Windows-specific `OSC 9;9`. |
| [Bash history facilities](https://durak.org/sean/pubs/software/bash-5.2/bashref_21.html) | `HISTCONTROL` и related history settings могут скрывать команды из shell history. | Нужно уважать user intent вроде `ignorespace` при command capture. |
| [fish interactive use](https://fishshell.com/docs/current/interactive.html#private-mode) | fish private mode не сохраняет команды в history. | Private shell mode должен переводить наш journal в no-command/no-output или explicit consent mode. |
| [zsh options](https://zsh.sourceforge.io/Doc/Release/Options.html) | zsh имеет `HIST_IGNORE_SPACE`, `INC_APPEND_HISTORY`, `SHARE_HISTORY` и другие history options. | Shell integration должна учитывать разные policies и shared history behavior. |
| [xterm control sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) | xterm sequences включают alternate screen, OSC, title, clipboard, modes. | Persisted output не простой текст; replay должен быть terminal-aware. |
| [OSC 8 hyperlinks](https://iterm2.com/documentation-escape-codes.html) | Terminal hyperlinks задаются escape sequences. | Links из restored output должны быть sanitized и не должны auto-trigger actions. |
| [Alacritty config](https://alacritty.org/config-alacritty.html) | Scrollback history задается лимитом строк. | Еще одно подтверждение: UI scrollback не должен быть infinite memory store. |
| [xterm.js Security](https://xtermjs.org/docs/guides/security/) | Web terminal input/output обрабатываются JS; keystrokes sensitive; escape sequences и links требуют осторожности. | История и replay должны считать terminal data untrusted и privacy-sensitive. |
| [xterm.js addons](https://xtermjs.org/docs/guides/using-addons/) | Addons расширяют terminal API; serialize addon в xterm.js ecosystem используется для state serialization. | Snapshot restore можно делать как optimization, но не как единственный journal. |
| [SQLite Backup API](https://www.sqlite.org/backup.html) | SQLite рекомендует backup API для корректной копии live database. | Нельзя копировать `.sqlite3` файл без учета WAL/SHM. |
| [SQLite WAL checkpoints](https://www.sqlite.org/wal.html#checkpointing) | WAL checkpoints нужны, иначе WAL может расти и влиять на производительность. | Нужны WAL metrics, checkpoints и короткие readers. |
| [SQLite limits](https://www.sqlite.org/limits.html) | SQLite имеет explicit limits, включая размер БД и строк/blob. | Нельзя проектировать unbounded blobs/rows без size guards. |
| [GNU screen logging manual mirror](https://www.math.utah.edu/docs/info/screen_17.html) | screen разделяет scrollback, logging и hardcopy. | View buffer, recording и export должны быть разными features. |
| [GNOME Terminal scrolling](https://help.gnome.org/users/gnome-terminal/stable/pref-scrolling.html.en) | Unlimited scrollback возможен, но большой scrollback может замедлять resize. | UI scrollback не должен быть нашим durable store. |
| [SQLite FTS5](https://www.sqlite.org/fts5.html) | FTS5 дает full-text search virtual tables. | Search index делать derived layer поверх journal, не source of truth. |
| [SQLite VACUUM](https://www.sqlite.org/lang_vacuum.html) | VACUUM rebuilds database file and can reduce file size after deletes. | Sensitive deletion требует compaction/backup policy, не только DELETE. |
| [SQLite PRAGMA optimize](https://www.sqlite.org/pragma.html#pragma_optimize) | SQLite рекомендует `PRAGMA optimize` для анализа query planner statistics. | History DB maintenance должна быть explicit, измеряемой и тестируемой. |
| [SQLCipher](https://www.zetetic.net/sqlcipher/) | SQLCipher шифрует SQLite databases, но требует отдельной сборки и key management. | Encryption at rest планировать отдельно, не заменяя redaction/private mode. |
| [libvterm](https://www.leonerd.org.uk/code/libvterm/) | libvterm отделяет terminal parser/state machine от UI. | Parser/projection должен быть отдельным replayable слоем. |
| [ttyrec](http://0xcc.net/ttyrec/) | ttyrec пишет terminal session с timing для playback. | Минимальная recording-модель: timed stream events, но нам нужны metadata/trust/privacy поверх. |
| [sudo 1.9.10 hide passwords](https://www.sudo.ws/posts/2022/03/sudo-1.9.10-hiding-passwords-in-session-recordings/) | sudo добавил hiding passwords in session recordings. | Raw I/O recording требует redaction/password-aware режимов. |
| [sudoers I/O logging](https://www.sudo.ws/docs/man/sudoers.man/#I_O_LOG_FILES) | sudoers описывает I/O log files, timing и replayable terminal logs. | Recording storage должен хранить timing, terminal size и иметь replay tooling. |
| [sudoreplay manual](https://www.sudo.ws/docs/man/sudoreplay.man/) | sudoreplay умеет проигрывать session I/O logs, list/search и filter. | Нужен history player/search вне live terminal pane. |
| [Unicode East Asian Width](https://www.unicode.org/reports/tr11/) | Unicode задает width properties, влияющие на terminal cell layout. | Parser/render version и width policy должны быть частью snapshots/replay. |
| [xterm.js Unicode handling](https://xtermjs.org/docs/guides/encoding/) | xterm.js описывает Unicode width handling и версионность Unicode addons. | Search/rendered transcript зависит от Unicode version, это надо версионировать. |
| [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) | Kitty поддерживает передачу images через terminal protocol. | Output segments должны быть bytes/media-aware, не line-only text. |
| [iTerm2 inline images](https://iterm2.com/documentation-images.html) | iTerm2 имеет inline image escape protocol. | История вывода должна учитывать media payload/redaction/export. |
| [WezTerm imgcat](https://wezterm.org/imgcat.html) | WezTerm поддерживает image display протоколы. | Replay/export должны иметь policy для terminal images. |
| [xterm bracketed paste](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) | xterm control sequences включают bracketed paste mode `?2004`. | Paste нужно отличать от typing и command submit. |
| [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) | Kitty описывает расширенный keyboard protocol для точной передачи key events. | Input history/debug должен знать encoding policy, не только final PTY bytes. |
| [WezTerm key encoding](https://wezterm.org/config/key-encoding.html) | WezTerm поддерживает разные key encoding protocols, включая CSI-u и kitty. | Нужен terminal capability profile для input replay/debug. |
| [xterm modified keys](https://invisible-island.net/xterm/modified-keys.html) | xterm `modifyOtherKeys` меняет encoding клавиш с modifiers. | Raw input bytes нельзя обратно надежно превратить в user key без protocol metadata. |
| [vttest](https://invisible-island.net/vttest/) | vttest проверяет VT100/xterm terminal behavior. | Parser/replay layer должен иметь conformance/golden tests. |
| [libtsm](https://github.com/Aetf/libtsm) | libtsm - terminal state machine library. | Parser/state machine лучше держать как отдельный слой от UI. |
| [Alacritty vte](https://github.com/alacritty/vte) | Alacritty выделяет ANSI parser в отдельный crate. | Terminal parsing should be independently testable and versioned. |
| [PowerShell Start-Transcript](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.host/start-transcript) | PowerShell умеет писать transcript session в текстовый файл. | Shell transcript полезен, но не заменяет terminal-level journal. |
| [GNOME Terminal save contents](https://help.gnome.org/users/gnome-terminal/stable/txt-save-text.html.en) | Terminal UI может сохранить visible/tab contents в файл. | Export text и durable restore - разные features. |
| [VS Code terminal accessibility](https://code.visualstudio.com/docs/terminal/basics#_accessibility) | VS Code terminal поддерживает accessibility и navigation commands. | Command blocks должны иметь keyboard/screen-reader model. |
| [pty(7)](https://man7.org/linux/man-pages/man7/pty.7.html) | Unix pseudoterminal - bidirectional channel between terminal emulator and process. | PTY byte boundary должен быть явным capture layer. |
| [termios(3)](https://man7.org/linux/man-pages/man3/termios.3.html) | `ECHO`, `ICANON`, input/output modes управляют terminal line discipline. | ECHO-off должен влиять на raw input redaction. |
| [pam_tty_audit](https://man7.org/linux/man-pages/man8/pam_tty_audit.8.html) | Linux PAM module может audit-ить TTY keystrokes, включая опцию password logging. | Keystroke logging sensitive и должен быть opt-in/strict-policy only. |
| [Microsoft Pseudoconsole](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session) | ConPTY создает pseudoconsole session, input/output pipes и resize API. | Windows capture/replay должен учитывать ConPTY boundary and resize. |
| [CreatePseudoConsole](https://learn.microsoft.com/en-us/windows/console/createpseudoconsole) | Windows API создает HPCON с initial terminal size. | Rows/cols являются частью process/terminal contract с момента запуска. |
| [ResizePseudoConsole](https://learn.microsoft.com/en-us/windows/console/resizepseudoconsole) | ConPTY имеет отдельный resize API. | Resize должен быть durable event, не UI-only change. |
| [Mosh](https://mosh.org/) | Mosh uses state synchronization for roaming remote terminal sessions. | Snapshots/state sync complement raw journal replay. |
| [Mosh paper](https://mosh.org/mosh-paper.pdf) | Mosh paper описывает SSP и prediction/state synchronization. | Для fast restore полезна convergence model, не только byte replay. |
| [Eternal Terminal](https://github.com/MisterTea/EternalTerminal) | ET поддерживает reconnect к remote shell после network interruption. | Reconnect continuity не равна persisted transcript/history. |
| [dtach](https://dtach.sourceforge.net/) | dtach дает detach/reattach без full terminal multiplexer features. | Live process attach можно иметь без screen/history persistence. |
| [iTerm2 tmux integration](https://iterm2.com/documentation-tmux-integration.html) | iTerm2 использует tmux integration/control mode, показывая tmux windows как native UI. | Для mux backend лучше structured integration, чем outer raw stream parsing. |
| [tmux control mode](https://github.com/tmux/tmux/wiki/Control-Mode) | tmux control mode дает machine-readable protocol для clients. | Structured mux API может стать source for topology/output events. |
| [tmux FAQ passthrough](https://github.com/tmux/tmux/wiki/FAQ#how-do-i-use-rgb-colour) | tmux terminal feature/passthrough behavior требует специальных настроек. | OSC/shell integration passthrough внутри tmux надо считать capability, не guarantee. |
| [WezTerm current working directory](https://wezterm.org/config/lua/pane/get_current_working_dir.html) | WezTerm получает cwd через shell integration/OSC 7. | CWD events имеют host/path/trust и privacy implications. |
| [Zellij features](https://zellij.dev/features/) | Zellij exposes session, remote, read-only and pane-related features. | Для zellij надо моделировать shared/read-only clients and structured pane state. |
| [Zellij CLI actions](https://zellij.dev/documentation/cli-actions.html) | Zellij CLI actions включают list panes/tabs and query current state. | Zellij backend может опираться на structured commands instead of screen scraping. |
| [Zellij pipe](https://zellij.dev/documentation/plugin-pipes.html) | Zellij plugin pipes позволяют передавать messages/plugins data. | Mux integration может стать data plane for metadata, но требует trust boundaries. |
| [Azure Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing) | Event sourcing хранит append-only events; materialized views/projections строятся отдельно. | Raw journal/events должны быть source of truth, blocks/search/snapshots - projections. |
| [Martin Fowler Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) | Event log позволяет rebuild state and temporal queries. | Terminal history можно пересобрать после parser/schema upgrade. |
| [AWS CloudTrail integrity validation](https://docs.aws.amazon.com/awscloudtrail/latest/userguide/cloudtrail-log-file-validation-intro.html) | CloudTrail использует digest files для проверки log integrity. | Для strict history нужен hash chain/signed checkpoints. |
| [Sigstore Rekor](https://docs.sigstore.dev/logging/overview/) | Rekor transparency log gives tamper-resistant append-only records. | Audit-mode history can learn from transparency log patterns. |
| [NIST SP 800-92](https://csrc.nist.gov/pubs/sp/800/92/final) | NIST описывает log management lifecycle: generation, storage, analysis, disposal. | Retention/deletion/integrity - core architecture, not later UI. |
| [OWASP Log Injection](https://owasp.org/www-community/attacks/Log_Injection) | Log data can be injected/manipulated by attacker-controlled content. | Terminal transcript viewer must sanitize untrusted output. |
| [CWE-150](https://cwe.mitre.org/data/definitions/150.html) | Improper neutralization of escape/control sequences in logs. | ANSI/OSC/control chars in history need safe rendering and export. |
| [Unicode Security Considerations UTR36](https://www.unicode.org/reports/tr36/) | Unicode text can be visually deceptive and security-sensitive. | Transcript display/search should handle bidi/confusables carefully. |
| [Unicode Security Mechanisms UTS39](https://www.unicode.org/reports/tr39/) | Confusables and identifier spoofing mechanisms. | Command/output search and UI should flag suspicious Unicode when relevant. |
| [IPython SQLite history](https://ipython.readthedocs.io/en/stable/interactive/reference.html#input-caching-system) | IPython keeps input/output history and uses a history database. | REPLs have their own command domain; shell blocks cannot explain inner commands. |
| [Node.js REPL history](https://nodejs.org/api/repl.html#persistent-history) | Node REPL supports persistent history. | App/REPL history is separate from terminal transcript and output journal. |
| [psql history](https://www.postgresql.org/docs/current/app-psql.html#APP-PSQL-VARIABLES) | `psql` supports history file behavior through variables. | Database REPL commands need nested domain handling if we want command-level fidelity. |
| [Python readline history](https://docs.python.org/3/library/readline.html#history-file) | Python readline can read/write command history files. | Shell history and REPL history are independent sources, not terminal truth. |
| [OpenTelemetry Logs Data Model](https://opentelemetry.io/docs/specs/otel/logs/data-model/) | Logs have timestamps, observed timestamps, body, severity, attributes and trace correlation. | Terminal events should use structured attributes and correlation IDs. |
| [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/) | Semantic conventions standardize resource/event attributes. | Terminal journal should use stable attribute names for backend/session/process/pane. |
| [Windows Event 4688](https://learn.microsoft.com/en-us/windows/security/threat-protection/auditing/event-4688) | Windows process creation audit can include command line. | Process audit can enrich command blocks but misses shell built-ins and REPL commands. |
| [Sysmon Event ID 1](https://learn.microsoft.com/en-us/sysinternals/downloads/sysmon) | Sysmon logs process creation with hashes and command line. | Optional process correlation, not required baseline feature. |
| [Linux audit execve](https://man7.org/linux/man-pages/man7/audit.rules.7.html) | Linux audit can match syscall events including execve. | OS-level exec tracing is privileged and not a replacement for terminal transcript. |
| [SQLite internal vs external BLOBs](https://www.sqlite.org/intern-v-extern-blob.html) | SQLite compares storing BLOBs internally vs external files. | Large stream/media artifacts need explicit DB vs external storage policy. |
| [SQLite Incremental BLOB I/O](https://www.sqlite.org/c3ref/blob_open.html) | SQLite supports incremental BLOB access. | Large segment/artifact reading should avoid loading whole blob where possible. |
| [SQLite How To Corrupt](https://www.sqlite.org/howtocorrupt.html) | SQLite documents corruption causes: broken locking, file operations while open, backup mistakes, network FS. | History storage needs operational guardrails, backup API and corruption tests. |
| [SQLite Recovery](https://www.sqlite.org/recovery.html) | SQLite has recovery guidance for salvaging data from corrupt databases. | History DB should support partial recovery/quarantine/rebuild. |
| [SQLite Atomic Commit](https://www.sqlite.org/atomiccommit.html) | SQLite explains atomic commit assumptions and filesystem behavior. | Reliability depends on filesystem semantics, not just SQL correctness. |
| [OWASP Logging Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html) | OWASP lists data to exclude from logs and log protection practices. | Terminal history must exclude/redact secrets and protect logs from tampering. |
| [OWASP Secrets Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html) | Secrets lifecycle includes storage, rotation and exposure handling. | Redaction/deletion policies must treat leaked terminal secrets as lifecycle events. |
| [GitHub Secret Scanning](https://docs.github.com/en/code-security/secret-scanning/introduction/about-secret-scanning) | Secret scanning uses provider patterns and validity checks. | Terminal history redaction should be rule/profile/version based and imperfect by design. |
| [GitHub Push Protection](https://docs.github.com/en/code-security/secret-scanning/protecting-pushes-with-secret-scanning) | Push protection blocks known secrets before they are committed. | Future feature: warn/block exporting committed terminal transcripts with detected secrets. |
| [SQLite PRAGMA](https://www.sqlite.org/pragma.html) | SQLite PRAGMA controls journal mode, synchronous, busy timeout, foreign keys, optimize and more. | History DB must set connection PRAGMAs explicitly and test them. |
| [SQLite Transactions](https://www.sqlite.org/lang_transaction.html) | SQLite documents DEFERRED/IMMEDIATE/EXCLUSIVE transactions and lock behavior. | Batch writer should use explicit transaction strategy. |
| [SQLite Foreign Keys](https://www.sqlite.org/foreignkeys.html) | Foreign key enforcement is disabled by default unless enabled per connection. | Cascading history deletes require `PRAGMA foreign_keys=ON` and tests. |
| [Diesel migrations](https://docs.rs/diesel_migrations/latest/diesel_migrations/) | Diesel supports embedded migrations and migration harness. | New Diesel persistence needs embedded migrations and migration tests. |
| [Diesel getting started migrations](https://diesel.rs/guides/getting-started) | Diesel workflow creates migrations and generates schema. | Diesel schema should be generated/tested, not hand-waved. |
| [Zstandard format](https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md) | Zstd frames are strong compression units, but random access depends on framing strategy. | Compress terminal output per segment or seekable chunk, not one giant stream. |
| [Zstd seekable format](https://github.com/facebook/zstd/blob/dev/contrib/seekable_format/zstd_seekable_compression_format.md) | Seekable zstd format indexes frames for random access. | Future cold-storage compression can use seekable framing for long histories. |
| [LZ4 frame format](https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md) | LZ4 frame format favors fast compression/decompression. | LZ4 is an option for hot segments where speed matters more than ratio. |
| [xterm.js serialize addon](https://github.com/xtermjs/xterm.js/tree/master/addons/addon-serialize) | Serialize addon exports terminal buffer state as escape/text representation. | Use as fast projection snapshot/hydration aid, not as canonical history store. |
| [Windows DPAPI CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata) | DPAPI protects data using Windows user/machine credentials. | Protect local DB encryption keys and sync tokens through OS crypto, not adjacent plain files. |
| [Windows Credential Manager CredWrite](https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritea) | Credential Manager persists credentials through the OS credential vault. | Store encryption/sync/token material as OS-backed secrets. |
| [Windows Known Folder IDs](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid) | Windows defines LocalAppData, RoamingAppData and other app storage locations. | Put DB, config, cache and runtime files in correct Windows locations. |
| [SHGetKnownFolderPath](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shgetknownfolderpath) | Windows API resolves known folders reliably. | Do not hardcode `C:\Users\...\AppData`; use OS folder APIs or vetted wrapper crates. |
| [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/) | XDG separates data, config, state, cache and runtime directories. | Cross-platform storage must separate durable history from cache/runtime artifacts. |
| [directories crate](https://docs.rs/directories/latest/directories/) | Rust crate resolves platform-specific project directories. | Use a path abstraction layer so Windows/Linux/macOS storage policies stay explicit. |
| [OpenTelemetry Metrics Data Model](https://opentelemetry.io/docs/specs/otel/metrics/data-model/) | Metrics model defines sums, gauges, histograms and temporality. | Expose writer lag, queue depth, WAL size, dropped segments and recovery results as metrics. |
| [Google SRE Monitoring Distributed Systems](https://sre.google/sre-book/monitoring-distributed-systems/) | Monitoring should expose symptoms and causes, not just implementation counters. | Terminal history needs user-facing reliability symptoms and low-level causes. |
| [Google SRE Service Level Objectives](https://sre.google/sre-book/service-level-objectives/) | SLOs define reliability targets from the user's point of view. | Define history durability/completeness targets before claiming "stable". |
| [MDN WebSocket bufferedAmount](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/bufferedAmount) | WebSocket exposes bytes queued but not yet transmitted. | Browser/gateway path must observe buffered bytes and apply backpressure policy. |
| [MDN WebSocketStream](https://developer.mozilla.org/en-US/docs/Web/API/WebSocketStream) | WebSocketStream integrates WebSockets with Streams API backpressure. | Useful pattern for future transport abstraction; baseline should still support regular WebSocket. |
| [Tokio mpsc](https://docs.rs/tokio/latest/tokio/sync/mpsc/) | Tokio bounded channels provide async backpressure. | The history writer should use bounded queues and explicit overflow policy. |
| [cargo-fuzz book](https://rust-fuzz.github.io/book/cargo-fuzz.html) | cargo-fuzz integrates libFuzzer for Rust targets. | Fuzz terminal parser, replay, redaction and import/export paths. |
| [proptest](https://github.com/proptest-rs/proptest) | Proptest provides property testing for Rust. | Encode journal invariants: ordered seq, idempotent replay, no orphan blocks, policy-safe redaction. |
| [asciicast v3](https://docs.asciinema.org/manual/asciicast/v3/) | v3 is an NDJSON stream with header, output/input/marker/resize/exit events and env metadata. | Export/import can use asciicast-like stream, but internal DB journal should remain richer. |
| [CloudEvents specification](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md) | CloudEvents defines event id/source/type/time/schema fields. | Terminal journal events need a stable envelope and schema version, not ad-hoc JSON. |
| [W3C Trace Context](https://www.w3.org/TR/trace-context/) | Trace Context standardizes cross-system trace identifiers. | Correlate UI submit, shell markers, PTY segments, process events, exports and sync records. |
| [SQLite Security](https://www.sqlite.org/security.html) | SQLite documents defensive settings, trusted schema concerns and attacker-controlled database guidance. | Imported/history DB handling needs defensive config and resource limits. |
| [SQLite DBCONFIG_DEFENSIVE](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html) | SQLite exposes defensive mode and trusted schema configuration. | Store initialization should harden connections before using untrusted/imported content. |
| [SQLite STRICT tables](https://www.sqlite.org/stricttables.html) | STRICT tables enforce rigid type rules in SQLite. | New Diesel tables should prefer strict typing where SQLite version allows it. |
| [SQLite Limits](https://www.sqlite.org/limits.html) | SQLite supports runtime limits for string/blob size, SQL length, columns and more. | Cap imported transcripts, search/index payloads and huge segments to avoid local DoS. |
| [SQLite Session Extension](https://www.sqlite.org/sessionintro.html) | Session extension records changesets/patchsets and applies them with conflict handling. | Future sync can use DB-level change streams, separate from terminal event semantics. |
| [Litestream How it Works](https://litestream.io/how-it-works/) | Litestream replicates SQLite WAL pages to object storage for point-in-time recovery. | Backup/PITR pattern is useful, but not a replacement for app-level session restore. |
| [libsodium secretstream](https://libsodium.gitbook.io/doc/secret-key_cryptography/secretstream) | secretstream provides chunked authenticated encryption for streams. | External artifacts can be encrypted per stream while detecting tampering/truncation. |
| [Unicode UAX #29](https://unicode.org/reports/tr29/) | Unicode text segmentation defines grapheme cluster boundaries. | Search/copy/redaction should understand grapheme clusters, not just bytes or scalar values. |
| [JSON Schema 2020-12](https://json-schema.org/draft/2020-12) | JSON Schema defines validation vocabularies for JSON data. | Export/import event schemas should be machine-validated and versioned. |
| [RFC 7464 JSON Text Sequences](https://www.rfc-editor.org/rfc/rfc7464.html) | RFC 7464 defines a streaming JSON text sequence format. | Useful model for robust event stream export/import with recoverable records. |
| [Windows CommandLineToArgvW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-commandlinetoargvw) | Windows documents command-line to argv parsing rules and caveats. | Stored command text is not safely equivalent to parsed argv on Windows. |
| [PowerShell about_Parsing](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_parsing) | PowerShell has expression mode, argument mode and native argument passing rules. | Rerun must preserve PowerShell parsing context, not normalize to generic shell text. |
| [Set-PSReadLineOption](https://learn.microsoft.com/en-us/powershell/module/PSReadline/set-psreadlineoption) | PSReadLine exposes history save style, duplicate handling and sensitive data behavior. | Respect Windows user history policy and capture quality around PSReadLine. |
| [Diesel SqliteConnection](https://docs.rs/diesel/latest/diesel/sqlite/struct.SqliteConnection.html) | Diesel exposes SQLite connection API used by typed persistence code. | Diesel layer still needs explicit connection initialization and PRAGMA/hardening strategy. |
| [SQLite secure_delete](https://www.sqlite.org/pragma.html#pragma_secure_delete) | `secure_delete` overwrites deleted content in ordinary SQLite tables when enabled, with caveats. | Sensitive purge must define secure-delete profile and limitations around WAL/temp/FTS. |
| [SQLite VACUUM INTO](https://www.sqlite.org/lang_vacuum.html#vacuuminto) | `VACUUM INTO` can create a compact backup database file. | Sanitized export/backup can use compacted copies, but must not bypass redaction policy. |
| [SQLite Temporary Files](https://www.sqlite.org/tempfiles.html) | SQLite documents temp files for journals, materialized views, transient indices and more. | Temp/cache files can contain history data and need location/cleanup policy. |
| [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/) | SQLCipher exposes keying, rekeying, migration and cipher PRAGMAs. | Encryption-at-rest design must include key lifecycle, migration and runtime PRAGMAs. |
| [Rust keyring crate](https://docs.rs/keyring/latest/keyring/) | `keyring` stores secrets in platform credential stores. | Use OS-backed secret references for DB keys/tokens instead of plain config files. |
| [Rust secrecy crate](https://docs.rs/secrecy/latest/secrecy/) | `secrecy` wraps secret values to avoid accidental exposure. | Persistence code should avoid debug/log exposure of keys, tokens and redaction internals. |
| [Rust zeroize crate](https://docs.rs/zeroize/latest/zeroize/) | `zeroize` provides best-effort memory clearing for sensitive values. | Key material handling should zeroize buffers where practical and document limits. |
| [WCAG Status Messages](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html) | Status changes should be programmatically determinable without moving focus. | History degraded/restored/search-complete states should be announced accessibly. |
| [WCAG No Keyboard Trap](https://www.w3.org/WAI/WCAG22/Understanding/no-keyboard-trap.html) | Keyboard users must be able to move focus away from a component. | Web terminal focus mode needs reliable escape/navigation keys. |
| [MDN ARIA live regions](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Guides/Live_regions) | Live regions announce dynamic content changes to assistive tech. | Terminal history/status updates need controlled announcements, not raw noisy output. |
| [WAI-ARIA log role](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Roles/log_role) | `log` role represents sequential, meaningful additions. | Command history timeline can expose semantic blocks as a log/feed. |
| [xterm.js VT features](https://xtermjs.org/docs/api/vtfeatures/) | xterm.js tracks support for VT features including graphics-related capabilities. | Capability profiles should record renderer support and unsupported replay features. |
| [iTerm2 proprietary escape codes](https://iterm2.com/documentation-escape-codes.html) | iTerm2 documents OSC 1337 features like file transfer, images and annotations. | Terminal output can embed active/binary artifacts requiring safe handling. |
| [Prometheus histograms](https://prometheus.io/docs/practices/histograms/) | Histograms measure distributions such as request durations and sizes. | Writer/replay/checkpoint/redaction latency metrics need useful buckets. |
| [ICO storage limitation](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/data-protection-principles/a-guide-to-the-data-protection-principles/storage-limitation/) | Storage limitation requires not keeping personal data longer than needed. | Terminal history retention needs explicit policy, review and deletion behavior. |
| [NIST Privacy Framework](https://www.nist.gov/privacy-framework) | Privacy Framework helps identify, govern, control, communicate and protect privacy risks. | Terminal history privacy should be designed as lifecycle controls, not only redaction. |
| [Tantivy docs.rs](https://docs.rs/tantivy/latest/tantivy/) | Tantivy is a Rust full-text search engine library; current docs.rs release is 0.25.x. | Future large/cold/global transcript search can use Tantivy as a derived index, not source of truth. |
| [SQLite FTS5 contentless-delete tables](https://www.sqlite.org/fts5.html#contentless_delete_tables) | FTS5 supports contentless-delete tables for index-only storage with delete support. | Search index can avoid duplicating raw content while still supporting deletion/update semantics. |
| [SQLite FTS5 trigram tokenizer](https://www.sqlite.org/fts5.html#the_trigram_tokenizer) | Trigram tokenizer supports substring search. | Useful for command/output fragments, paths and tokens where word tokenization fails. |
| [Transactional Outbox pattern](https://microservices.io/patterns/data/transactional-outbox.html) | Outbox stores messages in the same transaction as state changes. | Projection/rebuild/export/sync jobs should be committed durably with state changes. |
| [Apalis SQLite](https://docs.rs/apalis-sqlite/latest/apalis_sqlite/) | Rust SQLite-backed job storage for Apalis, with worker/job queue semantics. | Good reference pattern for durable jobs, but RC dependency risk argues for owned schema first. |
| [SQLite application_id](https://www.sqlite.org/pragma.html#pragma_application_id) | `application_id` identifies database files as belonging to an application format. | History DB should identify file type before recovery/import/migration. |
| [SQLite user_version](https://www.sqlite.org/pragma.html#pragma_user_version) | `user_version` stores app-defined schema/version integer. | Pair with Diesel migrations as a simple compatibility guard. |
| [Automerge Rust docs.rs](https://docs.rs/automerge/latest/automerge/) | Automerge is a CRDT library for concurrent document edits. | Useful for future collaborative metadata, not raw terminal transcript truth. |
| [Yjs documentation](https://docs.yjs.dev/) | Yjs provides shared data types and CRDT updates. | Good pattern for collaborative session notes/layout metadata, not privacy deletion semantics. |
| [cr-sqlite](https://github.com/vlcn-io/cr-sqlite) | cr-sqlite adds CRDT-style sync to SQLite tables. | Interesting future sync research, but terminal history deletion/redaction needs explicit policy first. |
| [W3C High Resolution Time](https://www.w3.org/TR/hr-time-3/) | Defines monotonic high-resolution time for measuring durations. | Store durations/replay timing with monotonic deltas, not only wall-clock timestamps. |
| [Rust Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) | `Instant` is monotonic where possible and opaque across processes. | Persist elapsed durations/deltas, not raw `Instant` values. |
| [OWASP Top 10 for LLM Applications](https://owasp.org/www-project-top-10-for-large-language-model-applications/) | OWASP covers prompt injection and sensitive information disclosure risks. | AI context from terminal history must be redacted, bounded and provenance-aware. |
| [NIST AI Risk Management Framework](https://www.nist.gov/itl/ai-risk-management-framework) | NIST AI RMF frames AI risk governance, measurement and management. | AI history integration needs risk controls and auditability. |
| [Model Context Protocol specification](https://modelcontextprotocol.io/specification) | MCP standardizes exposing tools/resources/prompts to model applications. | Terminal history exported to AI should use explicit resources with provenance and policy, not raw paste. |
| [Wave Durable Sessions](https://docs.waveterm.dev/durable-sessions) | Wave persists terminal history across reconnects/reboots and organizes work around terminal blocks. | Durable block/session history is a validated product direction. |
| [WindTerm Restore Sessions](https://kingtoolbox.github.io/2020/01/22/restore-sessions/) | WindTerm describes session/tab/window restoration behavior. | Users expect layout/session restore, but process/history guarantees must be explicit. |
| [Termius Documentation](https://www.termius.com/documentation) | Termius documents cross-device SSH client workflows, vault/sync and terminal usage. | Sync/product UX expectations exist, but terminal history sync needs separate privacy/deletion semantics. |
| [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) | Job Objects group processes and apply limits/management operations. | Native Windows backend should use process-tree lifecycle semantics, not only child PID tracking. |
| [AssignProcessToJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject) | Assigns a process to a Windows job object. | Windows native sessions need deterministic cleanup groups. |
| [TerminateJobObject](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-terminatejobobject) | Terminates all processes associated with a job. | Emergency cleanup should be explicit and recorded, not silent orphan killing. |
| [GenerateConsoleCtrlEvent](https://learn.microsoft.com/en-us/windows/console/generateconsolectrlevent) | Sends CTRL+C/CTRL+BREAK signals to console process groups. | Graceful cancellation differs from job termination and should be a journal event. |
| [ClosePseudoConsole](https://learn.microsoft.com/en-us/windows/console/closepseudoconsole) | Closes a Windows pseudoconsole object. | ConPTY close semantics should be part of session shutdown/recovery tests. |
| [SQLite Isolation](https://www.sqlite.org/isolation.html) | SQLite documents isolation and snapshot behavior, especially WAL readers. | Restore/export reads should understand snapshot isolation and writer interaction. |
| [SQLite Snapshot API](https://www.sqlite.org/c3ref/snapshot.html) | SQLite exposes snapshot APIs for WAL-mode database states. | Advanced consistent export/replay can record snapshot boundaries where available. |
| [Windows Volume Shadow Copy Service](https://learn.microsoft.com/en-us/windows/win32/vss/about-the-volume-shadow-copy-service) | VSS coordinates point-in-time volume snapshots for backup. | OS-level backup may help disaster recovery, but app-level DB/export policy remains necessary. |
| [notify crate](https://docs.rs/notify/latest/notify/) | Rust cross-platform filesystem notification crate. | Artifact stores can watch for external changes, but watcher events are only hints. |
| [ReadDirectoryChangesW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesw) | Windows API monitors directory changes. | Windows artifact integrity should not rely solely on watcher events. |
| [inotify](https://man7.org/linux/man-pages/man7/inotify.7.html) | Linux inotify monitors filesystem events with queue/overflow semantics. | Watcher overflow must trigger full rescan. |
| [OpenTelemetry events semantic conventions](https://opentelemetry.io/docs/specs/semconv/general/events/) | OTel documents event names and event attribute conventions. | Terminal journal event naming should be stable and documented. |
| [RFC 9162 Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc9162) | Certificate Transparency defines append-only Merkle tree logs and consistency proofs. | Strict/audit mode can borrow checkpoint/consistency proof patterns. |
| [BLAKE3 Rust crate](https://docs.rs/blake3/latest/blake3/) | BLAKE3 is a fast cryptographic hash function with Rust implementation. | Content-addressed artifact IDs/checksums can use strong fast hashes. |
| [IPFS Content Addressing](https://docs.ipfs.tech/concepts/content-addressing/) | IPFS explains content identifiers based on content hashes. | Useful conceptual model for immutable artifacts, with deletion/privacy caveats. |
| [CRIU project](https://github.com/checkpoint-restore/criu) | CRIU checkpoints and restores Linux process state in supported contexts. | Process-state restore is a separate advanced backend, not the native baseline. |
| [tmux-continuum](https://github.com/tmux-plugins/tmux-continuum) | tmux-continuum continuously saves tmux environment and auto-restores. | Mux-backed persistence is a different path from native process restore. |
| [Terminal Wrench](https://arxiv.org/abs/2604.17596) | Recent benchmark work studies reward-hackable terminal environments for AI agents. | Treat terminal history/output as untrusted AI context with provenance and guardrails. |
| [RFC 6455 WebSocket](https://www.rfc-editor.org/rfc/rfc6455) | WebSocket defines framing, ping/pong, close and protocol behavior. | It is not enough for terminal history delivery; application seq/ack/replay is required. |
| [Socket.IO Delivery Guarantees](https://socket.io/docs/v4/delivery-guarantees/) | Socket.IO documents message ordering, at-most-once default and app-level at-least-once patterns. | Terminal stream delivery needs explicit offsets/acks and replay from persistence. |
| [Socket.IO Connection State Recovery](https://socket.io/docs/v4/connection-state-recovery) | Stores session id, rooms and missed packets for reconnect recovery within a duration. | Browser terminal reconnect should recover missed pane events or mark unrecoverable gap. |
| [Socket.IO Offline Behavior](https://socket.io/docs/v4/client-offline-behavior/) | Client buffers events while disconnected and can spike on reconnect. | UI input/actions queued offline need caps, confirmation and stale-drop policy. |
| [VS Code Remote SSH](https://code.visualstudio.com/docs/remote/ssh) | VS Code runs a remote server over SSH and changes where terminals/processes run. | Remote terminal history needs execution-domain metadata and remote privacy policy. |
| [OpenSSH ssh_config](https://man.openbsd.org/ssh_config) | `ssh_config` defines ControlMaster, ControlPath, forwarding and many connection options. | SSH alias/mux/forwarding details affect provenance and can be sensitive. |
| [WezTerm SSH domains](https://wezterm.org/config/lua/SshDomain.html) | WezTerm models SSH hosts as explicit domains. | Execution domain should be first-class, not inferred from cwd text. |
| [WSL basic commands](https://learn.microsoft.com/en-us/windows/wsl/basic-commands) | WSL CLI can run distributions, commands and manage Linux environments from Windows. | WSL sessions require backend/domain metadata distinct from native Windows cmd/powershell. |
| [WSL interoperability](https://learn.microsoft.com/en-us/windows/wsl/filesystems#interoperability-between-windows-and-linux-commands) | WSL can run Windows tools from Linux and Linux tools from Windows path contexts. | Command provenance must distinguish Win32-from-WSL and WSL-from-Windows. |
| [WSL configuration](https://learn.microsoft.com/en-us/windows/wsl/wsl-config) | WSL config controls interop, automount, PATH behavior and systemd. | History capture should record relevant WSL policy/profile for reproducibility. |
| [OpenFeature specification](https://openfeature.dev/specification/) | OpenFeature standardizes feature flag evaluation concepts. | Persistence v2 rollout should use explicit flags and kill switches. |
| [Martin Fowler Feature Toggles](https://martinfowler.com/articles/feature-toggles.html) | Feature toggles separate release, ops, experiment and permissioning concerns. | History persistence flags should be categorized and cleaned up, not permanent if-statements. |
| [Unleash activation strategies](https://docs.getunleash.io/reference/activation-strategies) | Unleash supports gradual rollout and constrained activation strategies. | Roll out history writer by OS/backend/profile/session cohort. |
| [LaunchDarkly kill switch](https://launchdarkly.com/docs/home/flags/killswitch) | Kill switches disable functionality quickly during incidents. | Persistence writer/export/sync needs a safe emergency off switch. |
| [Toxiproxy](https://github.com/Shopify/toxiproxy) | Toxiproxy simulates network conditions for tests. | Reconnect/missed-output tests need repeatable network failure scenarios. |
| [Linux tc-netem](https://man7.org/linux/man-pages/man8/tc-netem.8.html) | netem emulates delay, loss, duplication and reordering. | Useful for transport chaos fixtures around remote/zellij/SSH/WebSocket paths. |
| [Protocol Buffers best practices](https://protobuf.dev/best-practices/dos-donts/) | Protobuf documents schema evolution practices like reserving deleted fields. | Terminal event payloads need explicit field deprecation/reservation rules. |
| [FlatBuffers evolution](https://flatbuffers.dev/evolution/) | FlatBuffers documents forward/backward-compatible schema evolution rules. | Binary event formats need compatibility rules before being used for durable history. |
| [CBOR RFC 8949](https://www.rfc-editor.org/rfc/rfc8949) | CBOR is a concise binary object representation with deterministic encoding guidance. | Possible future compact event payload format, but schema/version still required. |
| [MessagePack specification](https://github.com/msgpack/msgpack/blob/master/spec.md) | MessagePack defines compact binary serialization types. | Compact storage alone is not enough without schema evolution and validation. |
| [SQLite Partial Indexes](https://www.sqlite.org/partialindex.html) | Partial indexes index only rows matching a WHERE clause. | Useful for active sessions, non-deleted rows, pending jobs and unredacted findings. |
| [SQLite Indexes On Expressions](https://www.sqlite.org/expridx.html) | SQLite can index expressions with restrictions. | Derived searchable/sortable fields can be indexed without duplicating everything. |
| [SQLite Generated Columns](https://www.sqlite.org/gencol.html) | Generated columns compute values from other columns and can be indexed. | Helpful for derived metadata, but must remain compatible with migrations. |
| [SQLite EXPLAIN QUERY PLAN](https://www.sqlite.org/eqp.html) | Shows high-level query plan information. | Query plans for restore/search/prune should be regression-tested. |
| [SQLite Query Planner](https://www.sqlite.org/queryplanner.html) | Documents how SQLite chooses indexes and scans. | Large-history schema must be designed around actual query patterns. |
| [SQLite ANALYZE](https://www.sqlite.org/lang_analyze.html) | ANALYZE gathers statistics used by the query planner. | Maintenance jobs should keep planner stats fresh for large DBs. |
| [OpenTelemetry Baggage](https://opentelemetry.io/docs/specs/otel/baggage/) | Baggage propagates key/value context across process boundaries. | Do not propagate sensitive terminal/session/cwd metadata blindly. |
| [OpenTelemetry handling sensitive data](https://opentelemetry.io/docs/security/handling-sensitive-data/) | OTel docs warn to protect or remove sensitive data in telemetry. | Terminal telemetry/crash reports need redaction gates. |
| [Sentry Data Scrubbing](https://docs.sentry.io/security-legal-pii/scrubbing/) | Sentry supports scrubbing sensitive data before storage. | Crash/error reporting should scrub command/output/path tokens before upload. |
| [Crashpad Overview](https://chromium.googlesource.com/crashpad/crashpad/+/main/doc/overview_design.md) | Crashpad collects crash reports out-of-process. | Crash diagnostics can include sensitive process state and need policy. |
| [Windows optional diagnostic data](https://learn.microsoft.com/en-us/windows/privacy/optional-diagnostic-data) | Microsoft documents optional diagnostic data, including error codes, app/process reporting and device activity examples. | Windows crash/error diagnostics should be considered terminal-history data surfaces. |
| [Microsoft Purview eDiscovery holds](https://learn.microsoft.com/en-us/purview/ediscovery-create-a-litigation-hold) | eDiscovery holds preserve content for legal investigation. | Enterprise/audit mode needs hold states that override normal deletion. |
| [Microsoft Purview retention labels](https://learn.microsoft.com/en-us/purview/retention) | Retention labels/policies manage retention and deletion lifecycle. | Terminal history policy can borrow hold/retention/expiration separation. |
| [SEC electronic recordkeeping amendments](https://www.govinfo.gov/content/pkg/FR-2022-11-03/pdf/2022-22670.pdf) | SEC electronic recordkeeping guidance covers preservation and audit-trail requirements. | Strict/audit mode may need immutable retention patterns distinct from developer privacy mode. |
| [MDN Storage quotas](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria) | Browser storage has quotas and eviction behavior. | Browser-side caches/snapshots cannot be treated as durable truth. |
| [MDN StorageManager estimate](https://developer.mozilla.org/en-US/docs/Web/API/StorageManager/estimate) | Web apps can estimate storage usage/quota. | Frontend caches should report quota pressure and avoid silent cache loss. |
| [Windows Storage Sense](https://learn.microsoft.com/en-us/windows/configuration/storage/storage-sense) | Windows can clean temporary/local content based on policy. | App cache/temp/export paths must tolerate OS cleanup and report missing artifacts. |
| [Chrome Page Lifecycle API](https://developer.chrome.com/docs/web-platform/page-lifecycle-api) | Page lifecycle includes active/passive/hidden/frozen/discarded states. | Browser terminal clients must reconnect/replay, not rely on unload finalization. |
| [MDN Page Visibility API](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API) | Visibility changes are observable when tab becomes hidden/visible. | Hidden tab should reduce UI work but keep delivery/replay state explicit. |
| [MDN beforeunload](https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeunload_event) | `beforeunload` is limited, unreliable for mobile/app kills and affects bfcache. | Do not depend on beforeunload to persist terminal state. |
| [MDN BroadcastChannel](https://developer.mozilla.org/en-US/docs/Web/API/BroadcastChannel) | BroadcastChannel lets same-origin contexts communicate. | Useful for multi-tab coordination, but not canonical delivery or persistence. |
| [MDN Web Locks API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Locks_API) | Web Locks coordinate work across tabs/workers. | Browser input-owner election can use locks, while DB writer ownership stays server-side. |
| [MDN Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API) | Clipboard API requires secure context and is permission/user-activation constrained. | Live copy/paste and OSC52 handling need explicit user/policy gates. |
| [MDN transient activation](https://developer.mozilla.org/en-US/docs/Web/Security/User_activation) | Some APIs require transient user activation. | Historical replay cannot call privileged clipboard/file APIs automatically. |
| [Docker exec](https://docs.docker.com/reference/cli/docker/container/exec/) | `docker exec` runs a command in a running container, with TTY/env/workdir options. | Container command blocks need container/domain metadata. |
| [Docker attach](https://docs.docker.com/reference/cli/docker/container/attach/) | `docker attach` attaches local streams to a running container. | Attach is not the same as exec/logs and can affect the main process. |
| [Docker logs](https://docs.docker.com/reference/cli/docker/container/logs/) | `docker logs` retrieves container log output from logging driver. | Container logs are useful import/source, but not full interactive terminal transcript. |
| [Kubernetes kubectl exec](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_exec/) | `kubectl exec` executes commands in containers, optionally with stdin/TTY. | Kubernetes exec sessions need pod/container/namespace domain metadata. |
| [Kubernetes kubectl logs](https://kubernetes.io/docs/reference/kubectl/generated/kubectl_logs/) | `kubectl logs` prints pod/container logs. | Logs and exec transcripts are different read models. |
| [Windows Console Code Pages](https://learn.microsoft.com/en-us/windows/console/console-code-pages) | Windows console uses input/output code pages affecting character interpretation. | Windows replay metadata should include codepage/encoding profile. |
| [SetConsoleOutputCP](https://learn.microsoft.com/en-us/windows/console/setconsoleoutputcp) | Sets console output code page. | Codepage changes are terminal/session events for correct decode/replay. |
| [PowerShell about_Character_Encoding](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding) | PowerShell encoding behavior differs across versions and cmdlets. | PowerShell command/output history needs encoding-aware capture and export. |
| [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/) | Rust library for web-compatible character encodings. | Non-UTF-8 decode paths should use tested encoding library, not ad-hoc conversion. |
| [WM_POWERBROADCAST](https://learn.microsoft.com/en-us/windows/win32/power/wm-powerbroadcast) | Windows broadcasts power-management events such as suspend/resume. | Writer/transport should record suspend/resume and flush where possible. |
| [SetThreadExecutionState](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadexecutionstate) | Informs system the app is in use to prevent sleep/display powerdown. | Long strict-history/export jobs may need explicit power policy or user warning. |
| [systemd-inhibit](https://man7.org/linux/man-pages/man1/systemd-inhibit.1.html) | systemd inhibitors can delay/block sleep/shutdown for critical operations. | Cross-platform power events should be modeled as durability boundaries. |
| [OWASP WebSocket Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html) | OWASP recommends Origin validation, authentication, authorization, input validation and logging for WebSockets. | Terminal gateway WebSocket needs explicit handshake and message-level security. |
| [RFC 6454 Origin](https://www.rfc-editor.org/rfc/rfc6454) | Defines the web Origin concept and Origin header model. | Local gateway should validate origin, not only token presence. |
| [OWASP CSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html) | Covers Origin/Referer checks, Fetch Metadata and CSRF token patterns. | Local control endpoints need CSRF/Fetch Metadata defenses where HTTP is used. |
| [MDN Sec-Fetch-Site](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Sec-Fetch-Site) | Fetch Metadata header identifies relation between request initiator and target origin. | Optional defense-in-depth for HTTP control endpoints around local gateway. |
| [Chrome Private Network Access](https://developer.chrome.com/blog/private-network-access-preflight) | Chrome describes preflights for public websites accessing private/local networks. | Localhost/private-network access should be explicit, tokenized and origin-aware. |
| [WICG Private Network Access](https://wicg.github.io/private-network-access/) | Spec draft for restricting requests from less-private to more-private networks. | Future browser behavior can affect local terminal gateways. |
| [Microsoft Named Pipe Security](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights) | Windows named pipes have security descriptors and access rights. | Named-pipe transport still needs ACL/client identity policy. |
| [Microsoft Pipe Names](https://learn.microsoft.com/en-us/windows/win32/ipc/pipe-names) | Documents named pipe path syntax, local/remote names and naming rules. | Pipe naming should be per-user/app/session and not predictable global control. |
| [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html) | Covers file validation, storage, size limits and archive risks. | History import/export bundles need validation, size limits and quarantine. |
| [OWASP Path Traversal](https://owasp.org/www-community/attacks/Path_Traversal) | Path traversal uses `../` and absolute paths to escape intended directories. | Archive extraction must reject traversal and absolute paths. |
| [Python zipfile extractall warning](https://docs.python.org/3/library/zipfile.html#zipfile.ZipFile.extractall) | Python docs warn callers must validate paths to prevent escaping destination. | Archive extraction safety should be explicit even when using library helpers. |
| [Electron Security](https://www.electronjs.org/docs/latest/tutorial/security) | Electron security checklist covers untrusted content, navigation, CSP and dangerous APIs. | Desktop webview shell must keep terminal UI least-privilege. |
| [Tauri Security](https://v2.tauri.app/security/) | Tauri documents security boundaries, IPC and permissions. | Native shell/webview commands must be allowlisted and audited. |
| [MDN Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP) | CSP controls script, object, framing and resource loading behavior. | HTML transcript/export viewer needs strict CSP and no active scripts by default. |
| [MDN iframe sandbox](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/iframe#sandbox) | `sandbox` restricts iframe capabilities. | Rendered transcript previews should use sandboxed contexts. |
| [MDN Clear-Site-Data](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Clear-Site-Data) | Header clears cookies, storage and cache for a site. | Browser-side cached history/projections need clear policy after redaction/logout. |
| [Rust regex crate](https://docs.rs/regex/latest/regex/) | Rust regex avoids unbounded backtracking and documents worst-case performance. | Redaction/search rules should prefer linear-time engines over backtracking regex. |
| [OWASP ReDoS](https://owasp.org/www-community/attacks/Regular_expression_Denial_of_Service_-_ReDoS) | Inefficient regular expressions can take exponential time. | Secret scanning rules need ReDoS review, limits, tests and runtime metrics. |
| [RE2](https://github.com/google/re2) | RE2 is designed for predictable regular expression matching. | Good reference for safe regex policy and rejected pattern classes. |
| [Aho-Corasick Rust crate](https://docs.rs/aho-corasick/latest/aho_corasick/) | Multi-pattern exact matching implementation for Rust. | Use exact marker/prefix matching before expensive redaction rules. |
| [Hyperscan](https://intel.github.io/hyperscan/dev-reference/) | High-performance multiple pattern matching library. | Future high-volume scanner option, but CPU/build/runtime support must be evaluated. |
| [tempfile crate](https://docs.rs/tempfile/latest/tempfile/) | Rust temporary file API with persist operations. | Artifact writes should use same-directory temp+persist patterns carefully. |
| [atomic-write-file crate](https://docs.rs/atomic-write-file/latest/atomic_write_file/) | Rust crate for writing files through atomic replacement. | Useful for manifest/artifact writes, with explicit durability caveats. |
| [Windows ReplaceFile](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew) | Windows API replaces one file with another and can create backup. | Windows artifact replacement needs platform-specific tests. |
| [Windows MoveFileEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) | Windows move API supports replace and write-through flags. | Atomic replace semantics must be tested on Windows, not assumed from Unix. |
| [Windows FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) | Flushes buffered data for a file or device handle. | Durable external artifact writes need explicit flush decisions. |
| [Windows LockFileEx](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex) | Locks byte ranges in a file. | File locks are only coordination hints, not the history source of truth. |
| [Linux flock](https://man7.org/linux/man-pages/man2/flock.2.html) | Documents advisory file lock behavior and edge cases. | Cross-platform lock behavior differs and needs dedicated tests. |
| [OWASP Authorization Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html) | Authorization guidance emphasizes deny-by-default and server-side checks. | Share/export/delete/rerun must go through centralized authorization. |
| [NIST ABAC SP 800-162](https://csrc.nist.gov/pubs/sp/800/162/upd2/final) | ABAC models subject, object, action and environment attributes. | Session history permissions should be object/action/context based. |
| [Open Policy Agent](https://www.openpolicyagent.org/docs/latest/) | Policy-as-code engine using Rego. | Good reference for centralized, testable policy decisions. |
| [Cedar Policy](https://docs.cedarpolicy.com/) | Authorization policy language from AWS. | Useful model for auditable policies around history sharing and export. |
| [Biscuit tokens](https://www.biscuitsec.org/docs/) | Capability-style authorization tokens with attenuation. | Session share tokens should be narrow, caveated and revocable. |
| [OAuth 2.0 Token Exchange RFC 8693](https://www.rfc-editor.org/rfc/rfc8693) | Standard for scoped token exchange and delegated access. | Delegated access to history should be audience/resource/action scoped. |
| [Windows Naming Files, Paths, and Namespaces](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) | Documents reserved names, path formats, namespaces, separators and file naming rules. | Export/storage names must be generated and sanitized, not copied from terminal text. |
| [Windows Maximum Path Length Limitation](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation) | Explains MAX_PATH, long path opt-in and `\\?\` extended paths. | Long history/export paths need explicit Windows path strategy and tests. |
| [Windows File Streams](https://learn.microsoft.com/en-us/windows/win32/fileio/file-streams) | NTFS files can have alternate streams addressed through colon syntax. | Artifact/import/export validation must reject or model ADS instead of treating `:` as harmless text. |
| [Windows Reparse Points](https://learn.microsoft.com/en-us/windows/win32/fileio/reparse-points) | Reparse points allow filesystem filters to redirect/open special file system objects. | Artifact store root checks must account for junctions, symlinks and mount points. |
| [Reparse Points and File Operations](https://learn.microsoft.com/en-us/windows/win32/fileio/reparse-points-and-file-operations) | Documents how file operations interact with reparse points. | Path validation before open is not enough for critical writes. |
| [Symbolic Link Programming Considerations](https://learn.microsoft.com/en-us/windows/win32/fileio/symbolic-link-programming-considerations) | Windows symlink behavior depends on flags and target type. | Import/export/store should define symlink policy explicitly. |
| [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew) | Windows file open API controls desired access, sharing, creation disposition and flags. | Replace/delete/readers must be tested with realistic sharing modes. |
| [GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew) | Gets the final path for an opened file handle. | Critical artifact writes should verify final handle path after open. |
| [GetFileInformationByHandleEx](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex) | Retrieves file information classes for an open handle. | Store verifier can use handle-level identity metadata where available. |
| [FILE_ID_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info) | Contains volume serial number and file ID. | File identity can be tracked beyond mutable path strings. |
| [Windows Per-directory Case Sensitivity](https://learn.microsoft.com/en-us/windows/wsl/case-sensitivity) | Windows directories can be case-sensitive, especially with WSL interop. | Artifact names must not rely on global case-insensitive behavior. |
| [Change Journals](https://learn.microsoft.com/en-us/windows/win32/fileio/change-journals) | NTFS change journal records changes to files/directories on a volume. | USN can accelerate verification, but DB manifest still remains truth. |
| [FSCTL_QUERY_USN_JOURNAL](https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_query_usn_journal) | Queries the current state of the USN change journal. | Watcher/verifier should detect journal reset/range gaps. |
| [ReadDirectoryChangesW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesw) | Windows directory watcher API reports file changes. | Watcher events are hints and overflow/gap cases need full rescan. |
| [Rust OsStr](https://doc.rust-lang.org/std/ffi/struct.OsStr.html) | Platform-native borrowed string type for paths and process arguments. | Operational paths should stay as `Path`/`OsStr`, not lossy UTF-8 `String`. |
| [Rust Path](https://doc.rust-lang.org/std/path/struct.Path.html) | Rust path API represents platform path syntax and metadata operations. | Path operations should use structured APIs and treat display separately from identity. |
| [Kafka Message Delivery Semantics](https://kafka.apache.org/documentation/#semantics) | Kafka documents at-most-once, at-least-once and exactly-once semantics with producer/consumer tradeoffs. | Terminal history should define delivery guarantees explicitly instead of assuming WebSocket reliability. |
| [Kafka Design](https://kafka.apache.org/documentation/#design) | Kafka is built around persistent ordered logs, offsets and retention. | Per-pane terminal journal should use explicit stream offsets and retention rules. |
| [Azure Cosmos DB Transactional Outbox](https://learn.microsoft.com/en-us/azure/architecture/databases/guide/transactional-outbox-cosmos) | Microsoft documents an outbox approach for transactional event publication. | Journal writes and projection/export/sync jobs should commit atomically. |
| [AWS idempotent APIs](https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/) | AWS describes client request tokens and retry-safe API design. | Terminal control actions need idempotency keys for retries/reconnect. |
| [Stripe Idempotent Requests](https://docs.stripe.com/api/idempotent_requests) | Stripe stores first result for an idempotency key and replays it for retries. | UI submit/export/share/delete operations should deduplicate retries by scoped key. |
| [Delta Lake ACID transactions](https://delta-io.github.io/delta-rs/how-delta-lake-works/delta-lake-acid-transactions/) | Delta Lake records table changes in a transaction log. | Terminal snapshots/manifests can track high-water event ranges and artifact references. |
| [Apache Iceberg Table Spec](https://iceberg.apache.org/spec/) | Iceberg uses metadata files, snapshots, manifests and data files. | History restore snapshots should be manifest-based, not opaque screen blobs only. |
| [Apache Hudi Timeline](https://hudi.apache.org/docs/timeline/) | Hudi tracks commit timeline instants and file changes. | History maintenance jobs need explicit instants/states for compaction, cleanup and rollback. |
| [Amazon S3 consistency](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html#ConsistencyModel) | S3 documents strong read-after-write consistency for object operations. | Object-store sync still needs version/manifest policy, not filesystem assumptions. |
| [Amazon S3 Object Lock configuration](https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lock-configure.html) | S3 Object Lock supports retention and legal hold configuration. | Enterprise/audit history can use immutable object-store concepts, with deletion caveats. |
| [Google Cloud Storage object versioning](https://cloud.google.com/storage/docs/object-versioning) | GCS can retain noncurrent object versions. | Sync/delete must account for old object versions that still contain transcript data. |
| [Google Cloud Storage retention policies](https://cloud.google.com/storage/docs/bucket-lock) | Bucket Lock can make retention policy permanent for a bucket. | Legal hold/retention should be explicit and not mixed with normal user cleanup. |
| [Azure immutable blob storage](https://learn.microsoft.com/en-us/azure/storage/blobs/immutable-storage-overview) | Azure Blob Storage supports time-based retention and legal holds. | Cloud backup of terminal history needs immutable/hold state in policy model. |
| [restic design](https://restic.readthedocs.io/en/stable/100_references.html) | restic repository format uses encrypted content-addressed packs, indexes and snapshots. | Backups should be chunked, verified and restorable, not a single copied DB file. |
| [Kopia repositories](https://kopia.io/docs/repositories/) | Kopia stores encrypted snapshots in repositories across local and cloud backends. | Useful pattern for external artifact backup/GC and snapshot metadata. |
| [BorgBackup internals](https://borgbackup.readthedocs.io/en/stable/internals.html) | Borg stores chunks, manifests, indexes and archives with deduplication. | Terminal history backup needs reachability and prune logic for chunks/artifacts. |
| [Syncthing conflict handling](https://docs.syncthing.net/users/syncing.html#conflicting-changes) | Syncthing creates conflict files when concurrent changes cannot be reconciled. | Divergent history/sync branches should be visible, not silently merged. |
| [Grafana Loki architecture](https://grafana.com/docs/loki/latest/get-started/architecture/) | Loki separates distributors, ingesters, queriers, chunks and indexes for log storage. | Terminal search should separate ingest, chunks, metadata index and query planning. |
| [Loki labels](https://grafana.com/docs/loki/latest/get-started/labels/) | Loki labels identify log streams and should avoid high-cardinality values. | Terminal index labels must be low-cardinality and not use command text/cwd as labels. |
| [Loki label best practices](https://grafana.com/docs/loki/latest/get-started/labels/bp-labels/) | Loki warns about bounded label values and cardinality. | Use labels for source/fidelity/status, not user-controlled unique strings. |
| [ClickHouse MergeTree](https://clickhouse.com/docs/engines/table-engines/mergetree-family/mergetree) | MergeTree stores data in parts, sorted by primary key with sparse indexes. | Terminal history can use chunk catalogs and sorted seq/time ranges for pruning. |
| [ClickHouse data skipping indexes](https://clickhouse.com/docs/optimize/skipping-indexes) | Skipping indexes help avoid reading granules when predicates exclude them. | Bloom/minmax/token summaries should be prefilters, not source of truth. |
| [ClickHouse TTL](https://clickhouse.com/docs/guides/developer/ttl) | TTL can move/delete data according to time or rules. | Terminal hot/warm/cold retention should be explicit and testable. |
| [Elasticsearch data streams](https://www.elastic.co/docs/manage-data/data-store/data-streams) | Data streams manage append-only time-series data over backing indices. | Terminal transcript search is append-heavy and time-scoped. |
| [Elasticsearch ILM](https://www.elastic.co/docs/manage-data/lifecycle/index-lifecycle-management) | Index lifecycle management rolls data through phases. | History search projections need hot/warm/cold lifecycle and cleanup jobs. |
| [Elasticsearch searchable snapshots](https://www.elastic.co/docs/deploy-manage/tools/snapshot-and-restore/searchable-snapshots) | Searchable snapshots allow searching data stored in snapshots. | Cold terminal history can be slower but searchable from backup-like artifacts. |
| [Apache Lucene core features](https://lucene.apache.org/core/features.html) | Lucene provides inverted indexing, ranking, faceting and near-real-time search. | Rich search should be a derived inverted index, not canonical terminal storage. |
| [Tantivy docs](https://docs.rs/tantivy/latest/tantivy/) | Tantivy is a Rust full-text search engine with segment-based indexing. | Strong Rust option for medium/large derived search indexes. |
| [Quickwit architecture](https://quickwit.io/docs/main-branch/overview/architecture) | Quickwit builds distributed search over immutable splits and object storage. | Useful model for cold/global terminal search over immutable chunks. |
| [Quickwit indexing](https://quickwit.io/docs/overview/concepts/indexing) | Quickwit indexes documents into splits and publishes metadata. | Terminal search shards can be built/published as immutable derived artifacts. |
| [OpenObserve architecture](https://openobserve.ai/docs/architecture/) | OpenObserve stores logs/metrics/traces with object storage oriented architecture. | Confirms object-store based observability patterns for cold history. |
| [Parquet file format](https://parquet.apache.org/docs/file-format/) | Parquet organizes data into row groups, column chunks and pages with metadata. | Command/event analytics exports can use columnar files, not raw transcript blobs. |
| [Parquet Bloom filters](https://parquet.apache.org/docs/file-format/bloomfilter/) | Parquet supports bloom filters for page/row group pruning. | Token/path/command prefilters can reduce cold scans but require verification. |
| [OWASP LLM Prompt Injection Prevention](https://cheatsheetseries.owasp.org/cheatsheets/LLM_Prompt_Injection_Prevention_Cheat_Sheet.html) | OWASP covers direct/indirect prompt injection, remote content and tool manipulation defenses. | Terminal output passed to AI must be treated as untrusted data with clear boundaries. |
| [OWASP Agentic AI Threats and Mitigations](https://genai.owasp.org/resource/agentic-ai-threats-and-mitigations/) | OWASP describes threats around agents, tools, memory and autonomy. | Terminal agents need least privilege, approvals, audit and constrained tools. |
| [OWASP LLM01 Prompt Injection](https://genai.owasp.org/llmrisk/llm01-prompt-injection/) | OWASP classifies prompt injection as a top LLM application risk. | Command output can contain instructions that try to override user/developer intent. |
| [Microsoft Agent safety](https://learn.microsoft.com/en-us/agent-framework/agents/safety) | Microsoft guidance covers safety for agents, tools and human oversight. | AI actions over terminal history should use scoped tools and approval gates. |
| [Microsoft Defend against indirect prompt injection](https://learn.microsoft.com/en-us/security/zero-trust/sfi/defend-indirect-prompt-injection) | Microsoft describes indirect prompt injection and defense patterns. | Terminal transcript data is untrusted external content for the agent. |
| [MSRC indirect prompt injection defenses](https://msrc.microsoft.com/blog/2025/07/how-microsoft-defends-against-indirect-prompt-injection-attacks/) | MSRC describes prompt shields, spotlighting and task tracker concepts. | Context packaging should mark terminal output as data and track tasks separately. |
| [Microsoft MCP indirect injection guidance](https://developer.microsoft.com/blog/protecting-against-indirect-injection-attacks-mcp) | Microsoft discusses indirect injection risk in MCP tools and resources. | Terminal history exposed through MCP must include tool/resource permission policy. |
| [OpenAI Cookbook guardrails](https://cookbook.openai.com/examples/how_to_use_guardrails/) | OpenAI Cookbook discusses prompt injection, guardrail limitations and combining guardrails with other controls. | Same class applies when agents read terminal output before using terminal tools. |
| [NIST AI 600-1 Generative AI Profile](https://www.nist.gov/publications/artificial-intelligence-risk-management-framework-generative-artificial-intelligence) | NIST maps generative AI risks and controls to AI RMF. | Terminal AI context should have governance, measurement and risk controls. |
| [Google Secure AI Framework](https://blog.google/technology/safety-security/introducing-googles-secure-ai-framework/) | Google SAIF frames secure-by-design AI systems. | AI terminal features need secure development, monitoring and control layers. |
| [MITRE ATLAS](https://atlas.mitre.org/) | MITRE ATLAS catalogs adversary tactics and techniques against AI systems. | Use ATLAS-style scenarios for AI terminal red-team fixtures. |
| [PyRIT](https://github.com/Azure/PyRIT) | Microsoft/Azure PyRIT is a framework for generative AI red teaming. | Use red-team automation patterns for prompt-injection regression suites. |
| [garak](https://github.com/NVIDIA/garak) | garak tests LLM applications for vulnerabilities including prompt injection. | Useful inspiration for automated terminal-context attack fixtures. |
| [Giskard prompt injection](https://docs.giskard.ai/start/glossary/security/injection.html) | Giskard documents prompt-injection vulnerabilities and tests. | Prompt injection findings should be risk signals, not absolute truth. |
| [SQLite testing](https://www.sqlite.org/testing.html) | SQLite documents extensive test strategy including crash, I/O error, OOM and fuzz testing. | Terminal persistence needs explicit reliability test families, not only unit tests. |
| [SQLite TH3](https://www.sqlite.org/th3.html) | SQLite TH3 is a test harness with high branch coverage and fault tests. | Release gates should include storage-specific test suites and coverage goals. |
| [FoundationDB testing](https://apple.github.io/foundationdb/testing.html) | FoundationDB uses deterministic simulation and fault injection to test distributed behavior. | Build a deterministic terminal persistence simulator with replayable seeds. |
| [FoundationDB client testing](https://apple.github.io/foundationdb/client-testing.html) | FDB client tests run randomized operations in simulation. | Browser/client reconnect and ack behavior should be seed-replayable. |
| [TigerBeetle architecture](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/ARCHITECTURE.md) | TigerBeetle describes simulation testing and VOPR fault injection concepts. | Use random operation/fault plans for journal/outbox/snapshot invariants. |
| [TigerBeetle safety](https://docs.tigerbeetle.com/concepts/safety/) | TigerBeetle documents safety goals, storage fault tolerance and VOPR testing. | Persistence design should define safety properties before implementation. |
| [Jepsen analyses](https://jepsen.io/analyses) | Jepsen tests distributed systems under faults and publishes consistency analyses. | Sync/backup/conflict behavior should be tested under partition, crash and clock faults. |
| [Jepsen Elle](https://github.com/jepsen-io/elle) | Elle checks transactional histories for anomalies. | Inspired checkers can validate terminal event histories and idempotency behavior. |
| [TLA+ tools](https://lamport.azurewebsites.net/tla/tools.html) | TLA+ tools include TLC model checker for formal specs. | Small protocols like seq/ack/outbox/tombstones can be model-checked. |
| [Apalache](https://apalache.informal.systems/) | Apalache is a symbolic model checker for TLA+. | Useful alternative for bounded model checks of persistence protocols. |
| [Stateright](https://docs.rs/stateright/latest/stateright/) | Rust model checker for distributed systems and concurrent state machines. | Model-check Rust-like state machines for delivery and sync protocols. |
| [Loom](https://github.com/tokio-rs/loom) | Loom explores possible interleavings in concurrent Rust code. | Writer/outbox/lock concurrency should be tested under schedule permutations. |
| [Shuttle](https://docs.rs/shuttle/latest/shuttle/) | Shuttle provides deterministic concurrency testing for Rust async code. | Async ack/replay/worker races should be reproducible by seed. |
| [failpoints crate](https://docs.rs/failpoints/latest/failpoints/) | Rust failpoints inject failures at configured points. | Add failpoints around flush, rename, DB commit, outbox claim and replay. |
| [Chaos Mesh](https://chaos-mesh.org/docs/) | Chaos Mesh injects pod, network, I/O, time and kernel faults in Kubernetes. | Good pattern for structured chaos experiments and fault scopes. |
| [Chaos Mesh IOChaos](https://chaos-mesh.org/docs/simulate-io-chaos-on-kubernetes/) | IOChaos simulates filesystem latency, errors and attribute changes. | Persistence writer tests should include disk latency/errors and partial failures. |
| [Linux fault injection](https://docs.kernel.org/fault-injection/fault-injection.html) | Linux kernel supports fault injection for memory, I/O and other subsystems. | Lower-level fault tests can validate recovery assumptions. |
| [dm-flakey](https://docs.kernel.org/admin-guide/device-mapper/dm-flakey.html) | Device mapper target simulates intermittent I/O failure. | Useful model for power-loss/disk-failure tests around external artifact writes. |
| [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html) | OWASP covers encryption algorithms, key management and storage design. | Encryption-at-rest needs threat model, key management and authenticated encryption. |
| [OWASP Key Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html) | OWASP covers key lifecycle, storage, rotation and separation. | Terminal history needs key hierarchy and lifecycle records. |
| [NIST SP 800-57 Part 1](https://csrc.nist.gov/pubs/sp/800/57/pt1/r5/final) | NIST recommendations for key management and cryptographic key lifecycle. | Key versions, rotation, destruction and usage periods should be explicit. |
| [NIST SP 800-88 Rev. 1](https://csrc.nist.gov/pubs/sp/800/88/r1/final) | NIST media sanitization guidance includes cryptographic erase. | Selective deletion should use cryptographic erasure where keys are granular. |
| [NIST SP 800-38D](https://csrc.nist.gov/pubs/sp/800/38/d/final) | NIST specifies GCM authenticated encryption mode. | If using GCM-style AEAD, nonce uniqueness and associated data are mandatory. |
| [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/) | SQLCipher documents keying, rekeying and cipher PRAGMAs. | DB encryption requires explicit PRAGMA setup, migration and rekey workflow. |
| [SQLCipher design](https://www.zetetic.net/sqlcipher/design/) | SQLCipher describes page encryption, key derivation and HMAC/integrity approach. | Understand what DB encryption covers and what external artifacts still need. |
| [SQLite SEE](https://www.sqlite.org/see/doc/trunk/www/readme.wiki) | SQLite Encryption Extension encrypts database content with SQLite integration. | Alternative DB encryption option, but licensing/build tradeoffs matter. |
| [libsodium secretstream](https://libsodium.gitbook.io/doc/secret-key_cryptography/secretstream) | libsodium secretstream encrypts streams in chunks with authentication. | Good model for encrypted transcript/artifact streams. |
| [HPKE RFC 9180](https://www.rfc-editor.org/rfc/rfc9180) | Hybrid Public Key Encryption standard for encrypting to recipients. | Future shared/exported history can encrypt per recipient instead of one shared secret. |
| [age](https://age-encryption.org/) | age is a simple modern file encryption format and tool. | Useful reference for encrypted export bundles and recipient UX. |
| [Windows DPAPI CryptProtectData](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata) | DPAPI protects data using user or machine credentials. | Windows root key wrapping can use OS-backed user scope. |
| [Apple Keychain Services](https://developer.apple.com/documentation/security/keychain-services) | macOS/iOS Keychain stores secrets and credentials. | Cross-platform key storage needs platform-specific profiles. |
| [Secret Service API](https://specifications.freedesktop.org/secret-service/latest/) | Freedesktop Secret Service defines Linux desktop secret storage API. | Linux key storage may depend on available session/desktop service. |

## Что делают лучшие терминалы

### Warp - Block model

Самая сильная идея Warp - терминал как история Blocks, а не один большой text buffer.

Что важно:

- Block группирует command + output.
- Block можно копировать целиком, копировать только command или output.
- Block можно re-input.
- Command history хранит не только text, но и exit code, directory, duration, last run.
- Session restoration хранит в SQLite windows/tabs/panes и последние Blocks.
- Background output отдельный тип block, потому attribution не всегда возможен.

Практический вывод:

- Наш `command dock history` должен стать не глобальным списком строк, а read model поверх `terminal_command_blocks`.
- UI должен уметь показывать "команда", "результат", "status", "cwd", "время", "повторить", "копировать output".
- Для long output нужна block-level search/filter/collapse.

### VS Code - semantic protocol и честные restore modes

Самое важное в VS Code - `OSC 633`.

Полезные markers:

- `A` - prompt start.
- `B` - prompt end.
- `C` - pre-execution.
- `D` - command finished, optional exit code.
- `E` - exact command line.
- `P Cwd` - cwd property.
- nonce - защита от command spoofing.

Практический вывод:

- Если shell integration прислал `OSC 633 E`, это самый надежный источник command text.
- Если нет `E`, можно fallback на A/B/C, но помечать confidence ниже.
- Если нет shell integration, command blocks должны быть `unknown` или `heuristic`, не `trusted`.
- На Windows ConPTY может reprint viewport и смещать ожидания по позициям markers. Нужны Windows-specific tests.

### Windows Terminal - `OSC 133` как базовый слой

Windows Terminal использует `OSC 133` для prompt/command/output/finish и дает UX:

- scroll to previous/next prompt;
- select whole command;
- select command output;
- mark success/error через exit code;
- PowerShell prompt integration примером показывает, что exit code и cwd можно брать из shell.

Практический вывод:

- `OSC 133` должен быть базовым protocol, потому его понимают Windows Terminal, WezTerm, Kitty, Ghostty, VS Code в degraded mode.
- Для PowerShell делаем first-class integration.
- Для cmd.exe делаем weaker integration и явно показываем lower fidelity.

### iTerm2 - процессная persistence через long-lived servers

iTerm2 показывает правильную границу:

- crash/upgrade может пережить за счет long-lived server;
- обычный quit может убить jobs;
- reboot jobs не переживают;
- window content может восстановиться даже без live process.

Практический вывод:

- В native backend нельзя обещать "живой процесс переживет перезапуск app".
- Можно обещать durable history, visual restore и command replay с подтверждением.
- Для live process persistence нужен zellij/tmux/managed daemon process model.

### Zellij - resurrection safety

Самый сильный safety-паттерн Zellij:

- layout и commands сериализуются;
- viewport/scrollback можно включить отдельно;
- команды после resurrection не запускаются сразу;
- пользователь видит "Press ENTER to run".

Практический вывод:

- Никогда не auto-run restored commands по умолчанию.
- Особенно нельзя auto-run destructive commands, `rm`, deploy, migrations, production commands.
- Нужно состояние `restored_pending_confirmation`.

### tmux и tmux-resurrect - detach отличается от restore

tmux сам по себе решает другое:

- server живет отдельно и переживает disconnect;
- можно reattach к живому процессу;
- history-limit ограничивает pane history;
- `capture-pane` умеет брать visible/history/alternate screen.

tmux-resurrect добавляет:

- сохранение layout/cwd/focus;
- optional pane contents;
- conservative allowlist для running programs;
- idempotent restore.

Практический вывод:

- Для mux backend мы должны отличать `attach existing live mux session` от `restore saved transcript`.
- Restore running programs должен быть allowlist + confirmation.
- Pane content restore optional, потому он дорогой и не всегда точный.

### Kitty, WezTerm, Ghostty - shell integration ломается в реальном мире

Общий урок:

- shell integration не всегда включается автоматически;
- темы, prompt plugins, subshells, nix-shell, ssh, sudo, tmux/zellij passthrough ломают markers;
- некоторые shells имеют native markers, другие нет;
- integration лучше разбивать на feature flags: prompt marks, cwd, title, sudo, ssh, cursor.

Практический вывод:

- Нужен `ShellIntegrationQuality`:
  - `none`
  - `heuristic`
  - `markers_basic`
  - `markers_with_exit`
  - `rich_with_commandline`
  - `trusted_with_nonce`
- UI должен уметь сказать: "История команд точная" или "Границы команд определены эвристически".
- Для zellij/tmux нужно проверять passthrough `OSC`.

### asciinema и script - raw journal должен быть append-friendly

asciinema v2 полезен как storage-pattern:

- первая строка - header;
- дальше NDJSON event stream;
- события output/input/marker/resize;
- incremental writing лучше переживает crash;
- input не пишется по умолчанию.

`script(1)` полезен как warning:

- raw input logging может записать пароли, даже когда terminal echo off.

Практический вывод:

- High-volume stream лучше хранить append-only segments.
- Input capture должен быть opt-in или redacted.
- Для replay нужен timing/sequence, resize events и metadata.

### JetBrains - command block как IDE API

JetBrains новый terminal и embedded terminal API полезны не только продуктово, но и архитектурно:

- command block имеет output text и lifecycle callbacks;
- plugins могут подписываться на command started/finished;
- terminal становится semantic surface для IDE, а не opaque PTY widget;
- при этом JetBrains держит fallback/classic terminal, потому compatibility pain реальный.

Практический вывод:

- У нас должен быть backend/runtime API `list_command_blocks`, `command_block_output`, `subscribe_command_blocks`.
- Command block должен иметь stable id и не зависеть от React component lifecycle.
- Если shell integration сломалась, UI должен деградировать до transcript view, а не ломать терминал.

### Nushell, Atuin, PSReadLine - command history это отдельная система

Shell-level history уже давно решает часть задачи:

- Nushell умеет хранить history в SQLite, задавать isolation, sync_on_enter и max_size.
- Atuin хранит shell history с metadata и sync, но отдельно решает encrypted sync и deletion.
- PSReadLine хранит persistent PowerShell history, но это не равно текущей in-memory session history.
- cmd.exe через doskey имеет ограниченный command recall, но мало semantic metadata.

Практический вывод:

- Наша история не должна заменять shell history. Это другой слой.
- Нужно отличать:
  - shell native history;
  - user-visible command blocks;
  - raw terminal transcript;
  - UI command dock recents;
  - AI/context history.
- Import/export из shell history можно сделать позже, но storage source of truth для sessions должен быть наш journal.

### xterm.js и browser terminal - data is untrusted

xterm.js security docs важны для нашей browser UI:

- терминальный output может содержать escape sequences, links, OSC commands;
- keystrokes проходят через JS и могут быть sensitive;
- terminal input/output нельзя считать safe text.

Практический вывод:

- Persisted output при render/search/export должен проходить safe rendering path.
- Links/OSC/clipboard actions из restored history не должны auto-execute.
- Raw replay должен идти в terminal emulator как controlled restore, а не в DOM через `innerHTML`.
- При export в markdown/html надо escape/sanitize.

### SQLite - local-first, но с operational discipline

SQLite подходит под local terminal history, но есть несколько условий:

- WAL режим требует checkpointing.
- Live backup надо делать через SQLite backup API, а не копированием файла.
- Long read transaction может удерживать WAL.
- Huge blobs/rows имеют limits и бьют memory/perf.
- Single writer проще и надежнее, чем много concurrent writers.

Практический вывод:

- Делать `TerminalHistoryWriter` как single writer actor.
- Писать segments транзакциями.
- Делать WAL checkpoint по size/time.
- Измерять DB size, WAL size, writer lag, dropped/degraded writes.
- Backup/export через controlled API.

### tlog и Guacamole - terminal history как audit recording

Audit/session recording системы полезны тем, что они давно решают "полную историю" не как convenience, а как ответственность:

- запись отделена от playback;
- хранится timing;
- terminal size/resize важны;
- есть policy, кто записывается и как долго хранится запись;
- есть notice пользователю;
- есть risk feedback loops, когда playback сам пишет новые записи;
- storage latency влияет на user interaction.

Практический вывод:

- Нужно проектировать `history player`, а не только restore в live pane.
- Replay/export не должен писать события обратно в active journal.
- Для compliance/private mode нужен visible state: recording on/off/degraded.
- Журнал должен быть пригоден для диагностики и forensics, но sensitive by default.
- Writer должен иметь backpressure/degraded path, чтобы storage stall не ломал terminal input.

### Teleport - best effort vs strict recording

Teleport полезен не форматом, а policy:

- `best_effort` - если запись недоступна, сессия может продолжиться;
- `strict` - если запись невозможна, сессию лучше не открывать/прервать;
- recorded sessions являются audit artifact и могут проигрываться отдельно.

Практический вывод:

- Для developer terminal default должен быть `best_effort + visible degraded`.
- Для enterprise/compliance profile можно добавить `strict_history_required`.
- В runtime health надо различать:
  - `history_recording_ok`;
  - `history_recording_degraded`;
  - `history_recording_required_but_failed`.
- В UI нужно показывать, когда история больше не гарантируется.

### Shell history policies - user intent matters

Bash/fish/zsh/Nushell/PSReadLine показывают, что история команд - это не просто "все что нажали Enter":

- Bash `HISTCONTROL=ignorespace` скрывает команды с leading space.
- fish private mode не сохраняет history.
- zsh умеет immediate append и shared history, но также ignore-space.
- Nushell имеет формат и sync/isolation policies.
- PSReadLine persistent history отличается от PowerShell session history.

Практический вывод:

- Если shell явно говорит "не сохранять", наш journal не должен обходить это без explicit setting.
- Command blocks должны иметь `history_policy_applied`.
- Raw output может сохраняться даже когда shell history не сохраняет command, но UI обязан понимать privacy tradeoff.
- Для command dock можно предупреждать, если команда начинается с space и shell policy implies private.

### Terminal escape sequences - active content, not text

Terminal output может:

- поменять title;
- перейти в alternate screen;
- создать hyperlinks;
- запросить clipboard через OSC 52;
- скрыть/переместить cursor;
- стереть/перерисовать строки;
- включить bracketed paste или mouse tracking;
- вывести shell integration markers.

Практический вывод:

- Stored output нельзя рендерить как HTML.
- Historical replay должен идти в "inert mode": no clipboard/window side effects.
- Hyperlinks из history должны быть sanitized.
- Parser/replay должен учитывать mode changes.
- Для search нужен derived text index, а не raw bytes.

### GNU screen и GNOME Terminal - old lessons still matter

Старые терминалы показывают, что даже базовая "история" давно распадается на разные операции:

- scrollback buffer - посмотреть назад;
- logging - записывать stream;
- hardcopy/export - снять текстовый снимок;
- unlimited scrollback - удобство, но не reliable storage.

GNOME Terminal прямо предупреждает, что большой scrollback может замедлять resize. Значит durable history не должна жить в renderer buffer.

Практический вывод:

- UI scrollback должен быть ограниченным и быстрым.
- Durable journal хранится на диске и читается page/window chunks.
- Export создается из journal/read model, а не из visible buffer.
- Search идет по derived index, а не по React/xterm buffer.

### Search architecture - FTS as derived state

SQLite FTS5 полезен для transcript search, но FTS index нельзя делать единственным хранилищем:

- terminal output содержит ANSI/OSC/control sequences;
- line wrapping зависит от width;
- parser может улучшиться и изменить derived text;
- deletion/redaction должны чистить raw, snapshots и index;
- FTS может отставать от writer при degraded mode.

Практический вывод:

- `terminal_search_chunks` должен иметь `source_segment_id`, `parser_version`, `redaction_version`.
- FTS rebuild должен быть возможен из raw segments.
- Search UI показывает `index freshness`.
- Если index corrupted, history restore все равно работает.

### Encryption and secure deletion

SQLCipher/SQLite SEE показывают, что encryption at rest возможна, но это отдельный engineering track:

- build/linking;
- key storage;
- migration from unencrypted DB;
- backup/export behavior;
- crash recovery;
- performance.

Даже с encryption нельзя забывать:

- redaction before AI/share/export;
- private mode;
- backups;
- logs;
- FTS/snapshots;
- tombstones.

Практический вывод:

- Phase 1: redaction/private/delete scopes.
- Phase 2: optional encrypted store.
- Phase 3: sync/cloud only after encryption/deletion semantics are stable.

### sudo/sudoreplay - mature I/O log lessons

sudo полезен тем, что у него зрелая модель recording:

- I/O logs отделены от policy/event logs;
- `sudoreplay` может list/search/replay recordings;
- timing files нужны для playback;
- terminal size влияет на replay;
- password hiding появился как отдельная feature, потому raw I/O опасен;
- recordings можно хранить локально или через log server.

Практический вывод:

- `terminal_journal_events` и `terminal_stream_segments` должны быть разными слоями.
- Нужен `history player`, который умеет replay/search/list без live session.
- Redaction/password handling - обязательный early feature, не polish.
- Audit/compliance mode должен иметь strict policy and tamper-evident checks.

### Unicode, width, graphemes

Terminal transcript - это не просто UTF-8 string:

- emoji могут занимать 2 cells;
- combining marks могут занимать 0 cells;
- East Asian Ambiguous width зависит от policy;
- Unicode version меняет width tables;
- terminal emulator может иметь fallback glyph behavior;
- parser bugs могут менять wrapping/search.

Практический вывод:

- snapshots должны хранить `parser_version`, `unicode_version`, `cell_width_policy`.
- derived search text должен иметь `parser_version`.
- raw segments остаются source of truth.
- visual replay должен использовать recorded rows/cols и compatible width policy.

### Images and rich terminal protocols

Terminal output может быть media:

- Kitty graphics protocol;
- iTerm2 inline images;
- Sixel;
- WezTerm imgcat;
- hyperlinks;
- OSC 52 clipboard;
- title/window controls.

Практический вывод:

- stream segment payload может быть binary/bytes, не только UTF-8.
- `terminal_media_artifacts` может понадобиться позже для dedupe/redaction/export.
- markdown/plain export должен решать: omit media, link artifact, or embed.
- historical replay должен отключать unsafe side effects.

### Paste and input source attribution

Bracketed paste mode существует именно потому, что paste отличается от typed input:

- paste может содержать newlines/multiple commands;
- shells/editors обрабатывают paste иначе;
- pasted secrets встречаются часто;
- command dock submit - еще один отдельный input source.

Практический вывод:

- command/input records должны иметь `input_source`: typed, paste, ui_submit, programmatic, restored_confirmation.
- raw paste content sensitive by default.
- command block может быть created from pasted multi-line input, значит one input != one command.

### Keyboard/mouse protocols - input is semantic too

Terminal input имеет несколько уровней:

- physical key / mouse action;
- browser/OS key event;
- terminal key encoding protocol;
- PTY bytes;
- shell/editor interpretation.

Kitty keyboard protocol, xterm `modifyOtherKeys`, CSI-u и WezTerm key encodings показывают: без protocol metadata нельзя точно объяснить, почему в shell ушли именно такие bytes.

Практический вывод:

- Для normal history достаточно command blocks и trusted command text.
- Для debug/forensic mode нужен `terminal_input_events`.
- Mouse reporting events должны быть отдельным input kind.
- Исторический replay input в live process должен быть forbidden without explicit confirmation.

### Parser conformance

Terminal parser - это отдельный продуктовый риск:

- control sequences много;
- xterm behavior де-факто стандарт;
- VTE/libtsm/vte crates показывают, что state machine лучше отделять;
- parser bug меняет snapshots/search/replay;
- conformance tests нужны до "полной истории".

Практический вывод:

- Завести golden fixtures для raw stream -> screen snapshot.
- Завести fixtures для raw stream -> command blocks/events.
- Snapshot/read models должны иметь `parser_version`.
- Parser upgrade может требовать rebuild derived search chunks.

### Shell transcript vs terminal transcript

PowerShell `Start-Transcript` и GNOME Terminal "save contents" полезны как UX, но это не то же самое:

- PowerShell transcript не знает terminal layout/panes/resize/alt-screen/mux.
- Save contents может сохранить только текущий text view.
- Они не дают reliable command output sequence ranges.
- Они не отвечают на native vs zellij restore semantics.

Практический вывод:

- Можно добавить export/import adapters позже.
- Source of truth остается terminal journal.
- Shell transcript можно прикреплять как artifact, но не заменять journal.

### Accessibility and semantic navigation

Command blocks помогают не только красивому UI:

- screen reader может навигировать по commands;
- keyboard shortcuts могут jump previous/next command;
- failed commands можно быстро найти;
- output region можно выделить без mouse;
- restored/live boundary можно озвучить.

Практический вывод:

- Command blocks должны иметь accessible labels.
- Timeline/list UI должен работать keyboard-only.
- Restore banner должен быть announced.
- Search results should point to command/block provenance.

### PTY, ConPTY and capture layers

Unix PTY and Windows ConPTY define the real stream boundary:

- UI submit happens before PTY.
- Shell integration markers are bytes inside PTY output.
- Projection deltas happen after parser/render.
- Mux APIs may expose pane surfaces without raw child PTY bytes.
- Audit modules may capture keystrokes below shell integration.

Практический вывод:

- Every event should record `capture_layer`.
- Trust/confidence depends on capture layer.
- Native Unix PTY and Windows ConPTY need separate fixtures.
- Resize must be persisted as process/PTY event, not only UI geometry.

### termios, ECHO and password safety

`termios` gives important signals on Unix:

- `ECHO` off often means password/sensitive input.
- `ICANON` changes line buffering behavior.
- raw mode means app/TUI controls input directly.

Практический вывод:

- Raw input capture should check ECHO/raw mode when possible.
- If ECHO off, redact input aggressively.
- On Windows/ConPTY, where signal differs, default remains raw input off.
- Command text from shell integration can be more privacy-aware than key logging.

### Reconnect vs restore vs recording

Mosh, Eternal Terminal and dtach clarify three different capabilities:

- reconnect - live process continues across network drops;
- restore - user sees previous context after runtime restart;
- recording - historical transcript can be replayed/searched.

Практический вывод:

- Product UI should not collapse these into one "session saved" phrase.
- `preserves_process_state` needs detail: live attach, restarted, historical only.
- zellij/tmux are closer to live attach; native journal is historical restore.
- Mosh-like state sync inspires fast convergence, but not command history by itself.

### Mux and remote domains

tmux/zellij change the model:

- one server/session can have multiple clients;
- clients can be different sizes;
- clients can be read-only;
- mux may filter or wrap escape sequences;
- mux has its own commands/API;
- pane output should be separated from outer mux UI output.

Практический вывод:

- `session_id` is not enough; need mux/session/client/view identifiers.
- For tmux, prefer control mode where feasible.
- For zellij, prefer structured CLI/API/pane output surfaces where feasible.
- Shell integration quality must be measured inside each pane.
- Outer raw terminal stream from mux is not a reliable pane transcript.

### CWD and remote identity

CWD tracking is useful but sensitive:

- OSC 7 can include host and path.
- Remote shells can report remote cwd.
- SSH/tmux/zellij nesting can make "current cwd" ambiguous.
- Paths can leak usernames, project names, customer names.

Практический вывод:

- `cwd_source`, `cwd_host`, `cwd_path`, `cwd_trust`, `cwd_redaction_state`.
- Do not blindly display/share cwd in exported history.
- Shell integration CWD should be session/pane-scoped, not global.
- Remote cwd should be marked as remote context.

### Multi-client viewport semantics

If two clients view the same mux pane at different sizes:

- process may see one pane size or mux-managed size;
- each client may see different wrapping/cropping;
- scrollback/viewport is not identical per client;
- clipboard side effects are local to client.

Практический вывод:

- Store process/pane size separately from client viewport size.
- Replay historical pane content with recorded pane size.
- UI can have local viewport snapshots but not call them session truth.

### Event sourcing and projections

Terminal persistence is naturally event sourced:

- append stream segments and semantic events;
- derive command blocks;
- derive screen snapshots;
- derive search chunks;
- derive accessibility timeline;
- derive AI context windows;
- rebuild derived state when parser/redaction/schema changes.

Практический вывод:

- Raw segments/events are immutable source of truth.
- Derived tables need `projection_version`.
- Rebuild workers should be idempotent.
- Snapshot/projection corruption should not destroy raw history.
- Tests should replay raw fixtures into all projections.

### Tamper evidence

Checksums detect corruption, but not intent. Strict/audit profiles need stronger guarantees:

- segment hash;
- previous segment hash;
- session chain root;
- signed checkpoints;
- exported audit bundle with root hash;
- validation command.

Практический вывод:

- Default mode: checksum and integrity diagnostics.
- Strict mode: hash chain and optional signatures.
- UI can report "history integrity verified" or "history changed after recording".

### Safe transcript viewing

Terminal history viewer is a log viewer for hostile content:

- ANSI/OSC can move cursor, clear screen, create links, set title, request clipboard.
- Unicode bidi/control chars can make command text visually deceptive.
- Newlines/control chars can forge log lines.
- Hyperlinks can hide real URLs.

Практический вывод:

- Viewer renders inert/sanitized transcript, not live terminal side effects.
- Raw replay only in controlled terminal emulator replay mode.
- Export escapes control chars or marks them.
- Suspicious Unicode can be flagged in command/output views.

### Log lifecycle and disposal

NIST log management is relevant even for local developer history:

- generation;
- buffering/transmission;
- storage;
- analysis/search;
- archival;
- disposal.

Практический вывод:

- Retention policy belongs in data model.
- Delete must cover raw, projections, exports, backups and cache.
- Maintenance job should be observable.
- User-visible controls: clear block/session/workspace/all, private mode, export warning.

### Nested command domains and REPLs

The terminal may contain nested interpreters:

- shell starts `python`, `node`, `psql`, `mysql`, `ssh`, `docker exec`, `npm run dev`;
- inside those apps there may be prompts, commands, history and output;
- shell integration only sees the outer command lifecycle;
- app-level history may store inner commands but usually not terminal output or pane context.

Практический вывод:

- Add `command_domain` / `nested_domain` concept.
- Outer shell command block can be `python`, while inner REPL transcript is terminal-only unless integration exists.
- Do not fake inner command blocks from prompt-looking text unless marked heuristic.
- Future app integrations can promote inner commands to blocks.

### Structured journal events

OpenTelemetry logs are a useful shape for terminal events:

- `timestamp_ms` - when event happened.
- `observed_at_ms` - when writer observed it.
- `event_name` - stable semantic name.
- `attributes_json` - structured metadata.
- `resource/session/pane` - identity context.
- `trace_id` / `correlation_id` - connect command, process, output and export.

Практический вывод:

- Avoid ad hoc JSON blobs without stable attribute names.
- Keep event schema version.
- Correlate UI submit, shell marker, process creation and stream segments.
- Process/audit enrichers should attach attributes, not rewrite truth.

### Process creation correlation

OS-level process audit is useful but incomplete:

- Windows Event 4688/Sysmon can log process creation.
- Linux audit can trace execve.
- Shell built-ins (`cd`, `alias`, functions) may not create processes.
- REPL commands may not create new OS processes.
- Privileges/configuration requirements are high.

Практический вывод:

- Optional process correlation layer.
- It can improve command metadata/process tree view.
- It must not be required for basic history.
- It cannot replace shell integration and terminal journal.

### Large BLOB and artifact strategy

Huge outputs and media need careful storage:

- one giant SQLite row is hard to prune/read/replay;
- external files complicate backup/delete/integrity;
- incremental BLOB I/O can help but still needs policy;
- content hashes enable dedupe and integrity checks.

Практический вывод:

- bounded stream segments for default path;
- optional content-addressed artifact store for large/media payloads;
- DB metadata remains source for lifecycle/delete/export;
- backup must include DB and external artifact store consistently.

### SQLite operational hardening

SQLite reliability depends on how the DB is used:

- do not put hot history DB on unreliable network/cloud-synced filesystem by default;
- do not copy DB files while writer is live without backup API;
- WAL/SHM files are part of live state;
- antivirus/indexers/cloud sync can interfere with files;
- one writer actor is easier to reason about than many random connections;
- corruption recovery should salvage partial history, not fail all sessions.

Практический вывод:

- History store location should default to local app data.
- Backup/export must use SQLite backup API and artifact manifest.
- Add `storage_health` and `last_integrity_check`.
- Add recovery command/test path.

### Redaction and secret scanning

Terminal transcript leaks secrets through many surfaces:

- command text;
- raw input/paste;
- stdout/stderr;
- cwd/URLs;
- env dumps;
- screenshots/media;
- search chunks;
- exports and AI context.

Практический вывод:

- redaction profile/version in data model;
- secret findings table without storing raw secret;
- scan raw and derived artifacts;
- export/share/AI must require redaction pass;
- UI can show `possibly_sensitive` instead of overpromising safety.

### SQLite writer configuration

The history writer should own SQLite connection policy:

- `PRAGMA journal_mode=WAL`;
- `PRAGMA foreign_keys=ON`;
- `PRAGMA busy_timeout=...`;
- `PRAGMA synchronous=NORMAL` for best-effort developer mode;
- stronger synchronous mode for strict/audit mode;
- explicit checkpoint and journal size policy;
- explicit `BEGIN IMMEDIATE` batch transactions.

Практический вывод:

- PRAGMAs belong in store initialization, not scattered queries.
- Connection tests must assert PRAGMAs.
- Strict mode can trade latency for stronger durability.
- Writer contention should fail before partial batch work.

### Diesel migration discipline

Diesel helps with typed persistence, but migration quality is still our responsibility:

- migrations are versioned files;
- embedded migrations ship with binary;
- generated schema must match migrations;
- tests should open old DB fixtures and migrate forward;
- migrations that change derived projections should mark them stale.

Практический вывод:

- Add migration tests for empty DB, old DB and corrupted/partial DB.
- Do not mix Diesel schema and raw SQL schema changes without tests.
- Migration should be idempotent and explicit about projection rebuilds.

### Compression strategy

Terminal output is stream-like but user queries are random-access:

- command output range;
- search result context;
- replay from last snapshot;
- prune old segment ranges;
- export one block.

Практический вывод:

- Compress per bounded segment, not whole DB/session.
- Store uncompressed/compressed byte lengths and checksum.
- Pick zstd for cold/high-ratio segments, LZ4 or no compression for hot/low-latency segments.
- Compression is part of segment metadata and backup compatibility.

### Serialize projection and fast hydration

Есть два разных restore-сценария:

- correctness restore - восстановить truth из DB journal/snapshots;
- fast hydration - быстро показать пользователю последний viewport/scrollback.

xterm-style serialize projection полезен для второго:

- after browser reconnect;
- after UI reload;
- before full journal replay finishes;
- for cheap visual snapshots.

Но его нельзя делать единственным источником истории:

- serialized buffer can miss command semantics;
- derived projection can contain already-redacted/normalized text;
- malicious escape sequences and alternate screen can distort visible state;
- projection format can change with terminal renderer version.

Практический вывод:

- Store canonical stream/journal segments separately.
- Store projection snapshot with `renderer_version`, `terminal_size`, `unicode_version`, `capability_profile_id`.
- Mark projection as rebuildable cache unless user-visible restore depends on it.
- On corruption/mismatch: rebuild from journal if possible, otherwise show degraded restore.

### OS storage and key protection

History DB is sensitive because it contains commands, output, cwd, paths, tokens and possibly personal data.

Storage policy should separate:

- durable DB/state;
- derived cache;
- runtime sockets/locks;
- config;
- encryption keys and sync tokens.

Windows-specific default:

- DB/state under LocalAppData or app state folder;
- optional roaming metadata only if explicitly designed for sync;
- keys/tokens protected with DPAPI/Credential Manager;
- no hardcoded `C:\Users\...` paths in persistence code.

Cross-platform default:

- use OS folder APIs/wrapper crate;
- keep cache disposable;
- keep runtime files out of backups;
- treat secret storage as a separate dependency boundary.

### Reliability metrics and SLOs

"History is reliable" needs measurable evidence.

Minimum SLI set:

- percent of stream segments durably committed before pane close;
- max writer lag in milliseconds;
- queue depth and overflow count;
- dropped/truncated segment count;
- recovery success/failure count;
- snapshot age and replay distance;
- DB busy/locked duration;
- WAL/checkpoint duration;
- redaction failure count;
- export blocked by sensitive findings.

Suggested SLO shape:

| Area | Example target | Why |
| --- | --- | --- |
| Durability | 99.99% of non-private output segments committed within 2s | User expects restart not to lose work. |
| Restore | 99% restores show last snapshot under 500ms, full replay continues async | Fast UX without lying about completeness. |
| Recovery | 100% corrupt projections are rebuildable or quarantined without DB loss | Derived cache failure must not destroy truth. |
| Privacy | 100% exports pass latest redaction profile before leaving app | History is high-risk data. |

### Backpressure and queue health

Terminal output can burst faster than DB writes and browser rendering.

Backpressure chain:

```text
PTY/ConPTY/mux stream
  -> gateway reader
  -> parser/projection
  -> history writer queue
  -> DB batch transaction
  -> browser stream queue
  -> renderer buffer
```

Each arrow needs explicit behavior:

- bounded queue size;
- overflow policy;
- metrics;
- degraded state;
- user-visible warning if data may be incomplete.

Recommended modes:

| Mode | Behavior | Use |
| --- | --- | --- |
| `interactive_best_effort` | Preserve UI responsiveness, drop oldest derived projections first, never silently drop canonical segments without degraded marker | Default developer terminal. |
| `strict_history` | Slow reader/writer before dropping canonical history | Audit/high-value sessions. |
| `private` | Do not persist raw input/output, only minimal session metadata | Secrets/incognito. |

### Fuzzing, property tests and chaos fixtures

Terminal history is parser + storage + replay, so normal unit tests are not enough.

Must-have test classes:

- fuzz parser with random ANSI/OSC/UTF-8/control bytes;
- fuzz redaction with partial secrets across segment boundaries;
- property-test replay idempotence;
- property-test sequence ordering and command block ranges;
- crash during batch insert;
- crash after stream segment commit but before projection snapshot;
- DB locked while output bursts;
- disk full during compression/export;
- corrupt projection artifact;
- stale migration/rebuild path;
- WebSocket disconnect during long output;
- Windows ConPTY resize during output burst;
- zellij/tmux passthrough with pane splits and attach/detach.

Minimal invariant set:

```text
stream_seq is monotonic per pane
segment ranges do not overlap
command block stream ranges are valid or marked incomplete
snapshot projection_seq <= last committed projection_seq
derived projection can be deleted without deleting canonical journal
redaction never exposes raw secret in search/export indexes
restore never auto-runs historical commands
```

### Event envelope, schema and correlation

Terminal history events should have two layers:

- stable event envelope;
- typed payload for event-specific details.

Envelope shape:

```text
event_id
event_type                 -- terminal.output, terminal.resize, command.started, command.finished, history.redacted
source                     -- app/runtime/session/pane/backend identity
occurred_at_ms
observed_at_ms
schema_version
trace_id
span_id
correlation_id
capture_layer
trust_level
payload_encoding           -- json, bytes_ref, cbor_future
payload_schema_ref
```

Why this matters:

- UI submit, shell marker and process/audit event can be correlated;
- import/export can validate records;
- schema upgrades can upcast payloads;
- partial stream recovery can skip bad records without losing the whole file.

### SQLite hardening and resource limits

Terminal history stores hostile data: ANSI escape sequences, huge logs, malformed Unicode, imported transcripts and user search queries.

Hardening defaults:

- open DB through one persistence boundary;
- set PRAGMAs and DB config in one initializer;
- prefer STRICT tables for new typed schema when SQLite version supports it;
- lower SQLite runtime limits for import/search paths;
- keep `trusted_schema` off for untrusted/imported DB handling;
- avoid user-controlled SQL fragments;
- run integrity checks and projection rebuilds after crash/import.

Practical limits to define:

| Limit | Why |
| --- | --- |
| max segment bytes | Prevent one command from creating unbounded memory/DB pressure. |
| max output per command block before cold artifacting | Keep UI/search responsive. |
| max import file size | Avoid local DoS through imported transcripts. |
| max search query length | Avoid expensive pathological FTS queries. |
| max export event payload | Keep exports streamable and recoverable. |

### Windows command identity and rerun semantics

On Windows, command history cannot treat text, argv and process launch as the same thing.

Different identities:

- displayed command line;
- exact command text entered in PowerShell/cmd/bash;
- native argv after shell parsing;
- process command line from OS audit;
- UI template used by "rerun";
- shell integration marker command text.

Rerun rule:

- rerun through the same shell/domain when possible;
- preserve quoting/parsing mode metadata;
- never convert PowerShell command text to generic argv automatically;
- show edit/confirm when command source is heuristic or parsing context is missing.

### Backup, sync and point-in-time restore

Three features must stay separate:

| Feature | Source | User expectation |
| --- | --- | --- |
| Session restore | Terminal journal + snapshots + launch metadata | Continue work context after app restart. |
| Backup/PITR | DB/WAL/artifact backups | Recover older local data after deletion/corruption. |
| Sync | App-level events or DB changesets with conflict rules | Move history across devices/accounts. |

Useful patterns:

- SQLite Session Extension for future DB change exchange;
- Litestream-like WAL backup for local history disaster recovery;
- terminal journal events for semantic restore and UI;
- export/import event stream for user-controlled portability.

Do not promise:

- DB backup equals live process restore;
- sync can merge arbitrary live terminal process state;
- PITR can safely resurrect commands without user action.

### Encrypted external artifacts

Some payloads should eventually leave the main SQLite file:

- large inline images/media;
- huge command outputs;
- debug bundles;
- cold archived segments;
- exports.

If stored externally, the artifact layer needs:

- content hash;
- authenticated encryption or MAC;
- chunk order and final marker validation;
- artifact metadata in DB;
- garbage collection with DB tombstones;
- recovery if DB points to missing artifact.

Rule:

- encrypted external artifact without authentication is not enough;
- artifact encryption key must be OS-backed or derived from OS-backed root key;
- every decrypted artifact must be checked before replay/export.

### Export/import contracts

Export is product API, not just "dump JSON".

Recommended export bundle:

```text
manifest.json
schemas/
events.jsonseq
segments/
artifacts/
checksums.txt
redaction_report.json
```

Required behavior:

- validate event schemas;
- include terminal size/resizes/timing;
- include redaction profile/version;
- include source app/schema version;
- allow partial import with quarantine;
- never import active content as executable commands;
- import unknown event types as opaque records only when compatibility allows it.

### Deletion, compaction and temp-file privacy

Deletion has multiple layers:

```text
user intent
  -> logical tombstone
  -> raw payload purge/redaction
  -> derived index rebuild
  -> WAL/checkpoint policy
  -> compaction/VACUUM policy
  -> backup/export retention
  -> external artifact GC
  -> temp/cache cleanup
```

Important distinctions:

- `DELETE` changes logical state but may leave bytes in free pages.
- `secure_delete` helps ordinary tables but has performance and scope caveats.
- WAL/temp/export files may still contain sensitive bytes.
- Backups can keep old data after active DB is cleaned.
- FTS/search/projection caches need explicit rebuild after purge.

Recommended deletion classes:

| Class | Behavior | Use |
| --- | --- | --- |
| `hide_from_ui` | Hide block/session but retain raw data | Undo or soft delete. |
| `redact_payload` | Replace sensitive ranges and rebuild derived views | Secret leaked in command/output. |
| `purge_session` | Delete raw + derived + artifacts, checkpoint/compact on policy | User removes session history. |
| `privacy_export_delete` | Also check backups/exports inventory | Right-to-delete style cleanup. |

### Secret/key lifecycle in Rust persistence

Key storage and key handling are different.

Storage:

- OS-backed credential store for root secrets;
- DB stores only references and metadata;
- rotation/rekey records are durable;
- dev-only file provider is loudly marked unsafe.

Handling:

- secret wrapper types for key/token values;
- no `Debug` output for secrets;
- zeroize temporary buffers where practical;
- short-lived key material in memory;
- startup health check for missing/inaccessible secrets.

Non-goals:

- do not claim perfect memory erasure in managed/runtime layers;
- do not rely on obfuscation;
- do not store SQLCipher passphrase in app config.

### Accessible terminal history model

Accessible history needs semantic structure:

- command blocks as navigable log entries;
- status messages for `history restored`, `history degraded`, `search complete`, `export blocked`;
- keyboard route out of terminal raw input mode;
- focus target for history list, search result, command output, restore boundary;
- reduced-noise mode for live output announcements;
- screen-reader labels for trust/degraded/redacted states.

The terminal grid can remain visual. The accessible model should be a read model over command blocks, journal events and restore status.

### Media and file-transfer artifact sandbox

Terminal output may request images, file transfers, hyperlinks or inline media.

Safe baseline:

- capture protocol metadata without auto-opening files;
- size caps before decoding;
- store media as artifact records with checksums;
- lazy preview behind user action for unknown/large payloads;
- redact/export policy for images and binary data;
- block side effects during historical replay;
- capability profile records what renderer could display.

Artifact states:

```text
observed -> stored -> scanned -> previewable
observed -> blocked_oversize
observed -> quarantined_unknown_protocol
stored -> missing_on_disk
stored -> redacted
```

### Retention and privacy review loop

History retention should be understandable and changeable.

Policy dimensions:

- per workspace/project;
- per session;
- private mode;
- command category;
- huge output policy;
- bookmark/pin exceptions;
- backup/export retention;
- sync/delete propagation.

Review UX:

- show oldest retained history;
- show estimated DB/artifact size by category;
- show sessions with possible secrets;
- show backups/exports that may still contain old data;
- let user simulate prune before applying.

### Actionable metric buckets

Metrics should map to product decisions.

Examples:

| Metric | Good buckets/questions |
| --- | --- |
| `history_writer_commit_ms` | Is most output durable under 250ms/1s/2s? |
| `history_restore_snapshot_ms` | Does viewport appear under 100ms/500ms/1s? |
| `history_replay_catchup_ms` | How long until restored history is complete? |
| `history_checkpoint_ms` | Are WAL checkpoints causing UI pauses? |
| `history_redaction_scan_ms` | Can export happen interactively? |
| `history_artifact_decode_bytes` | Are media payloads within safe preview limits? |

### Scalable search tiers

Search should start simple, but not block future scale.

Top 3 architecture options:

1. SQLite derived chunks + optional FTS5.  
   🎯 10   🛡️ 9   🧠 6  
   Объем: примерно 500-900 строк.  
   Best baseline: same DB, transactional rebuilds, easy delete/retention, enough for local history.

2. Tantivy derived index for large/cold/global history.  
   🎯 8   🛡️ 8   🧠 8  
   Объем: примерно 900-1600 строк.  
   Strong Rust search engine, but index lifecycle, schema migrations and redaction rebuilds become separate responsibilities.

3. External search service.  
   🎯 4   🛡️ 5   🧠 9  
   Объем: примерно 1500-3000 строк.  
   Too much operational surface for local-first terminal history unless product later becomes multi-user/server-side.

Recommendation:

- Phase 1: derived search chunks + FTS5 if bundled SQLite supports it.
- Phase 2: Tantivy only for large history/global workspace search.
- Never make search index canonical.

### Durable background jobs and outbox

History persistence creates follow-up work:

- rebuild command blocks after parser upgrade;
- rebuild search chunks after redaction;
- compress cold segments;
- garbage-collect artifacts;
- compact/VACUUM on policy;
- prepare export bundles;
- verify backup checkpoints;
- sync metadata later.

These jobs must survive restart.

Pattern:

```text
DB transaction:
  insert/update canonical rows
  insert durable job/outbox rows
commit

worker:
  claim job
  execute idempotently
  write result/progress
  retry/backoff or quarantine
```

Rules:

- jobs are idempotent;
- jobs reference immutable sequence ranges or schema versions;
- job output is another projection/artifact with version;
- failed jobs surface in diagnostics UI;
- memory-only queue is allowed only as acceleration after durable enqueue.

### Sync and CRDT boundary

CRDTs are attractive, but terminal history has hard semantics:

- append-only output order matters;
- privacy deletion must propagate and be explainable;
- redaction can rewrite/delete payloads;
- replay requires deterministic sequence ranges;
- live process state is not mergeable.

Good CRDT candidates:

- bookmarks;
- annotations;
- shared session notes;
- layout preferences;
- command docs;
- collaborative review comments.

Bad CRDT candidates:

- raw terminal output;
- secret deletion;
- command execution order;
- process state;
- audit transcript integrity.

Recommendation:

- sync terminal events as ordered logs/checkpoints;
- use CRDT only for collaborative metadata around history;
- define delete/tombstone semantics before cloud sync.

### Time model and replay fidelity

Store two time classes:

```text
wall_time_ms              -- user-visible timestamp, sortable across sessions/devices roughly
monotonic_delta_us        -- timing inside one process/session
observed_wall_time_ms     -- when app observed event
source_clock              -- app, shell, mux, imported, remote
clock_quality             -- local, remote, skew_unknown, imported
```

Why:

- wall clock can jump;
- monotonic `Instant` cannot be persisted directly across process restarts;
- imported/asciicast data may have relative timing only;
- remote/mux sessions may report times from another machine;
- replay speed and UX timestamps need different clocks.

### AI/RAG history context safety

Terminal history is tempting AI context, but dangerous:

- output can contain prompt injection;
- commands can contain secrets;
- cwd/path/usernames can be sensitive;
- old output may be stale;
- untrusted command text can be spoofed by terminal output.

Minimum context packet:

```text
context_id
session_id
pane_id
command_block_ids
stream_seq_ranges
redaction_profile_id
trust_level
source_summary
included_fields
excluded_sensitive_findings
created_at_ms
```

Rules:

- attach command blocks with provenance and redaction state;
- include exact ranges, not just copied text;
- bound token/byte size;
- mark historical terminal output as untrusted content;
- never include private-mode sessions by default;
- keep AI export records in `terminal_export_manifests` or a dedicated context table.

### DB identity and compatibility guards

Before opening/migrating/importing:

- check file exists and is local/safe path;
- check SQLite header readable;
- check `application_id`;
- check `user_version`;
- run migrations only through owned persistence boundary;
- if mismatch: open read-only quarantine/import path, not normal app DB path.

This prevents:

- accidentally opening wrong SQLite file as history DB;
- applying migrations to an unrelated DB;
- treating imported debug bundle as live DB;
- corrupting user data during recovery.

### Windows process lifecycle model

Native Windows terminal sessions need more than `child.kill()`.

Lifecycle layers:

```text
launch spec
  -> CreateProcess / shell process
  -> ConPTY handle
  -> console control events
  -> Job Object membership
  -> graceful shutdown deadline
  -> terminate job fallback
  -> journal shutdown event
```

Important distinctions:

- CTRL+C/CTRL+BREAK are graceful console signals;
- closing ConPTY is terminal transport shutdown;
- terminating a Job Object is force cleanup;
- child process PID is not the whole process tree;
- zellij/tmux attach path should not be killed like native children.

Data to persist:

- backend kind;
- launch spec;
- process group/job ID where available;
- graceful stop attempt;
- force cleanup reason;
- exit status;
- orphan detection result.

### Consistent backup/export levels

Not every backup is equal.

| Level | Mechanism | Guarantees | Use |
| --- | --- | --- | --- |
| Logical export | Event/export manifest + artifacts | Semantic, portable, redacted | User export/import/debug. |
| SQLite online backup | SQLite backup API / compact copy | Consistent DB copy | Local backup while app runs. |
| Snapshot read | WAL snapshot/read transaction | Stable view for query/export | Long restore/export reads. |
| OS volume snapshot | VSS or platform backup | File-level volume state | Disaster recovery, not app semantics. |

Rule:

- semantic history export should not depend on OS volume snapshot;
- DB backup should include WAL/artifacts/manifest correctly;
- VSS can help external backup tools but does not know terminal deletion/redaction policy.

### File watcher and artifact integrity

External artifact store can be modified by antivirus, cleanup tools, user actions or sync tools.

Watcher usage:

- wake integrity scan quickly;
- update diagnostics;
- trigger missing-artifact quarantine;
- detect unexpected external writes.

Watcher non-goals:

- not a security boundary;
- not a complete event log;
- not a replacement for checksums;
- not a deletion policy.

Periodic integrity scan remains required:

```text
DB artifact manifest
  -> stat file
  -> hash if needed
  -> verify size/hash/encryption tag
  -> mark ok/missing/modified/quarantined
```

### Content-addressed artifact store

For large media/log/debug artifacts:

- store by hash;
- keep DB metadata and references;
- dedupe identical payloads;
- verify before replay/export;
- garbage-collect unreachable hashes.

Tradeoffs:

| Property | Benefit | Risk |
| --- | --- | --- |
| Immutable blobs | Strong integrity and dedupe | Harder privacy deletion if backups keep blob. |
| Hash IDs | Easy corruption detection | Secret-derived content hash may leak equality. |
| Shared artifacts | Saves space | Delete must be reference-counted/tombstoned. |

Recommendation:

- use content addressing only behind retention/deletion-aware metadata;
- keep encrypted artifact layer for sensitive payloads;
- never expose raw artifact hash as public/share identifier without threat review.

### Process checkpointing boundary

Process checkpoint/restore is a separate advanced capability:

- platform-specific;
- often privileged;
- fragile with terminals, sockets, GPU, files and network;
- not portable to Windows native backend;
- not the same as terminal history.

Honest product language:

- native backend restores history and launch context;
- mux backend can attach to still-running mux session;
- CRIU-like backend could restore specific Linux process trees later;
- no backend should auto-rerun destructive commands during restore.

### Terminal product evidence

Patterns from current products:

- Wave Durable Sessions make terminal history durable across reconnect/reboot and attach it to blocks.
- JetBrains exposes command blocks as API and keeps compatibility fallback.
- Warp uses block-centric command history and SQLite-backed session restoration.
- Zellij/tmux restore is powerful but mux-specific.
- Termius-style sync/vault UX raises user expectations, but terminal transcript sync has stronger privacy semantics.

Conclusion:

- durable blocks are the right UX model;
- native and mux restore need separate guarantees;
- sync/backup/deletion must be designed before cloud features.

### AI terminal adversarial surface

AI agents reading terminal history face special risks:

- terminal output can contain instructions to the model;
- command output can fake prompts/status;
- logs can include credentials;
- old history can be irrelevant or malicious;
- AI can misinterpret restored historical output as live state.

Required guardrails:

- mark all terminal output as untrusted content;
- include live/historical boundary;
- include provenance and trust source;
- strip/neutralize ANSI/OSC before model context;
- exclude private/sensitive sessions unless user explicitly selects them;
- cite exact command block and stream ranges.

### Transport delivery and reconnect semantics

Terminal history has two separate delivery loops:

- process/gateway to durable DB;
- durable DB/gateway to browser renderer.

The first loop protects history. The second loop protects what user sees live.

Browser transport state:

```text
client_id
connection_id
session_id
pane_id
last_server_seq_sent
last_client_seq_acked
last_client_input_seq
replay_window_start_seq
connected_at_ms
disconnected_at_ms
gap_state                 -- none, replayable, unrecoverable
```

Reconnect flow:

1. Browser sends last acked pane stream seq.
2. Server replays from DB or memory replay window.
3. If replayable, client fills gap before live stream.
4. If unrecoverable, UI marks visible history gap and offers full restore/reload.

Rules:

- WebSocket ping/pong only detects liveness, not missed semantic events.
- Every pane stream event needs monotonically increasing seq.
- UI input queued offline needs expiry and confirmation.
- Server must not trust client ack for retention until durable DB has data.

### Remote, SSH and WSL execution domains

`cwd` is not enough. A command belongs to an execution domain.

Domain examples:

| Domain | Example | Important metadata |
| --- | --- | --- |
| `windows_native` | `cmd.exe`, PowerShell | Windows cwd, codepage/ConPTY, PSReadLine. |
| `wsl_linux` | Ubuntu distro shell | distro name, Linux cwd, WSL version/config. |
| `win32_from_wsl` | `notepad.exe` launched from WSL | path translation, interop policy. |
| `ssh_remote` | remote shell over SSH | host alias, user redaction, remote cwd, ControlMaster. |
| `wezterm_ssh_domain` | WezTerm SSH domain | configured domain, remote shell integration quality. |
| `mux_remote` | zellij/tmux over SSH | mux session ID, client viewport, attach/resurrect mode. |

Data model implications:

- `cwd` should include domain and host/path kind.
- `rerun` must target same domain or ask user.
- Export/AI should classify host/user/path/port metadata as potentially sensitive.
- Shell integration quality is per domain and per mux layer.

### Feature flag rollout for persistence

Terminal persistence is high blast radius:

- storage writes on every pane;
- migrations;
- privacy behavior;
- restore UX;
- redaction/export;
- zellij/native divergence.

Flag categories:

| Flag type | Example | Lifetime |
| --- | --- | --- |
| Release flag | enable new writer for Windows native | Short. Remove after rollout. |
| Ops flag | disable stream persistence after DB incident | Long-lived kill switch. |
| Migration flag | use Diesel schema v2 writer | Until migration complete. |
| Permission flag | enable strict/audit history for selected users | Product policy. |
| Experiment flag | FTS5 vs derived chunks search | Temporary experiment. |

Rules:

- flags must be visible in diagnostics;
- persistence flags must be included in bug reports;
- stale flags get cleanup owner/date;
- disabling writer should put UI into explicit degraded/no-history state.

### Network chaos and reconnect fixture set

Required scenarios:

- WebSocket disconnect during long output;
- browser reload after server committed DB but before UI received event;
- server restart after client sent input but before ack;
- reconnect with replayable gap;
- reconnect with unrecoverable pruned gap;
- high latency while command floods output;
- packet loss/reordering on SSH/mux remote path;
- client offline buffered input expires;
- network recovers with multiple panes outputting simultaneously.

Assertions:

- no duplicate visible output after replay;
- no lost committed stream seq without gap marker;
- stale queued input is not auto-sent;
- restore from DB matches replayed live view;
- degraded state is visible and export records the gap.

### Remote context privacy profile

Sensitive fields:

- SSH host alias and real host;
- username;
- proxy/jump host;
- forwarded ports;
- agent forwarding;
- remote cwd;
- WSL distro name;
- Windows/Linux path mappings;
- cloud workspace/project names;
- environment markers like `WSLENV`.

Recommended policy:

- classify remote metadata separately from command/output;
- redact host/user/path by profile in exports/AI;
- keep full metadata local if user opts in;
- never expose SSH config details in share links by default.

### Event payload encoding and upcasting

Durable terminal events can start as JSON, but the schema must assume future changes.

Top 3 payload strategies:

1. JSON payloads with schema registry.  
   🎯 9   🛡️ 8   🧠 5  
   Объем: примерно 400-800 строк.  
   Best initial choice for debugability and migrations. Larger on disk, but compression handles much of it.

2. CBOR/MessagePack payloads with explicit schema IDs.  
   🎯 7   🛡️ 8   🧠 7  
   Объем: примерно 700-1200 строк.  
   Smaller payloads, but debugging and schema enforcement need more tooling.

3. Protobuf/FlatBuffers for stable event families.  
   🎯 7   🛡️ 9   🧠 8  
   Объем: примерно 1000-1800 строк.  
   Strong evolution story if discipline is high, but early feature churn makes it easy to overfit.

Recommendation:

- Phase 1: JSON + schema registry + upcaster tests.
- Phase 2: optional binary codec for high-volume payload classes.
- Always store `codec`, `schema_ref`, `schema_version` and `upcaster_version`.

Upcaster rule:

```text
old event payload -> validate old schema -> upcast -> validate current schema -> rebuild projections
```

### SQLite query plan and index discipline

Critical query classes:

- restore last N panes/session blocks;
- replay from snapshot seq to current seq;
- search command/output context;
- list recent command blocks by cwd/status;
- prune expired sessions;
- find artifacts with zero references;
- find pending failed jobs;
- find sensitive findings before export.

Required discipline:

- every critical query has an index rationale;
- partial indexes for non-deleted/active/pending rows;
- generated columns only when migration compatibility is acceptable;
- `EXPLAIN QUERY PLAN` golden snapshots for critical queries;
- `ANALYZE`/`PRAGMA optimize` maintenance after large imports/deletes;
- performance fixtures with realistic large history.

Example index candidates:

```text
terminal_stream_segments(session_id, pane_id, start_seq, end_seq)
terminal_command_blocks(session_id, pane_id, started_at_ms)
terminal_command_blocks(cwd, status, started_at_ms) WHERE deleted_at_ms IS NULL
terminal_background_jobs(state, next_run_at_ms, priority)
terminal_artifact_store(content_hash) WHERE deleted_at_ms IS NULL
terminal_redaction_findings(session_id, severity) WHERE state = 'active'
```

### Crash, telemetry and diagnostics privacy

Diagnostics surfaces:

- crash dumps;
- app logs;
- tracing spans;
- breadcrumbs;
- debug bundles;
- panic messages;
- database error messages;
- feature flag reports;
- OS-level Windows Error Reporting.

Policy:

- terminal command/output/path fields are sensitive by default;
- crash upload must pass privacy gate;
- breadcrumbs should reference IDs/ranges, not raw output;
- debug bundle generation is explicit user action;
- private-mode sessions excluded by default;
- telemetry baggage must not carry cwd/host/user/token-like metadata.

Recommended diagnostic event shape:

```text
diagnostic_id
session_id_hash
pane_id_hash
event_kind
feature_flags
writer_health_summary
last_error_class
raw_text_included=false
redaction_profile_id
```

### Enterprise retention and legal hold

Enterprise/audit mode needs different semantics from developer mode.

States:

| State | Meaning |
| --- | --- |
| `normal_retention` | Prune according to user/workspace policy. |
| `user_deleted` | User requested deletion; tombstone retained if sync/audit requires it. |
| `legal_hold` | Do not purge even if retention expires. |
| `audit_immutable` | Append-only record required, deletion becomes redaction/tombstone. |
| `privacy_purge_pending` | Waiting for backup/export propagation. |

Rules:

- UI must explain why data cannot be deleted immediately.
- Legal hold must be explicit, scoped and auditable.
- Developer/local mode should not inherit enterprise retention by accident.
- Audit integrity and privacy deletion are separate product profiles.

### Quota and disk-pressure policy

Storage pressure plan:

```text
measure DB + WAL + artifacts + cache
  -> compare soft/hard quota
  -> stop optional derived work first
  -> compress/cold-prune eligible data
  -> warn user before canonical loss
  -> enter degraded/no-history mode only with explicit marker
```

Priority order under pressure:

1. Delete rebuildable caches/projections.
2. Stop previews/exports/imports.
3. Compress cold segments.
4. Prune expired non-pinned sessions.
5. Ask user before deleting canonical active history.

Never silently delete:

- pinned/bookmarked blocks;
- active command output;
- legal hold sessions;
- unexported debug evidence for current incident;
- data needed to satisfy explicit strict/audit mode.

### Browser lifecycle and multi-tab ownership

Browser terminal UI can disappear without a clean shutdown:

- mobile/browser process kill;
- tab discard;
- freeze while hidden;
- bfcache interactions;
- laptop sleep;
- network change;
- duplicate app tab.

Rules:

- server DB is canonical for history;
- browser cache is projection;
- no critical persistence in `beforeunload`;
- client reconnect uses ack/replay state;
- only one input owner per pane unless collaborative mode is explicit;
- read-only viewers are allowed but must not steal input focus/ownership.

Multi-tab model:

```text
session_id
pane_id
client_id
client_role               -- input_owner, viewer, background_cache, reconnecting
lock_source               -- server, web_lock, user_override
last_heartbeat_ms
last_visibility_state
```

### Clipboard, paste and OSC52 side-effect policy

Clipboard operations cross security boundaries:

- local browser clipboard;
- terminal OSC52 clipboard requests;
- remote SSH/tmux/zellij clipboard passthrough;
- paste into live process;
- copy from historical replay.

Policy:

- historical replay never writes clipboard;
- live OSC52 requires allowlist/permission/profile;
- paste events are distinct from typed input;
- clipboard writes are journaled as side-effect attempts;
- remote/container/mux domain is included in side-effect decision;
- exports/AI never include clipboard payload by default.

Suggested side-effect states:

```text
observed
blocked_by_policy
requested_user_consent
allowed_live
ignored_historical_replay
failed_browser_permission
```

### Container execution domains

Container terminal modes:

| Mode | Semantics | History treatment |
| --- | --- | --- |
| `docker_exec` | New process inside running container | Command block with container/image/workdir/user metadata. |
| `docker_attach` | Attach to main process streams | Treat as live process attach, attribution may be weak. |
| `docker_logs` | Logging-driver output | Import/read model, not interactive transcript. |
| `kubectl_exec` | Remote exec into pod/container | Remote/container domain with namespace/pod/container metadata. |
| `kubectl_logs` | Pod/container logs | Log import/source, not command lifecycle. |

Data to store:

- orchestrator kind;
- namespace/project;
- pod/container ID/name;
- image digest/name if available;
- exec vs attach vs logs;
- TTY/stdin flags;
- remote cluster/context redaction state.

### Windows encoding and codepage profile

Windows terminal history should store:

```text
input_codepage
output_codepage
shell_encoding_policy
powershell_version
console_host_kind          -- conhost, windows_terminal, conpty, imported
utf8_mode
decode_errors_count
```

Why:

- cmd.exe output may use OEM code page;
- PowerShell 5.1 and PowerShell 7 encoding defaults differ;
- external native programs may emit non-UTF-8 bytes;
- replay/search/redaction can corrupt text if decoded with wrong assumptions.

Rule:

- raw bytes remain canonical for stream segments;
- decoded text projection stores encoding profile/version;
- decode errors are observable and searchable as diagnostics.

### Sleep, resume and power-event durability

Power events can interrupt:

- DB writer batch;
- WebSocket connection;
- SSH/mux session;
- export/backup;
- artifact write;
- long-running command output.

Behavior:

- on suspend/shutdown signal: flush small pending batches if safe;
- mark power event in journal;
- on resume: verify DB integrity/writer lag;
- force reconnect/replay for clients;
- mark gaps if remote/mux output advanced while local app slept;
- strict/audit mode can warn/prevent sleep during critical export/backup if OS allows it.

### Local gateway and WebSocket threat model

Gateway capabilities:

- create terminal sessions;
- send terminal input;
- resize panes;
- read output/history;
- export/import history;
- access local paths/artifacts;
- possibly start shells/processes.

That makes it a local privileged API.

Minimum gateway rules:

- bind only to loopback unless explicitly configured;
- per-run high-entropy token;
- token scoped to gateway/session;
- validate `Origin` and `Host`;
- reject wildcard CORS;
- no auth through ambient cookies alone;
- message-level authorization;
- rate limit input/control messages;
- audit failed auth/origin attempts;
- shutdown token when app exits or session closes.

Handshake shape:

```text
expected_origin
host_header
token_id
token_scope
runtime_slug
session_id
expires_at_ms
client_fingerprint
```

### DNS rebinding and private-network access

Attack shape:

```text
attacker.example loads in browser
  -> DNS later resolves attacker.example to 127.0.0.1
  -> page attempts WebSocket/HTTP to local gateway
  -> browser may send request with attacker Origin/Host
```

Defenses:

- Origin allowlist;
- Host allowlist;
- random unguessable token in query/header;
- bind to `127.0.0.1`/`::1` only;
- no `Access-Control-Allow-Origin: *` on control API;
- reject browser credentials/cookies as sole auth;
- support/handle Private Network Access preflights where applicable.

### Named pipe and local IPC alternative

Named pipes can reduce browser exposure if only native helper talks to runtime, but they are not magic.

Need:

- per-user security descriptor;
- deny remote pipe access unless explicit;
- randomized/per-session pipe name where possible;
- client identity check;
- impersonation policy;
- audit connection attempts;
- same message authorization as TCP/WebSocket gateway.

Decision:

- Web browser UI still needs a transport bridge.
- Native desktop shell can use named pipe/Unix socket internally.
- The security model should be transport-independent: token + identity + scope + audit.

### Archive import and debug bundle safety

History bundles may contain:

- manifest;
- event streams;
- raw segments;
- artifacts;
- screenshots/media;
- diagnostic logs;
- search/projection caches.

Import pipeline:

```text
receive bundle
  -> size cap
  -> quarantine directory
  -> manifest parse
  -> path normalization
  -> reject absolute/parent/symlink escapes
  -> checksum verification
  -> schema validation
  -> redaction scan
  -> import as inert historical data
```

Rules:

- never extract archive directly into live store;
- never trust archive filenames;
- entry count and decompressed size limits;
- no auto-execution of imported commands/scripts;
- unknown files stay quarantined;
- import report is saved.

### Inert transcript/export viewer

Rendering history as HTML is dangerous because output can contain:

- ANSI hyperlinks;
- fake prompts;
- HTML-looking text;
- URLs;
- filenames;
- terminal media;
- clipboard escape requests.

Safe viewer defaults:

- escape all terminal text;
- no inline scripts;
- CSP `default-src 'none'` style baseline where possible;
- sandboxed iframe for preview;
- disable navigation/open by default;
- user action required for links/files/media;
- no service worker registration in transcript viewer;
- clear browser caches when redaction/delete policy requires it.

### Desktop webview security

If shipping Electron/Tauri/native shell:

- disable Node/browser dangerous APIs in untrusted contexts;
- allowlist navigation and external opens;
- isolate terminal renderer from native command APIs;
- expose narrow commands with explicit permissions;
- validate all IPC payloads;
- keep gateway token out of persistent web storage;
- rotate token on restart/session close;
- log denied IPC/navigation attempts.

### Atomic artifact writes and file replacement

Terminal history inevitably creates external artifacts:

- compressed stream chunks;
- media payloads;
- export bundles;
- debug bundles;
- search/shard files;
- manifests and checkpoints.

If these are outside SQLite, atomicity must be designed explicitly. A reliable write flow:

```text
create temp file in same directory
write bytes
flush/fsync file
verify length/checksum
atomic replace/rename final path
flush/fsync parent directory where supported
commit DB manifest row
background verifier checks DB manifest vs filesystem
```

Top 3 options:

1. **Keep all artifacts inside SQLite BLOBs** - 🎯 7   🛡️ 8   🧠 5  
   Объем: примерно 600-1200 строк.  
   Хорошо для простоты, backup consistency and transactionality. Плохо для очень больших media/chunks, streaming and incremental cold storage.

2. **External artifacts with temp+fsync+atomic replace+manifest** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1200-2400 строк.  
   Лучший базовый вариант: SQLite хранит truth/manifest, файловая система хранит тяжелые blobs. Нужно отдельно тестировать Windows replace/share semantics.

3. **Content-addressed external artifacts with verifier** - 🎯 8   🛡️ 10   🧠 9  
   Объем: примерно 2200-4200 строк.  
   Самый сильный вариант для масштаба: artifact path derives from hash, manifest immutable, background GC/verifier. Сложнее deletion/retention/legal hold.

Рекомендация: начинать с варианта 2, но сразу проектировать manifest так, чтобы позже перейти к content-addressed storage.

### File locks and single-writer coordination

File locks are useful, but dangerous as an architecture foundation.

They should protect:

- single writer process for external artifacts;
- compaction/export job ownership;
- "only one repair job at a time";
- crash recovery mutex.

They should not decide:

- whether a session exists;
- which events are durable;
- whether a command block is complete;
- who can read/export/share.

Правильная модель:

- DB job rows are source of truth;
- file locks are a runtime guard;
- stale locks are detected by owner heartbeat/process identity;
- Windows and Unix lock behavior have separate tests;
- network/shared folders are either unsupported for active store or run in strict degraded mode.

### Redaction engine and ReDoS safety

Secret redaction over terminal history is an attack surface. Terminal output is attacker-controlled data: a command can print megabytes of crafted text, ANSI escapes, huge lines and secret-like payloads.

Safe redaction pipeline:

```text
normalize bounded chunk
strip/control-classify terminal sequences
exact multi-pattern scan
linear regex scan
entropy/provider validators
record rule metrics
write redacted projection
preserve raw only if policy allows it
```

Rules:

- no unreviewed backtracking regex in hot path;
- rule has ID, version, severity, tests and owner;
- each rule has max input bytes/time budget;
- every match records rule ID and projection range;
- raw secret exposure creates lifecycle event, not just hidden text;
- export/AI/share use redacted projection by default.

Top 3 redaction strategies:

1. **Regex-only redaction with Rust `regex`** - 🎯 7   🛡️ 7   🧠 4  
   Объем: примерно 500-1000 строк.  
   Быстро начать, но слабее для provider-specific validation and exact multi-pattern scale.

2. **Layered scanner: exact patterns + linear regex + validators** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1400-2800 строк.  
   Лучший baseline: fast prefixes, safe regex, provider validators, rule metrics.

3. **Pluggable scanning engine with hot/cold profiles** - 🎯 8   🛡️ 10   🧠 9  
   Объем: примерно 2600-5200 строк.  
   Максимально масштабируемо, но для первой версии слишком много surface area.

Рекомендация: вариант 2. Он достаточно мощный для терминала и не превращает redaction в отдельную платформу раньше времени.

### Authorization policy boundary

History actions are security-sensitive:

- open saved session;
- view pane transcript;
- copy command;
- rerun command;
- export bundle;
- share link/token;
- attach AI context;
- delete/purge;
- recover/quarantine import.

The authorization boundary should be centralized. UI can ask for permission state, but final decision must happen server/runtime side.

Policy input should include:

- subject: user/device/workspace/role;
- object: session/pane/command/artifact/export;
- action: view/export/share/rerun/delete/attach_ai;
- environment: local/remote, private mode, redaction state, retention hold, device trust, time.

Top 3 policy approaches:

1. **Typed Rust policy functions only** - 🎯 8   🛡️ 8   🧠 5  
   Объем: примерно 700-1500 строк.  
   Простая первая версия, хорошо тестируется, но policy spread risk grows over time.

2. **Central policy service API with typed Rust rules now** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1400-2600 строк.  
   Лучший старт: одна authorization boundary, audit decisions, later Cedar/OPA-compatible model.

3. **Embed full policy engine immediately** - 🎯 6   🛡️ 9   🧠 9  
   Объем: примерно 3000-6000 строк.  
   Powerful for enterprise, but high integration complexity before product semantics settle.

Рекомендация: вариант 2. Сделать свой typed policy boundary сейчас, но модель subject/object/action/environment держать совместимой с ABAC/Cedar/OPA patterns.

### Capability grants for sharing

Share/export should not copy broad account permissions into a link. Correct pattern is an attenuated grant:

```text
grant can view session X
only pane Y
only redacted transcript
expires at T
cannot export raw
cannot rerun
can be revoked
audit every use
```

Grant properties:

- token stored only as hash in DB;
- actions are explicit;
- resources are explicit;
- caveats include expiry, audience, device/workspace, redaction profile;
- revoke is immediate for server-checked grants;
- every policy decision stores policy version and inputs.

This keeps future multi-user/shared sessions sane. Without it, "share session" often becomes accidental read access to everything attached to the workspace.

### Windows path identity and artifact store hardening

For Terminal Persistence v2, artifact paths are security boundaries:

- stream chunk files;
- media payloads;
- exported bundles;
- debug import quarantine;
- derived search shards;
- temporary write files.

The path string is only a user-facing name. A safe storage layer should separate:

- display name;
- generated storage name;
- canonical/final path;
- opened handle identity;
- volume/root identity;
- artifact manifest identity.

Top 3 approaches:

1. **String path validation only** - 🎯 5   🛡️ 4   🧠 3  
   Объем: примерно 300-700 строк.  
   Simple, but too fragile for Windows: reserved names, ADS, symlink/junction race, long paths and case sensitivity create edge cases.

2. **Canonical root guard + generated artifact names** - 🎯 9   🛡️ 8   🧠 6  
   Объем: примерно 900-1800 строк.  
   Good baseline: never use command/output text as filename, resolve root, reject traversal/reserved names/ADS/reparse points by policy, store normalized display name separately.

3. **Handle/file-id verified writes for critical artifacts** - 🎯 8   🛡️ 10   🧠 8  
   Объем: примерно 1600-3200 строк.  
   Strongest for reliability: open file, inspect final path/file ID/volume, verify it remains under expected store, then commit manifest. More platform-specific code.

Рекомендация: вариант 2 for all artifacts, вариант 3 for manifests, snapshots, export bundles and writer-owned chunks.

### Reparse point and symlink policy

Default policy for active history store:

- no user-controlled symlinks inside live store;
- no following reparse points during import/extract;
- quarantine symlink/reparse entries in debug bundles;
- allow symlink only as explicit external attachment reference, never as artifact storage path;
- verify critical paths after open where platform APIs allow it.

Why:

- symlink/junction can redirect writes outside the store;
- path can be swapped between validation and open;
- mount points can cross volume boundaries;
- backup/export can accidentally include more than intended.

### Windows filename and export naming policy

Export names should be stable and safe:

```text
session-2026-04-29T10-30-00Z/
  manifest.json
  panes/
    pane-0001.transcript.ndjson
    pane-0001.redacted.txt
  artifacts/
    sha256-...bin
```

Not safe:

- command text as filename;
- cwd path as directory structure;
- prompt title as filename;
- raw remote hostname/user as path segment;
- filenames with `CON`, `NUL`, trailing dot/space or colon stream syntax.

Display metadata can preserve original command/cwd/title, but storage names should be generated IDs.

### File watcher and USN strategy

Watchers are useful for operational health:

- detect artifact deletion/tampering;
- detect external cleanup;
- catch verifier-needed changes;
- accelerate repair scans.

But watcher events are not complete truth:

- buffers can overflow;
- process can be offline;
- events can coalesce;
- network/removable filesystems differ;
- USN journal can reset or not cover every target filesystem.

Rule: watcher/USN only marks "needs rescan". DB manifest + checksum/file identity verifier decides final state.

### Rust path handling rule

Operational paths:

- use `Path`/`PathBuf`/`OsStr`;
- avoid lossy UTF-8 roundtrip;
- keep display string separate;
- serialize path refs through explicit encoding/escaping;
- do not compare display paths for security.

The persistence DB should store:

- generated storage key;
- platform path bytes/ref if needed;
- display path separately;
- normalized/canonical/final path metadata;
- verification state.

### Event delivery semantics and idempotency

Terminal history has multiple delivery paths:

- PTY/mux output to runtime;
- runtime to browser;
- browser input to runtime;
- runtime journal writer to DB;
- DB outbox to projection/search/export/sync workers;
- backup/sync upload to object store.

Each path has different failure modes. The architecture should not claim "exactly once delivery" at transport level. It should provide exactly-once effects where needed through:

- per-session/per-pane stream IDs;
- monotonically increasing sequence numbers from writer authority;
- idempotency keys for user operations;
- deduplication windows/tables;
- ack/replay protocol for browser clients;
- durable outbox for workers;
- replayable journal as source of truth.

Top 3 approaches:

1. **Trust WebSocket order and keep only UI state** - 🎯 3   🛡️ 3   🧠 3  
   Объем: примерно 200-500 строк.  
   Works in demos, fails on reconnect, tab reload, server restart and duplicate submit.

2. **Per-pane seq/ack/replay + idempotent UI actions** - 🎯 9   🛡️ 8   🧠 7  
   Объем: примерно 1200-2600 строк.  
   Strong baseline: browser can ask for missed seq ranges and user actions can be retried safely.

3. **Full event bus with producer transactions and consumer offsets** - 🎯 7   🛡️ 9   🧠 9  
   Объем: примерно 3000-7000 строк.  
   Powerful for distributed deployments, but too heavy before local persistence semantics settle.

Рекомендация: вариант 2 now, with schema shaped so variant 3 can be introduced later if terminal-platform becomes multi-node.

### Transactional outbox and inbox workers

Projection/search/export/sync should not run as side effects after commit without durable intent.

Correct write pattern:

```text
begin transaction
insert journal events / stream segment manifest
insert or update command block
insert outbox messages for projection/search/export/sync
commit
worker claims outbox row
worker executes idempotently
worker records result/attempts
```

For inbound replicated events, use inbox/dedup:

```text
incoming source_id + event_id
check inbox
apply if unseen
record inbox state
emit local outbox if needed
```

Rules:

- workers are idempotent;
- retries are expected;
- poison messages move to quarantine;
- every outbox row has scope and causal event range;
- derived layers can be rebuilt from canonical journal.

### Snapshot manifest and lineage model

Terminal restore snapshots should be manifests, not magical screen blobs.

Manifest should answer:

- which session/pane/backend;
- which journal sequence range;
- which projection version;
- which terminal parser/render version;
- which artifact chunks;
- which redaction/search profile;
- parent snapshot/checkpoint;
- checksum/hash chain state.

Top 3 snapshot styles:

1. **Latest screen snapshot only** - 🎯 4   🛡️ 4   🧠 3  
   Объем: примерно 300-700 строк.  
   Fast but weak: cannot explain history, cannot rebuild correctly, loses lineage.

2. **Periodic snapshot manifests over journal ranges** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1400-2800 строк.  
   Best baseline: fast restore and full replay correctness.

3. **Table-format style manifest tree with compaction and branches** - 🎯 8   🛡️ 10   🧠 9  
   Объем: примерно 3000-6500 строк.  
   Strong for sync/cold storage/branching, but should evolve from option 2.

Рекомендация: option 2, but include parent/high-water fields from day one.

### Object store and immutable artifact sync

If history sync/backup uses object storage:

- never treat bucket/container as POSIX filesystem;
- upload immutable content-addressed artifacts;
- write small manifests last;
- store object version/ETag/generation where provider exposes it;
- make deletion a tombstone/reachability problem;
- test lifecycle/retention/legal hold behavior.

Safe flow:

```text
upload artifact chunks
verify returned version/etag/checksum
upload manifest referencing chunks
record sync checkpoint
only then mark backup/sync complete
```

For privacy:

- raw transcript chunks must be encrypted/redacted according to policy;
- object versions can keep deleted data alive;
- retention/legal hold can override user deletion;
- export/sync UI must show this honestly.

### Backup restore drills

A backup that was never restored is only a hope.

Minimum restore drill:

```text
create temp restore directory
restore DB/artifact set
open DB defensively
verify schema/application_id/user_version
verify artifact manifests/checksums
rebuild projections/search
sample replay panes
report missing/corrupt/redacted/held data
delete temp restore according to policy
```

Frequency:

- automatic lightweight drill after backup format changes;
- periodic sample drill for long-lived stores;
- manual support drill button;
- drill results stored in DB.

### Sync conflict and branch strategy

Terminal transcript is not collaborative text.

Safe conflict model:

- append-only stream has one writer authority at a time;
- duplicate events dedupe by source/event ID;
- divergent sequences create branch/conflict records;
- UI shows local and remote branches;
- user can keep both, archive one, or promote one;
- raw byte streams are not auto-merged.

CRDT use later:

- session notes;
- command bookmarks;
- layout preferences;
- labels/tags;
- collaborative comments.

Not for:

- PTY byte stream ordering;
- process lifecycle events;
- command output attribution;
- redaction/delete lineage.

### Search architecture for long-lived terminal history

Terminal search has at least five different products:

- command search;
- output text search;
- file/path/token search;
- replay range lookup;
- analytics/debug queries.

They should not all hit one table. A scalable model:

```text
canonical journal / stream segments
  -> chunk catalog
  -> hot command index
  -> hot transcript FTS
  -> warm Tantivy/FTS shards
  -> cold object-store searchable chunks
  -> analytics export/read model
```

Top 3 approaches:

1. **One SQLite FTS table for everything** - 🎯 6   🛡️ 6   🧠 4  
   Объем: примерно 500-1200 строк.  
   Good first prototype, but sensitive duplication, rebuild pain and large-history performance issues grow fast.

2. **Chunk catalog + hot FTS + rebuildable warm shards** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1800-3600 строк.  
   Best baseline: canonical chunks remain truth, search projections can be rebuilt, query planner can skip chunks.

3. **Object-store search engine style cold tier** - 🎯 8   🛡️ 10   🧠 9  
   Объем: примерно 3500-8000 строк.  
   Powerful for enterprise/long retention, but only after option 2 proves the data model.

Рекомендация: вариант 2 now, schema-compatible with option 3 later.

### Label and cardinality policy

Safe indexed labels:

- `workspace_id`;
- `session_kind`;
- `backend_kind`;
- `execution_domain`;
- `shell_kind`;
- `trust_level`;
- `exit_status_class`;
- `redaction_profile_id`;
- `parser_version`;
- `private_mode_state`.

Unsafe labels:

- full command text;
- full cwd;
- hostname/user;
- PID;
- git branch names;
- generated temp paths;
- token-like strings;
- container pod names if high cardinality;
- raw prompt/title.

Rule: high-cardinality or sensitive values can be searchable attributes/snippets after policy/redaction, but not global index labels.

### Chunk catalog and prefilter strategy

Each chunk should answer:

- which session/pane/stream;
- seq range;
- wall time and monotonic range;
- byte range and decompressed size;
- compression codec;
- checksum/hash;
- redaction profile;
- parser/projection version;
- min/max timestamps;
- token summary or bloom filter;
- command block range;
- artifact references;
- storage tier.

Query flow:

```text
parse query
authorize scope
choose labels/time/seq filters
select candidate chunks
apply bloom/token/minmax prefilters
scan FTS/Tantivy/raw projection
verify matches against authorized redacted content
return snippets with provenance
record query metrics
```

### Hot, warm and cold history tiers

Recommended tiers:

| Tier | What lives there | Target behavior |
| --- | --- | --- |
| Hot | active/recent sessions, command blocks, recent transcript FTS | instant restore/search |
| Warm | compressed chunks, rebuildable search shards, snapshots | seconds-level search/replay |
| Cold | object-store backup chunks, immutable manifests, sparse/searchable snapshots | explicit slower search |

Policies:

- moving to warm/cold is recorded as lifecycle event;
- search UI shows tier and cost/latency;
- cold search can return partial results;
- private sessions may never leave hot encrypted local store;
- legal hold can pin chunks across tiers.

### Search index versioning and repair

Every index needs:

- index kind;
- tokenizer version;
- redaction profile;
- parser/projection version;
- source seq range;
- source chunk refs;
- build state;
- merge/optimize state;
- corruption/rebuild state.

Search index is disposable. It can be deleted and rebuilt from canonical journal/chunks. Therefore:

- no unique data only inside search index;
- no raw sensitive text in index unless policy allows;
- stale index returns freshness warning;
- tokenizer changes trigger rebuild jobs;
- redaction changes invalidate snippets/indexes.

### Query budgets and partial results

Every search should have:

- max chunks scanned;
- max decompressed bytes;
- timeout;
- max snippets;
- tier allowlist;
- raw/redacted permission;
- cancellation token;
- resume cursor.

User-visible states:

- complete;
- partial because timeout;
- partial because cold tier skipped;
- blocked by policy;
- stale index;
- rebuild required;
- degraded because chunks missing.

### AI context safety for terminal history

Terminal history can feed AI in several ways:

- selected command blocks;
- recent pane transcript;
- search results/snippets;
- failure logs;
- exported debug bundle;
- MCP resources/tools;
- "explain this terminal" summaries;
- "rerun/fix this command" workflows.

Every path has the same core risk: terminal output is untrusted text that can contain instructions to the model.

Safe context package:

```text
trusted user request
selected command blocks with IDs
untrusted terminal output as data
redaction report
source ranges
trust/confidence labels
allowed tools/actions
forbidden actions
approval requirements
```

Unsafe package:

```text
paste entire transcript into one prompt
ask model to decide what is instruction vs output
allow terminal/file/share tools broadly
hide redaction findings
execute suggested commands automatically
```

Top 3 approaches:

1. **System prompt says "ignore malicious output"** - 🎯 4   🛡️ 3   🧠 3  
   Объем: примерно 100-300 строк.  
   Better than nothing, but not an architecture. Prompt text cannot be the only boundary.

2. **Structured context + deterministic tool/action policy** - 🎯 9   🛡️ 9   🧠 7  
   Объем: примерно 1400-3000 строк.  
   Best baseline: context is typed, output is untrusted, tool scope is explicit, risky actions require approval.

3. **Isolated agent sandbox + continuous red-team evals** - 🎯 8   🛡️ 10   🧠 9  
   Объем: примерно 3200-7000 строк.  
   Strongest for agentic workflows, but should build on option 2.

Рекомендация: option 2 now, with red-team fixture format from option 3.

### Context packaging and provenance

Each AI context item should carry:

- source type: user command, terminal output, shell marker, search snippet, summary, file artifact;
- trust level;
- redaction profile;
- seq range;
- command block ID;
- pane/session;
- whether it is instruction-eligible;
- whether it is output/data-only;
- whether raw text was truncated;
- sensitive findings count.

Rules:

- command output is data-only by default;
- search snippets are data-only by default;
- summaries are marked derived and cite source ranges;
- AI may not invent provenance;
- answer UI can link back to exact command/output range.

### AI action gates

Actions that require deterministic approval/policy:

- send terminal input;
- rerun command;
- paste multi-line command;
- create/delete/share/export history;
- attach history to AI context;
- read raw unredacted chunks;
- open links/files from transcript;
- modify shell integration scripts;
- install packages suggested by terminal output.

Approval record should include:

- user-visible diff/command;
- exact source context used;
- prompt-injection findings;
- policy decision;
- tool scope;
- expiration;
- final user confirmation.

### Prompt-injection detection for terminal output

Detection inputs:

- phrases targeting model/system/developer instructions;
- hidden Unicode/bidi/confusables;
- ANSI/OSC hyperlinks;
- Markdown/code fences that look like user instructions;
- base64/encoded instructions;
- fake approval text;
- "copy this command" suggestions;
- malicious MCP/tool descriptions emitted in logs.

Detection outputs:

- risk finding with source range;
- severity/confidence;
- downgrade context trust;
- strip or quote risky content in AI package;
- require approval for actions derived from affected ranges.

Important: detection is advisory. It reduces risk but never replaces deterministic action policy.

### AI red-team fixtures

Terminal history tests should include:

- `npm test` output containing "ignore previous instructions";
- `curl` output containing malicious Markdown instructions;
- build log with ANSI hyperlink to local file;
- package manager output suggesting a destructive command;
- fake shell prompt asking model to run command;
- hidden bidi text that changes visible command meaning;
- MCP server log advertising changed tool permissions;
- encoded instruction in base64/hex;
- output that asks to export secrets.

Each fixture should assert:

- context item is data-only;
- risky range is flagged;
- AI action gate blocks automatic execution;
- approval UI shows source and risk;
- redaction stays applied.

### Reliability proof stack

Testing Terminal Persistence v2 should be layered:

```text
unit tests
property tests
concurrency schedule tests
model checks for small protocols
deterministic simulation
fault-injection integration tests
crash/power-loss tests
backup restore drills
chaos experiments
release checklist gates
```

This stack is not optional for "never lose history" claims. A feature can launch with a limited guarantee, but the guarantee must match the test layer actually in place.

### Core persistence invariants

Minimum invariants:

- committed journal events are ordered per stream;
- no duplicate command submit for same idempotency key;
- every command block output range points to existing journal/chunks;
- every snapshot high-water seq is covered by journal or manifest;
- every external artifact referenced by DB has checksum/identity verification;
- outbox row eventually reaches done or quarantined;
- restore never shows unverified content as live truth;
- deletion creates tombstone before physical purge;
- redacted export never includes raw ranges denied by policy;
- AI action cannot execute from data-only terminal output.

Each invariant should have:

- formal statement;
- test fixture;
- simulation checker;
- production diagnostic query;
- user-facing degraded state when violated.

### Deterministic terminal persistence simulator

Simulate these actors:

- PTY/native backend;
- zellij/tmux backend;
- browser client;
- history writer;
- SQLite/DB layer;
- external artifact store;
- outbox workers;
- search/index workers;
- backup/sync target;
- AI context packager.

Simulate these faults:

- process crash at every await/commit boundary;
- DB busy/locked/corrupt/rollback;
- write/flush/rename failure;
- partial artifact write;
- browser reconnect and duplicate actions;
- network delay/drop/reorder;
- clock jump/suspend/resume;
- outbox worker crash after side effect;
- redaction rule timeout;
- path/reparse/identity mismatch.

Every simulation run stores:

```text
seed
scenario version
actor schedule
fault plan
initial DB fixture
final DB/export bundle
invariant failures
minimal reproduction if available
```

Top 3 implementation options:

1. **Ad-hoc integration fault tests only** - 🎯 5   🛡️ 5   🧠 5  
   Объем: примерно 700-1600 строк.  
   Useful but not enough: failures are hard to reproduce and coverage is narrow.

2. **Seeded deterministic simulator for persistence protocols** - 🎯 9   🛡️ 9   🧠 8  
   Объем: примерно 2400-5200 строк.  
   Best baseline: replayable bugs, clear invariant checks, good fit for writer/outbox/reconnect/storage.

3. **Full distributed/system simulator with virtual filesystem and browser model** - 🎯 8   🛡️ 10   🧠 10  
   Объем: примерно 6000-14000 строк.  
   Powerful long-term, but too heavy before the first protocol simulator is working.

Рекомендация: option 2, with a small virtual filesystem and deterministic network/clock first.

### Fault injection matrix

Critical fault points:

| Boundary | Faults |
| --- | --- |
| PTY/mux stream | split UTF-8, huge burst, stalled output, duplicate close, resize storm |
| Browser transport | reconnect, duplicate submit, stale ack, hidden tab, offline queue |
| DB writer | busy, rollback, commit error, migration interruption, WAL checkpoint failure |
| Artifact writer | short write, flush fail, rename fail, cross-volume mismatch, sharing violation |
| Outbox worker | crash before side effect, crash after side effect, poison message, retry storm |
| Redaction | timeout, ReDoS attempt, rule panic, profile change mid-export |
| Search | stale index, corrupt shard, partial cold scan, tokenizer migration |
| Backup/sync | upload duplicate, missing chunk, object version conflict, tombstone race |
| AI context | injection payload, tool drift, approval expired, policy denied |

For each fault:

- expected invariant;
- expected user-facing state;
- retry/recovery policy;
- telemetry event;
- regression fixture.

### Model checking candidates

Good candidates for TLA+/Apalache/Stateright:

- browser seq/ack/replay;
- idempotency key lifecycle;
- single-writer ownership;
- outbox claim/retry/quarantine;
- snapshot lineage/high-water coverage;
- delete tombstone vs sync/backup purge;
- share capability grant revoke/expiry;
- search index freshness state machine;
- AI action approval expiry.

Not worth modeling formally at first:

- full terminal VT parser;
- Unicode rendering;
- real filesystem implementation;
- full browser lifecycle;
- LLM behavior.

### Release reliability gates

Every persistence release should record:

- migration up/down or forward-only compatibility result;
- link/schema/doc checks for exported formats;
- crash-safety simulation seeds passed;
- Windows path/fault fixtures passed;
- restore drill passed;
- redaction/AI-context adversarial fixtures passed;
- search rebuild passed;
- outbox lag/poison tests passed;
- backup/export import roundtrip passed;
- known degraded modes documented.

Release can still ship with gaps, but gaps must be explicit and tied to product guarantee text.

### Encryption and key management architecture

Terminal Persistence v2 should assume transcript data contains secrets:

- API keys and tokens;
- passwords typed into prompts;
- private paths and hostnames;
- build logs with env variables;
- SSH/container/cloud metadata;
- AI context snippets;
- exported debug bundles.

Encryption protects storage theft and backup leakage. It does not protect:

- compromised running process;
- user-approved export/share;
- screenshots;
- clipboard;
- logs/crash dumps;
- malware running as the same user;
- AI context after send.

Top 3 approaches:

1. **SQLCipher-only DB encryption** - 🎯 7   🛡️ 6   🧠 5  
   Объем: примерно 900-1800 строк.  
   Good first layer for SQLite pages, but insufficient for external artifacts, exports, search shards and selective cryptographic erase.

2. **Envelope encryption for DB key, artifacts, search and exports** - 🎯 9   🛡️ 9   🧠 8  
   Объем: примерно 2600-5600 строк.  
   Best baseline: SQLCipher for DB, per-artifact/per-stream DEKs, OS-wrapped KEKs, authenticated manifests and key lifecycle tables.

3. **Full multi-device E2EE with recipient keys and recovery governance** - 🎯 7   🛡️ 10   🧠 10  
   Объем: примерно 7000-16000 строк.  
   Strong for sync/collaboration, but too much before local key hierarchy and restore drills are stable.

Рекомендация: вариант 2. It gives strong local protection and prepares for option 3.

### Envelope key hierarchy

Recommended hierarchy:

```text
OS key store / passphrase / enterprise root
  -> root wrapping key reference
    -> workspace KEK
      -> DB encryption key
      -> stream/artifact DEKs
      -> search shard DEKs
      -> export bundle DEKs
      -> AI context cache DEKs
```

Rules:

- never store plaintext keys in the DB;
- store wrapped keys with key version and algorithm;
- use unique DEKs for selective deletion units;
- rotate KEKs by rewrapping DEKs;
- authenticate metadata as associated data;
- audit key unwrap/use for sensitive operations;
- make recovery profile explicit.

Associated data should include:

- artifact ID;
- session/pane/stream IDs;
- seq range;
- schema version;
- codec/compression;
- redaction profile;
- key version;
- storage tier.

### SQLCipher plus external artifact encryption

SQLCipher/SEE-style DB encryption covers SQLite pages. It does not automatically cover:

- external stream chunks;
- media files;
- object-store blobs;
- exported archives;
- derived search indexes;
- temp files outside encrypted DB;
- debug bundles copied out of the store.

Correct model:

```text
SQLite DB encrypted with DB key
DB stores encrypted artifact metadata and wrapped DEK refs
external artifacts encrypted independently
manifest authenticates artifact metadata and checksums
restore verifies DB key, artifact key and manifest before replay
```

### Crash-safe rekey and rotation

Rotation jobs should be stateful:

```text
prepare new key version
wrap new key material
rewrap selected DEKs
verify sample decrypt
commit active key version
mark old key retiring
destroy old key only after retention/rollback window
record audit event
```

Crash states:

- prepared but not active;
- partially rewrapped;
- verified but not committed;
- active but old key retained;
- destroy pending;
- failed/quarantined.

No rekey operation should be invisible or only implied by current config.

### Cryptographic erase and deletion

Selective cryptographic erase works only if data was encrypted with a destroyable key at the right granularity.

Good deletion unit examples:

- private session DEK;
- export bundle DEK;
- cold chunk DEK;
- AI context cache DEK;
- search shard DEK.

Bad deletion unit:

- one global key for all history.

Erase workflow:

```text
authorize delete
write tombstone
mark affected chunks/projections
destroy/retire selected DEKs or wrapping records
invalidate search/AI/export caches
queue physical GC
record limitations and backups/object versions still alive
```

Important limitations:

- secure delete on SSD/filesystems is not guaranteed for every copy;
- WAL/temp/pagefile/crash dumps may contain historical data unless controlled;
- object versions/backups/legal hold can retain old ciphertext;
- cryptographic erase requires key destruction evidence.

### Recovery profiles

Product needs explicit profiles:

| Profile | Recovery | Risk |
| --- | --- | --- |
| Local-only OS key | No recovery if OS/user key lost | Best privacy, worst account recovery |
| Passphrase recovery | User can restore with phrase | Weak passphrases need KDF and UX guardrails |
| OS keychain sync | Convenient across devices | Tied to platform account behavior |
| Enterprise escrow | Admin recovery possible | Higher governance and insider-risk burden |
| Per-export recipient keys | Good for sharing/export | More key UX and revocation complexity |

No profile is universally best. The UI must show the selected recovery and deletion guarantees.

## Исправленная архитектурная сводка

### Не одна история, а 6 слоев

| Слой | Что хранит | Источник истины | UI |
| --- | --- | --- | --- |
| Shell native history | То, что shell считает командами | shell files/db | optional import/reference |
| Command dock recents | Быстрые UI-команды | DB read model + local cache | bottom command dock |
| Command blocks | User-visible command lifecycle | shell integration + UI submit + journal | blocks/timeline |
| Terminal stream journal | Raw/semantic terminal facts | PTY/mux stream writer | replay/search/debug |
| Screen snapshots | Быстрая hydration | projection snapshots | restore viewport/scrollback |
| Session restore metadata | layout/backend/launch/focus | runtime/session store | saved sessions list |

Почему это важно:

- Shell history может не знать output.
- Output journal может не знать command boundaries.
- Snapshot может не знать sequence semantics.
- Command dock может быть глобальным cache, но не session truth.
- Saved session layout не равен transcript.

### Source-of-truth decisions

| Вопрос | Решение |
| --- | --- |
| Где хранить команды пользователя? | `terminal_command_blocks`, session/pane-scoped, Diesel. |
| Где хранить output? | `terminal_stream_segments`, chunked append-only, compressed optional. |
| Где хранить semantic markers? | `terminal_journal_events`, small searchable rows. |
| Где хранить viewport/scrollback snapshots? | `terminal_screen_snapshots`, periodic and on save/close. |
| Где хранить command dock recents? | DB read model + browser cache fallback. |
| Чем restore быстрее всего? | Snapshot first, then journal replay. |
| Чем restore точнее всего? | Journal + semantic events + original resize sequence. |
| Что делать с live process? | Native: false. Zellij/tmux: attach path отдельно. |
| Что делать с destructive restored commands? | Never auto-run, explicit confirmation. |
| Что делать с secrets? | Redaction + private mode + raw input off by default. |

### Trust model

Каждая command block / event должна иметь trust:

- `trusted_nonce` - shell integration marker с nonce.
- `trusted_ui_submit` - команда отправлена через наш UI.
- `shell_reported` - marker есть, nonce нет.
- `terminal_observed` - событие наблюдали через PTY/projection.
- `heuristic` - восстановлено по prompt/text эвристикой.
- `untrusted` - raw OSC/output без validation.

Правило:

- `rerun`, `copy command`, AI attachment и analytics должны учитывать trust.
- Untrusted command text нельзя запускать без подтверждения/edit.

## Целевая модель данных

Новые таблицы стоит писать через Diesel. Старый `rusqlite` оставить до отдельного рефактора.

### 1. `terminal_command_blocks`

Структурированный слой для пользователя.

```text
id
session_id
tab_id
pane_id
backend_kind
command_text
command_text_source       -- osc633, osc133_cmdline, shell_hook, ui_submit, heuristic
cwd
cwd_source                -- osc633, osc7, osc9_9, shell_hook, launch_spec, unknown
shell_program
started_at_ms
ended_at_ms
duration_ms
exit_code
status                    -- pending, running, succeeded, failed, cancelled, unknown
start_stream_seq
end_stream_seq
start_projection_seq
end_projection_seq
integration_quality
created_by                -- user, agent, system, unknown
redaction_state           -- clean, redacted, sensitive_unknown
```

Зачем отдельно:

- command history UI;
- repeat command;
- copy command/output;
- search by cwd/status/exit code;
- AI/context attachment;
- audit/debug.

### 2. `terminal_stream_segments`

High-volume raw stream. Не одна строка на байт и не один blob на всю сессию.

```text
id
session_id
pane_id
stream_kind               -- pty_input, pty_output, rendered_delta, mux_surface_delta
first_seq
last_seq
started_at_ms
ended_at_ms
payload_blob
payload_encoding          -- raw_utf8, bytes, json, bincode
compression               -- none, zstd, lz4
byte_len_uncompressed
byte_len_compressed
checksum
```

Почему segments:

- меньше SQLite rows;
- проще batch writes;
- можно сжимать chunk-level;
- можно prune по sequence ranges;
- можно replay с середины.

### 3. `terminal_journal_events`

Low-volume semantic facts.

```text
id
session_id
pane_id
command_block_id
event_seq
stream_seq
timestamp_ms
capture_layer             -- ui, shell_marker, pty, conpty, projection, mux_api, audit
kind                      -- prompt_start, prompt_end, pre_exec, command_finish, cwd, resize, clear, title, exit, marker, alt_screen_enter, alt_screen_leave
payload_json
trust                     -- trusted, shell_reported, terminal_observed, heuristic, untrusted
```

Почему отдельно от stream:

- markers ищутся часто;
- command blocks строятся из events;
- UI navigation по commands не должен сканировать megabytes output.

### 4. `terminal_screen_snapshots`

Точки быстрой hydration.

```text
id
session_id
pane_id
captured_at_ms
stream_seq
projection_seq
rows
cols
active_buffer             -- normal, alternate
screen_blob
scrollback_blob
compression
terminal_modes_json
cursor_json
screen_hash
```

Restore:

1. Найти последний snapshot перед нужной точкой.
2. Hydrate terminal projection из snapshot.
3. Replay stream/events после snapshot.
4. Пометить visible output как `restored`, пока не пришел live output.

### 5. `terminal_history_policy`

Retention и privacy должны быть data model, не hardcoded if.

```text
scope                     -- global, workspace, session, pane
max_bytes
max_days
capture_input             -- false by default for raw keystrokes
capture_output
capture_screen_snapshots
redaction_profile_id
private_mode
```

### 6. `terminal_history_maintenance`

Operational state для надежности.

```text
id
store_id
last_checkpoint_at_ms
last_integrity_check_at_ms
db_size_bytes
wal_size_bytes
writer_lag_ms
pending_segment_count
last_error_code
last_error_message
degraded_since_ms
```

Зачем:

- UI может честно показать `History degraded`.
- Тесты могут проверять writer lag и checkpoint behavior.
- Диагностика Windows/SQLite locked/disk full становится проще.

### 7. `terminal_history_deletions`

Deletion/tombstone слой на будущее.

```text
id
scope                    -- block, pane, session, workspace, all
target_id
deleted_at_ms
reason                   -- user_clear, retention, privacy, corruption, admin_policy
applies_to_commands
applies_to_output
applies_to_snapshots
sync_state               -- local_only, pending_sync, synced
```

Зачем:

- clear history должен быть проверяемым событием.
- Если позже появится sync/cloud, без tombstones deletion будет ненадежным.
- Можно объяснить пользователю, что именно удалено.

### 8. `terminal_search_chunks`

Derived search/read model.

```text
id
session_id
pane_id
command_block_id
source_segment_id
source_snapshot_id
start_stream_seq
end_stream_seq
parser_version
redaction_version
text
text_hash
created_at_ms
index_state              -- fresh, stale, rebuilding, failed
```

Зачем:

- быстрый поиск по transcript;
- FTS5 можно подключить поверх этой таблицы;
- raw journal остается source of truth;
- index можно пересобрать после parser/redaction изменений.

### 9. `terminal_history_exports`

Контроль export/share/debug.

```text
id
scope                    -- block, pane, session, workspace
target_id
format                   -- markdown, asciicast, plain_text, json, debug_bundle
created_at_ms
redaction_profile_id
included_commands
included_output
included_timing
included_raw_bytes
output_path
checksum
```

Зачем:

- export должен быть audit-able;
- sensitive export требует redaction;
- debug bundles должны быть воспроизводимыми.

### 10. `terminal_media_artifacts`

Опциональный слой для image/rich protocols.

```text
id
session_id
pane_id
source_segment_id
stream_seq
media_kind               -- kitty_graphics, iterm2_image, sixel, osc8_link, unknown
mime_type
byte_len
content_hash
storage_ref
redaction_state
created_at_ms
```

Зачем:

- не раздувать stream segments повторяющимися images;
- применять redaction/delete к media отдельно;
- export может ссылаться на artifact;
- replay может решать, показывать media или placeholder.

### 11. `terminal_integrity_chain`

Для audit/strict profile.

```text
id
store_id
session_id
pane_id
segment_id
segment_hash
previous_hash
chain_hash
algorithm
created_at_ms
signature
```

Зачем:

- detect tampering;
- audit/compliance mode;
- debug "journal был изменен после записи";
- не обязательно для default developer mode.

### 12. `terminal_input_events`

Debug/forensic слой для input metadata. По умолчанию не хранит raw sensitive text без policy.

```text
id
session_id
pane_id
stream_seq
timestamp_ms
input_source             -- typed, paste, ui_submit, mouse_reporting, programmatic, restored_confirmation
encoding_protocol        -- legacy, csi_u, kitty_keyboard, modify_other_keys, unknown
key_or_event_kind
modifier_state
byte_len
payload_redacted
payload_hash
privacy_state
```

Зачем:

- debug keyboard/protocol bugs;
- distinguish paste vs typing;
- mouse reporting provenance;
- do not rely on raw PTY bytes only;
- keep raw input off by default.

### 13. `terminal_capability_profiles`

Снимок capability context для replay/restore.

```text
id
session_id
pane_id
captured_at_ms
term
shell_program
backend_kind
pty_kind                 -- conpty, unix_pty, mux_surface, imported
keyboard_protocol
mouse_reporting_enabled
bracketed_paste_enabled
shell_integration_quality
graphics_protocols_json
unicode_version
cell_width_policy
parser_version
```

Зачем:

- объяснять качество replay;
- debug Windows/ConPTY/zellij issues;
- version derived read models;
- avoid false confidence.

### 14. `terminal_client_views`

View/client-specific state, especially for mux/remote.

```text
id
session_id
pane_id
client_id
client_kind              -- browser, desktop, tmux_client, zellij_client, remote_readonly
attached_at_ms
detached_at_ms
rows
cols
scroll_position
read_only
can_input
can_copy
can_export
```

Зачем:

- distinguish pane truth from viewer viewport;
- read-only sharing;
- multi-client restore;
- do not mix local UI scrollback with durable pane transcript.

### 15. `terminal_remote_contexts`

Remote/mux identity context.

```text
id
session_id
pane_id
context_kind             -- local, ssh, wsl, container, tmux, zellij, mosh, unknown
host
user
cwd
cwd_source
cwd_trust
redaction_state
captured_at_ms
```

Зачем:

- cwd/host privacy;
- command replay in correct context;
- search/filter by context;
- explain why native vs mux restore guarantees differ.

### 16. `terminal_projection_runs`

Track derived read model builds.

```text
id
projection_name           -- command_blocks, search_chunks, screen_snapshots, accessibility_timeline
projection_version
parser_version
redaction_version
started_at_ms
finished_at_ms
status                    -- running, succeeded, failed, stale
source_start_seq
source_end_seq
error_message
```

Зачем:

- search/block projections can be rebuilt;
- UI can show stale/degraded derived views;
- parser upgrades can schedule rebuilds;
- tests can assert projection coverage.

### 17. `terminal_history_schema_versions`

Event/schema compatibility layer.

```text
id
schema_name
schema_version
applied_at_ms
compatible_from_version
requires_projection_rebuild
notes
```

Зачем:

- long-lived history survives releases;
- migrations can mark projections stale;
- debug bundles can explain format versions.

### 18. `terminal_integrity_checkpoints`

Audit checkpoint layer.

```text
id
store_id
session_id
up_to_segment_id
up_to_event_seq
root_hash
algorithm
signed_by
signature
created_at_ms
verification_state
```

Зачем:

- validate tamper-evident history;
- export audit bundles;
- strict mode can require recent checkpoint.

### 19. `terminal_command_domains`

Nested command/REPL/app domain tracking.

```text
id
session_id
pane_id
parent_domain_id
domain_kind              -- shell, repl, ssh, container, database_repl, app_prompt, unknown
program
started_at_seq
ended_at_seq
started_at_ms
ended_at_ms
integration_quality
history_source           -- shell_integration, app_history, heuristic, none
trust
```

Зачем:

- shell command `python` can contain inner REPL history;
- avoid fake command blocks inside REPL;
- allow future integrations for psql/python/node/ipython;
- communicate fidelity honestly.

### 20. `terminal_process_correlations`

Optional OS/process audit enrichment.

```text
id
session_id
pane_id
command_block_id
process_id
parent_process_id
executable
command_line
source                   -- windows_4688, sysmon, linux_audit, backend_spawn, heuristic
started_at_ms
ended_at_ms
trust
redaction_state
```

Зачем:

- process tree view;
- distinguish shell built-ins from subprocesses;
- optional audit integrations;
- not required for normal history.

### 21. `terminal_artifact_store`

Metadata for large external/content-addressed artifacts.

```text
id
content_hash
storage_kind             -- sqlite_blob, external_file, compressed_external_file
storage_ref
byte_len
compression
encryption_state
created_at_ms
last_referenced_at_ms
delete_state
```

Зачем:

- media/huge logs dedupe;
- external storage lifecycle;
- backup/delete consistency;
- keep stream segment rows bounded.

### 22. `terminal_redaction_findings`

Records redaction decisions without storing raw secret.

```text
id
session_id
pane_id
source_table
source_id
source_seq
finding_kind             -- token, password, private_key, url_secret, env_secret, cwd_sensitive, unknown
rule_id
rule_version
redaction_profile_id
confidence
action                   -- redacted, flagged, skipped, failed
created_at_ms
```

Зачем:

- explain what was redacted;
- re-scan after rule updates;
- avoid storing secret itself;
- audit export/AI safety.

### 23. `terminal_backup_records`

Backup/export consistency records.

```text
id
store_id
backup_kind              -- sqlite_backup, debug_bundle, user_export, automatic_snapshot
started_at_ms
finished_at_ms
status
included_db
included_wal_checkpoint
included_artifacts
included_search_index
root_hash
output_ref
error_message
```

Зачем:

- backup is not file copy;
- artifact store consistency;
- debug support;
- verify restore from backup.

### 24. `terminal_storage_locations`

Track storage roots and risk.

```text
id
store_id
path
storage_kind             -- local_disk, cloud_synced, network_share, removable, unknown
supports_locks
supports_atomic_rename
last_checked_at_ms
risk_level
warning_message
```

Зачем:

- warn about OneDrive/Dropbox/SMB/NFS-style risk;
- enforce strict mode location rules;
- diagnostics for corruption/locking issues.

### 25. `terminal_store_pragmas`

Effective SQLite/runtime store settings.

```text
id
store_id
journal_mode
synchronous
foreign_keys_enabled
busy_timeout_ms
wal_autocheckpoint_pages
journal_size_limit_bytes
transaction_mode
configured_at_ms
profile                  -- best_effort, strict, test
```

Зачем:

- diagnostics can show actual durability profile;
- tests can assert connection setup;
- strict mode can refuse unsafe settings.

### 26. `terminal_compression_profiles`

Compression policy and compatibility.

```text
id
profile_name
algorithm                -- none, zstd, zstd_seekable, lz4
level
segment_max_bytes
hot_retention_ms
cold_after_ms
created_at_ms
```

Зачем:

- segment compression is policy-driven;
- future cold storage migration;
- replay/export can check compatibility.

### 27. `terminal_secret_storage_refs`

References to OS-backed secret material. This table stores metadata, not secret bytes.

```text
id
profile_name
provider                 -- windows_dpapi, windows_credential_manager, macos_keychain, secret_service, file_dev_only
scope                    -- user, machine, session
purpose                  -- db_encryption_key, sync_token, export_key
secret_ref
created_at_ms
rotated_at_ms
last_verified_at_ms
status                   -- active, rotated, missing, inaccessible, dev_only
```

Зачем:

- history encryption can rotate keys;
- startup can verify secret accessibility;
- DB backup/export does not accidentally include raw keys;
- dev/test storage is visibly unsafe and cannot be confused with production.

### 28. `terminal_writer_health_samples`

Time-series-ish health samples. Can be local metrics table or exported to telemetry sink.

```text
id
session_id
pane_id
sampled_at_ms
writer_queue_depth
writer_lag_ms
db_batch_latency_ms
db_busy_ms
wal_size_bytes
last_committed_stream_seq
last_projected_seq
dropped_segment_count
degraded_reason
```

Зачем:

- debug "history disappeared" reports;
- prove SLOs;
- detect slow DB/checkpoint/compression;
- show honest UI badge when history is degraded.

### 29. `terminal_chaos_runs`

Records for reliability tests and local QA fixtures.

```text
id
scenario_name
backend_kind              -- native, zellij, tmux, imported
platform                  -- windows, linux, macos
fixture_version
started_at_ms
ended_at_ms
result                    -- passed, failed, flaky, skipped
failure_summary
artifact_ref
```

Зачем:

- Windows/zellij/native parity is testable;
- regressions are tracked by scenario;
- replay fixtures become part of release confidence;
- support can ask for exact scenario coverage.

### 30. `terminal_event_schemas`

Schema registry for event payloads and compatibility.

```text
id
schema_ref
event_type
schema_version
json_schema
created_at_ms
deprecated_at_ms
compatibility             -- backward, forward, breaking
upcaster_name
```

Зачем:

- terminal journal survives app upgrades;
- import/export can validate data;
- unknown event types have explicit compatibility behavior;
- projections can rebuild after schema changes.

### 31. `terminal_export_manifests`

Durable record of exports/imports.

```text
id
session_id
export_kind               -- asciicast, jsonseq, markdown, debug_bundle
schema_version
redaction_profile_id
created_at_ms
file_ref
checksum
status                    -- created, imported, quarantined, failed
failure_summary
```

Зачем:

- user can see what left the app;
- support can reproduce debug bundles;
- redaction/export policy is auditable;
- imports can quarantine unsafe/incompatible bundles.

### 32. `terminal_sync_checkpoints`

Future-proofing for sync/backup without pretending it is live process restore.

```text
id
profile_id
checkpoint_kind           -- db_backup, wal_pitr, app_event_sync, changeset_sync
session_id
last_event_seq
last_db_change_id
created_at_ms
remote_ref
integrity_hash
status                    -- pending, uploaded, verified, failed, restored
```

Зачем:

- sync/backup/PITR have separate state from session restore;
- conflict handling can be introduced later;
- restore UI can explain what was recovered and from where;
- integrity of remote checkpoints is checkable.

### 33. `terminal_key_lifecycle_events`

Records key/secret lifecycle without storing secret material.

```text
id
secret_ref_id
event_type                -- created, verified, rotated, rekeyed, missing, inaccessible, revoked
provider
occurred_at_ms
actor_kind                -- user, system, migration, recovery
result                    -- ok, failed, partial
failure_summary
```

Зачем:

- encryption/storage failures are diagnosable;
- rekey/migration is auditable;
- backup/export knows which key profile was used;
- missing OS credential is handled gracefully.

### 34. `terminal_temp_artifacts`

Tracks temporary files and caches that may contain history data.

```text
id
session_id
artifact_kind             -- sqlite_temp, export_temp, decoded_media, debug_bundle, projection_cache
path_ref
created_at_ms
expires_at_ms
size_bytes
contains_sensitive_data
cleanup_status            -- pending, deleted, missing, failed
failure_summary
```

Зачем:

- temp files are included in privacy model;
- cleanup failures are visible;
- cache quotas are enforceable;
- debug/export temp files do not linger silently.

### 35. `terminal_accessibility_announcements`

Derived events for assistive technologies and keyboard-only UX.

```text
id
session_id
pane_id
announcement_kind         -- restored, degraded, search_done, export_blocked, redacted, live_boundary
message_key
priority                  -- polite, assertive
created_at_ms
delivered_at_ms
dedupe_key
```

Зачем:

- status changes are announced without moving focus;
- noisy terminal output does not spam screen readers;
- restore/search/export states are accessible;
- accessibility behavior is testable.

### 36. `terminal_retention_reviews`

Records user/system review of retention policy.

```text
id
policy_id
reviewed_at_ms
actor_kind                -- user, system
estimated_db_bytes
estimated_artifact_bytes
sessions_to_prune
possible_secret_sessions
backup_export_refs
decision                  -- accepted, postponed, changed_policy
```

Зачем:

- retention becomes understandable;
- privacy cleanup can include backups/exports;
- large history growth is handled before disk pressure;
- policy changes are explainable later.

### 37. `terminal_background_jobs`

Durable job queue for projection, redaction, artifact and maintenance work.

```text
id
job_kind                  -- rebuild_search, rebuild_projection, redact_range, compress_cold, gc_artifacts, prepare_export
session_id
pane_id
target_ref
schema_version
priority
state                     -- queued, claimed, running, succeeded, failed, quarantined
attempt_count
next_run_at_ms
claimed_by
claimed_at_ms
last_error
created_at_ms
updated_at_ms
```

Зачем:

- restart-safe maintenance;
- idempotent projection rebuilds;
- redaction/export/GC can be retried;
- diagnostics can show stuck jobs.

### 38. `terminal_search_profiles`

Versioned search/index strategy.

```text
id
profile_name
engine                    -- derived_chunks, sqlite_fts5, tantivy
tokenizer                 -- unicode_words, trigram, path_aware, custom
schema_version
source_projection_version
created_at_ms
status                    -- active, stale, rebuilding, disabled
```

Зачем:

- FTS5/Tantivy can be swapped as derived projection;
- search rebuilds are explicit;
- redaction can invalidate only affected profiles;
- tests can assert tokenizer behavior.

### 39. `terminal_time_anchors`

Maps wall time and monotonic timing for replay/debug.

```text
id
session_id
pane_id
anchor_kind               -- session_start, process_start, import_start, reconnect, mux_attach
wall_time_ms
monotonic_origin_label
source_clock
clock_quality
created_at_ms
```

Зачем:

- replay timing can use relative monotonic deltas;
- UI can show wall-clock timestamps;
- remote/imported timing is honest about quality;
- clock jumps become diagnosable.

### 40. `terminal_ai_context_exports`

Records terminal history sent to AI/context consumers.

```text
id
session_id
pane_id
consumer_kind             -- local_ai, mcp_resource, debug_prompt, external_export
context_id
stream_seq_start
stream_seq_end
command_block_ids_json
redaction_profile_id
trust_summary
byte_count
created_at_ms
status                    -- created, blocked_sensitive, expired, deleted
```

Зачем:

- AI context is auditable;
- prompt injection boundaries are explicit;
- secret/redaction policy applies to AI too;
- exact provenance/ranges can be cited later.

### 41. `terminal_process_lifecycle_events`

Native/mux process lifecycle facts.

```text
id
session_id
pane_id
backend_kind
event_kind                -- launched, ctrl_c_sent, ctrl_break_sent, conpty_closed, job_terminated, exited, orphan_detected
process_id
process_group_ref
job_ref
occurred_at_ms
exit_code
reason
trust_level
```

Зачем:

- graceful vs force shutdown is visible;
- Windows Job Object cleanup is auditable;
- restore can explain process state honestly;
- zellij/tmux attach is not confused with native process lifecycle.

### 42. `terminal_artifact_integrity_checks`

Periodic and watcher-triggered artifact verification.

```text
id
artifact_id
check_kind                -- startup, scheduled, watcher_hint, export, restore
checked_at_ms
expected_size_bytes
actual_size_bytes
expected_hash
actual_hash
result                    -- ok, missing, modified, hash_mismatch, decrypt_failed, skipped
action_taken              -- none, quarantined, rebuilt, marked_missing
```

Зачем:

- missing/modified artifacts do not create silent broken restore;
- antivirus/cloud-sync/user edits become diagnosable;
- export/replay can verify before use;
- GC can avoid deleting referenced data.

### 43. `terminal_file_watch_events`

Best-effort filesystem watcher hints.

```text
id
watch_root_id
path_ref
event_kind                -- created, modified, deleted, renamed, overflow, unknown
observed_at_ms
source                    -- notify, readdirectorychangesw, inotify, fsevents, poll
processed_state           -- pending, scanned, ignored, failed
```

Зачем:

- watcher overflow can trigger full rescan;
- external artifact mutations are visible;
- Windows/Linux/macOS behavior can be debugged separately;
- watcher is clearly a hint layer, not truth.

### 44. `terminal_consistency_snapshots`

Records boundaries for backup/export/read consistency.

```text
id
session_id
snapshot_kind             -- logical_export, sqlite_backup, wal_snapshot, vss_external, import_quarantine
started_at_ms
finished_at_ms
db_page_count
wal_frame_ref
last_event_seq
artifact_manifest_hash
result                    -- ok, failed, partial, quarantined
```

Зачем:

- backups/export can be compared to journal ranges;
- partial exports are explainable;
- VSS/external backups are not confused with semantic exports;
- recovery can identify exactly what was captured.

### 45. `terminal_client_delivery_state`

Tracks browser/client delivery and ack state.

```text
id
client_id
connection_id
session_id
pane_id
last_server_seq_sent
last_client_seq_acked
last_replay_start_seq
last_replay_end_seq
connected_at_ms
disconnected_at_ms
gap_state                 -- none, replayable, unrecoverable, full_reload_required
```

Зачем:

- reconnect can replay missed output;
- duplicate output can be detected;
- unrecoverable gaps are visible;
- client delivery is not confused with DB durability.

### 46. `terminal_execution_domains`

Normalizes local, WSL, SSH and mux execution contexts.

```text
id
domain_kind               -- windows_native, wsl_linux, win32_from_wsl, ssh_remote, mux_remote, container, imported
display_name
host_fingerprint_ref
user_redaction_state
wsl_distro
wsl_config_hash
ssh_config_alias
mux_session_ref
path_style                -- windows, posix, uri, unknown
created_at_ms
last_seen_at_ms
```

Зачем:

- rerun targets correct shell/domain;
- remote/WSL metadata can be redacted;
- cwd/path parsing is domain-aware;
- zellij/tmux/SSH layers are explicit.

### 47. `terminal_feature_flag_states`

Captures persistence-affecting flags for diagnostics/repro.

```text
id
session_id
flag_name
flag_kind                 -- release, ops, migration, permission, experiment
value
reason
evaluated_at_ms
expires_at_ms
owner
```

Зачем:

- bug reports include rollout state;
- kill switch effects are explainable;
- stale flags can be cleaned;
- migrations can be audited.

### 48. `terminal_transport_chaos_runs`

Records reconnect/network reliability scenarios.

```text
id
scenario_name
backend_kind              -- native, wsl, ssh, zellij, tmux, websocket_only
fault_kind                -- disconnect, latency, loss, reordering, timeout, server_restart, browser_reload
started_at_ms
ended_at_ms
result                    -- passed, failed, flaky, skipped
failure_summary
artifact_ref
```

Зачем:

- transport reliability is tested, not assumed;
- Windows/SSH/zellij/browser paths have comparable coverage;
- regressions are tracked by scenario;
- reconnect bugs produce artifacts.

### 49. `terminal_payload_codecs`

Versioned payload encoding registry.

```text
id
codec_name                -- json, cbor, messagepack, protobuf, flatbuffers
schema_ref
schema_version
upcaster_version
created_at_ms
deprecated_at_ms
compatibility             -- current, read_only, deprecated, blocked
```

Зачем:

- event payloads can evolve safely;
- binary codecs are introduced without breaking old history;
- projections know which upcaster produced current shape;
- imports can reject unsupported codecs.

### 50. `terminal_query_plan_baselines`

Performance contract for critical SQLite queries.

```text
id
query_name
schema_version
query_hash
expected_plan_hash
fixture_name
max_rows_scanned
max_duration_ms
recorded_at_ms
status                    -- active, stale, failing, accepted_change
```

Зачем:

- restore/search/prune regressions are caught early;
- index changes are intentional;
- large-history behavior is testable;
- query plans become part of migration review.

### 51. `terminal_diagnostic_reports`

Privacy-gated diagnostics/crash/debug records.

```text
id
report_kind               -- crash, panic, debug_bundle, telemetry_snapshot, support_export
created_at_ms
session_id_hash
pane_id_hash
redaction_profile_id
raw_text_included
private_sessions_excluded
upload_state              -- local_only, pending_user_approval, uploaded, blocked
storage_ref
```

Зачем:

- diagnostics are auditable;
- raw terminal text is not uploaded accidentally;
- support exports can be regenerated/redacted;
- privacy review has concrete records.

### 52. `terminal_legal_holds`

Enterprise/audit retention override.

```text
id
scope_kind                -- workspace, session, pane, command_block, export
scope_ref
hold_reason
created_by
created_at_ms
expires_at_ms
status                    -- active, released, expired
release_reason
```

Зачем:

- retention/delete behavior is explainable;
- audit/legal mode can preserve required records;
- user deletion can show why purge is blocked;
- enterprise policy remains separate from developer default.

### 53. `terminal_storage_quota_samples`

Tracks local storage pressure over time.

```text
id
sampled_at_ms
db_bytes
wal_bytes
artifact_bytes
cache_bytes
temp_bytes
estimated_free_bytes
soft_quota_bytes
hard_quota_bytes
pressure_state            -- ok, warning, critical, no_history
```

Зачем:

- storage pressure is visible;
- cleanup can be proactive;
- strict mode can fail before data loss;
- UI warnings are based on measured state.

### 54. `terminal_browser_clients`

Tracks browser tabs/windows attached to terminal sessions.

```text
id
client_id
session_id
pane_id
role                      -- input_owner, viewer, background_cache, reconnecting
visibility_state          -- visible, hidden, frozen, discarded_unknown
last_heartbeat_ms
last_ack_seq
lock_state                -- server_owner, web_lock_owner, released, expired
created_at_ms
updated_at_ms
```

Зачем:

- multi-tab input conflicts are prevented;
- hidden/frozen tab behavior is diagnosable;
- reconnect can use last ack state;
- browser lifecycle is not confused with session lifecycle.

### 55. `terminal_side_effect_events`

Clipboard, file, link and similar side-effect attempts.

```text
id
session_id
pane_id
domain_id
effect_kind               -- clipboard_write, clipboard_read, file_transfer, link_open, paste_input
source                    -- osc52, browser_ui, keyboard_paste, terminal_media, user_action
policy_result             -- allowed, blocked, consent_required, failed_permission, historical_replay_ignored
payload_ref
occurred_at_ms
redaction_state
```

Зачем:

- replay can suppress side effects;
- clipboard/file operations become auditable;
- remote/container domain policy can differ;
- exports/AI can exclude side-effect payloads.

### 56. `terminal_container_contexts`

Container/Kubernetes execution metadata.

```text
id
domain_id
orchestrator              -- docker, kubernetes, podman, unknown
mode                      -- exec, attach, logs, imported
namespace
pod_name
container_name
container_id
image_ref
tty_enabled
stdin_enabled
context_redaction_state
created_at_ms
```

Зачем:

- docker exec/attach/logs are not conflated;
- kubectl exec/logs have domain metadata;
- rerun/export/AI can classify cluster/container details;
- TTY behavior is reproducible.

### 57. `terminal_encoding_profiles`

Encoding/codepage metadata for decoded text projections.

```text
id
session_id
pane_id
profile_kind              -- windows_codepage, utf8, powershell, imported, unknown
input_codepage
output_codepage
shell_encoding_policy
powershell_version
utf8_mode
decode_errors_count
created_at_ms
```

Зачем:

- Windows mojibake bugs are diagnosable;
- search/redaction can rebuild decoded text after decoder fixes;
- raw bytes remain canonical;
- exports can choose correct encoding.

### 58. `terminal_power_events`

Suspend/resume/shutdown-related durability events.

```text
id
event_kind                -- suspend_requested, resume, shutdown_requested, inhibitor_started, inhibitor_failed
platform                  -- windows, linux, macos, browser
occurred_at_ms
sessions_affected_json
writer_flush_result
reconnect_required
gap_state
```

Зачем:

- sleep/resume bugs are explicit;
- strict/audit mode can explain failed flush/inhibit;
- reconnect gaps after sleep are traceable;
- exports/backups can record interruption.

### 59. `terminal_gateway_auth_tokens`

Local gateway token and scope metadata. Store token hash, not raw token.

```text
id
token_hash
scope_kind                -- gateway, session, pane, export, debug
session_id
pane_id
runtime_slug
allowed_origin
allowed_host
created_at_ms
expires_at_ms
revoked_at_ms
last_used_at_ms
```

Зачем:

- gateway tokens are scoped and expirable;
- token misuse can be diagnosed;
- browser reconnect can be authorized without persistent raw token storage;
- export/debug tokens are not confused with terminal input tokens.

### 60. `terminal_gateway_security_events`

Rejected and suspicious local gateway access attempts.

```text
id
event_kind                -- bad_origin, bad_host, bad_token, expired_token, rate_limited, pna_preflight, forbidden_message
remote_addr
origin
host
token_id
session_id
occurred_at_ms
action_taken              -- rejected, challenged, logged, revoked
```

Зачем:

- DNS rebinding/CSWSH attempts are visible;
- security incidents can be investigated;
- rate limits and origin policy are testable;
- local gateway is not silent.

### 61. `terminal_ipc_endpoints`

Local IPC endpoint registry.

```text
id
endpoint_kind             -- websocket_loopback, named_pipe, unix_socket, desktop_ipc
endpoint_ref
security_descriptor_ref
owner_user_ref
allowed_origins_json
created_at_ms
closed_at_ms
state                     -- active, closing, closed, failed
```

Зачем:

- Windows named pipe/TCP/webview IPC are modeled consistently;
- endpoint lifetime is auditable;
- cleanup can remove stale endpoints;
- support can see which transport was active.

### 62. `terminal_import_quarantine_records`

Archive/debug-bundle import safety state.

```text
id
bundle_ref
source_kind               -- user_file, support_bundle, sync_download, test_fixture
received_at_ms
entry_count
compressed_bytes
expanded_bytes
path_validation_state
schema_validation_state
redaction_scan_state
status                    -- quarantined, rejected, imported, partial, failed
failure_summary
```

Зачем:

- unsafe imports do not touch live history store;
- zip/path traversal attempts are visible;
- support bundles can be inspected safely;
- partial import is explainable.

### 63. `terminal_viewer_security_profiles`

Rendering/export viewer restrictions.

```text
id
profile_name
viewer_kind               -- in_app_history, html_export, debug_bundle, ai_preview
csp_policy
sandbox_flags
allow_links
allow_media_preview
allow_clipboard
allow_service_worker
created_at_ms
```

Зачем:

- historical replay/viewer side effects are centrally controlled;
- HTML export security is testable;
- private/debug/AI viewers can differ safely;
- CSP/sandbox policy is not scattered across UI code.

### 64. `terminal_file_write_transactions`

Crash-safe external artifact write ledger.

```text
id
artifact_id
path_ref
temp_path_ref
write_kind                 -- stream_segment, media_payload, export_bundle, manifest, search_shard
bytes_expected
bytes_written
file_flush_state           -- skipped, pending, done, failed, unsupported
dir_flush_state            -- skipped, pending, done, failed, unsupported
replace_state              -- pending, replaced, failed, recovered
checksum
started_at_ms
finished_at_ms
result                     -- success, partial, failed, recovered
failure_summary
```

Зачем:

- artifact durability becomes observable;
- Windows replace/flush failures are explainable;
- recovery can find temp files and partial writes;
- DB manifest and filesystem can be reconciled.

### 65. `terminal_file_lock_records`

Runtime coordination and stale-lock diagnostics.

```text
id
lock_kind                  -- writer, compactor, export, repair, backup
path_ref
owner_process_ref
owner_session_id
acquired_at_ms
expires_at_ms
released_at_ms
stale_state                -- active, released, stale, stolen, unknown
result                     -- acquired, denied, expired, force_recovered
```

Зачем:

- concurrent writer bugs become visible;
- support can see why backup/export did not start;
- stale locks after crash can be cleaned safely;
- lock behavior is not confused with DB truth.

### 66. `terminal_redaction_rules`

Versioned redaction and secret scanning rules.

```text
id
rule_set_id
rule_name
rule_kind                  -- exact, aho_corasick, regex_linear, validator, entropy
pattern_ref
version
severity
enabled
max_input_bytes
timeout_ms
test_fixture_ref
created_at_ms
```

Зачем:

- redaction is testable and rollbackable;
- ReDoS/performance budget is explicit;
- export/AI/share can state which rule set was used;
- noisy or slow rules can be disabled without code changes.

### 67. `terminal_policy_decisions`

Auditable authorization decisions.

```text
id
policy_engine
policy_version
subject_ref
resource_kind              -- session, pane, command_block, artifact, export_bundle, ai_context
resource_ref
action                     -- view, copy, rerun, export, share, delete, attach_ai
environment_json
decision                   -- allow, deny, challenge, redact_only
reason_code
evaluated_at_ms
```

Зачем:

- share/export/delete/rerun decisions can be explained;
- policy changes are auditable;
- tests can assert deny-by-default;
- enterprise mode can plug in richer engines later.

### 68. `terminal_capability_grants`

Narrow grants for sharing and delegated access.

```text
id
grant_kind                 -- share_link, support_bundle, ai_context, delegated_view
token_hash
subject_ref
resource_kind
resource_ref
actions_json
caveats_json               -- expiry, audience, redaction profile, device/workspace scope
created_at_ms
expires_at_ms
revoked_at_ms
last_used_at_ms
```

Зачем:

- share links are attenuated instead of broad account access;
- revoke and expiry are first-class;
- raw/export/rerun permissions can be separated;
- support and audit can inspect grant lifecycle without seeing token plaintext.

### 69. `terminal_path_identity_records`

Platform path and file identity metadata for critical artifacts.

```text
id
artifact_id
platform                  -- windows, linux, macos
display_path_ref
storage_key
canonical_path_ref
final_path_ref
volume_serial
file_id
case_sensitivity_state    -- unknown, insensitive, sensitive
reparse_state             -- none, symlink, junction, mount_point, other, rejected
verified_at_ms
verification_result       -- ok, moved, missing, outside_root, reparse_rejected, unknown
```

Зачем:

- path string is not treated as identity;
- Windows file ID/volume can detect unexpected replacement;
- symlink/junction escapes become visible;
- display path and storage path stay separated.

### 70. `terminal_artifact_path_resolutions`

Every sensitive path decision around import/export/store.

```text
id
operation_kind            -- write_artifact, export_bundle, import_bundle, verify, repair
requested_path_ref
normalized_path_ref
final_path_ref
root_ref
decision                  -- allow, reject, quarantine, generated_name
reason_code               -- traversal, reserved_name, ads, long_path, reparse, cross_volume, ok
created_at_ms
```

Зачем:

- path rejection is testable and debuggable;
- support can explain why a bundle entry was quarantined;
- Windows-specific filename hazards have first-class reason codes;
- import/export does not silently rewrite dangerous names.

### 71. `terminal_fs_watch_resync_events`

Filesystem watcher/USN health records.

```text
id
watcher_kind              -- read_directory_changes, usn, inotify, fsevents, notify_crate, manual_scan
root_ref
last_seen_cursor
event_count
overflow_detected
journal_reset_detected
full_rescan_started_at_ms
full_rescan_finished_at_ms
result                    -- clean, mismatch_found, repaired, failed
```

Зачем:

- watcher gaps force rescan instead of silent trust;
- external artifact tampering/deletion is discoverable;
- Windows USN resets/overflows become operational events;
- verifier behavior is auditable.

### 72. `terminal_delivery_offsets`

Per-client/per-pane delivery and replay state.

```text
id
client_id
session_id
pane_id
stream_id
last_sent_seq
last_acked_seq
last_persisted_seq
replay_from_seq
gap_state                 -- none, replaying, unrecoverable_gap, compacted
updated_at_ms
```

Зачем:

- browser reconnect can request missed ranges;
- support can see whether output was persisted vs merely sent;
- gaps are explicit;
- replay logic is testable.

### 73. `terminal_idempotency_keys`

Retry-safe user/runtime operations.

```text
id
scope_kind                -- session, pane, export, share, delete, ai_context
scope_ref
operation_kind            -- submit_input, create_export, create_share, delete_history, rerun_command
idempotency_key_hash
request_hash
first_seen_at_ms
last_seen_at_ms
result_ref
state                     -- pending, completed, failed, expired, conflict
```

Зачем:

- duplicate clicks/retries do not duplicate commands/exports/shares;
- result can be returned after reconnect;
- request mismatch under same key is detectable;
- destructive actions are safer.

### 74. `terminal_outbox_messages`

Durable jobs emitted with journal/state changes.

```text
id
aggregate_kind            -- session, pane, command_block, artifact, export, policy
aggregate_ref
event_type
event_seq_low
event_seq_high
payload_ref
status                    -- pending, claimed, done, failed, quarantined
attempt_count
next_attempt_at_ms
claimed_by
created_at_ms
updated_at_ms
```

Зачем:

- projection/search/export/sync jobs cannot be missed after commit;
- retries are visible;
- poison jobs can be quarantined;
- derived layers can lag without losing truth.

### 75. `terminal_inbox_dedup_records`

Inbound sync/import event deduplication.

```text
id
source_kind               -- local_peer, sync_remote, import_bundle, support_restore
source_ref
source_event_id
source_event_hash
first_seen_at_ms
applied_at_ms
decision                  -- applied, duplicate, rejected, conflict, quarantined
reason_code
```

Зачем:

- repeated imports/sync retries are safe;
- conflict vs duplicate is explicit;
- imported history can be audited;
- source-specific bugs are diagnosable.

### 76. `terminal_snapshot_manifests`

Restore checkpoint lineage.

```text
id
session_id
pane_id
snapshot_kind             -- screen, scrollback, projection, export, backup_checkpoint
parent_snapshot_id
manifest_version
base_seq
high_water_seq
parser_version
projection_version
redaction_profile_id
artifact_refs_json
checksum
created_at_ms
```

Зачем:

- restore can choose nearest valid snapshot;
- projection changes can trigger rebuild;
- snapshots have parent/high-water lineage;
- missing artifacts are detectable before restore.

### 77. `terminal_backup_restore_drills`

Proof that backup can actually restore.

```text
id
backup_ref
drill_kind                -- automatic_sample, manual_support, migration_guard, release_guard
started_at_ms
finished_at_ms
temp_restore_ref
schema_check_state
artifact_check_state
projection_rebuild_state
sample_replay_state
result                    -- passed, warning, failed, canceled
summary
```

Зачем:

- backup quality is measurable;
- release/migration can block on restore failures;
- corrupt/missing chunks are found early;
- support can run safe restore diagnostics.

### 78. `terminal_sync_conflicts`

Visible branch/conflict records for history sync.

```text
id
session_id
pane_id
stream_id
local_branch_ref
remote_branch_ref
base_seq
conflict_seq
conflict_kind             -- duplicate_writer, divergent_seq, tombstone_conflict, artifact_missing, policy_conflict
created_at_ms
resolved_at_ms
resolution                -- keep_local, keep_remote, keep_both, archive_remote, manual
```

Зачем:

- terminal byte streams are not silently merged;
- user/support can understand divergent history;
- retention/delete conflicts are explicit;
- sync can be conservative without hiding data.

### 79. `terminal_chunk_catalog`

Search/replay catalog over canonical stream chunks.

```text
id
session_id
pane_id
stream_id
seq_low
seq_high
time_low_ms
time_high_ms
byte_offset
compressed_bytes
decompressed_bytes
codec
checksum
storage_tier              -- hot, warm, cold, external, missing
redaction_profile_id
parser_version
projection_version
artifact_ref
created_at_ms
```

Зачем:

- query planner can skip irrelevant chunks;
- restore can pick ranges quickly;
- lifecycle movement is visible;
- missing/corrupt chunks are detectable.

### 80. `terminal_chunk_prefilters`

Bloom/token/minmax summaries for candidate pruning.

```text
id
chunk_id
prefilter_kind            -- bloom_token, trigram, minmax_time, minmax_seq, command_status, path_prefix
profile_version
payload_ref
false_positive_target
created_at_ms
last_verified_at_ms
state                     -- active, stale, rebuilding, failed
```

Зачем:

- broad searches scan fewer chunks;
- prefilter versions are rebuildable;
- false positives are expected and verified;
- stale summaries do not become truth.

### 81. `terminal_search_indexes`

Derived full-text/search index metadata.

```text
id
index_kind                -- sqlite_fts5, tantivy, quickwit_split, contentless_tokens, analytics_columnar
scope_kind                -- hot_local, warm_local, cold_object, export_bundle
source_seq_low
source_seq_high
tokenizer_version
redaction_profile_id
parser_version
artifact_refs_json
build_state               -- queued, building, active, stale, failed, quarantined
merge_state               -- none, pending, running, complete, failed
created_at_ms
updated_at_ms
```

Зачем:

- search index can be rebuilt safely;
- stale/redaction-incompatible indexes are detectable;
- hot/warm/cold search tiers are explicit;
- migration can invalidate only affected indexes.

### 82. `terminal_search_queries`

Search audit, budgets and result status.

```text
id
subject_ref
query_hash
scope_json
tier_allowlist_json
budget_json
policy_decision_id
started_at_ms
finished_at_ms
chunks_considered
chunks_scanned
bytes_decompressed
result_state              -- complete, partial, canceled, denied, stale_index, failed
resume_cursor_ref
```

Зачем:

- expensive searches are measurable;
- partial results are explainable;
- authorization/redaction is attached to query;
- support can debug slow history search.

### 83. `terminal_lifecycle_tier_events`

Hot/warm/cold movement and retention transitions.

```text
id
artifact_id
chunk_id
from_tier
to_tier
reason                    -- ttl, quota_pressure, manual_pin, legal_hold, backup_complete, privacy_policy
policy_version
started_at_ms
finished_at_ms
result                    -- moved, skipped, pinned, failed, quarantined
```

Зачем:

- retention is auditable;
- pinned/legal-held chunks are protected;
- cold migration failures are visible;
- search can explain slow/missing tiers.

### 84. `terminal_ai_context_packages`

AI context package boundary and policy state.

```text
id
session_id
request_ref
package_kind              -- explain, fix_error, summarize, search_answer, agent_task, export_context
policy_decision_id
redaction_profile_id
token_budget
byte_budget
item_count
sensitive_finding_count
created_at_ms
sent_at_ms
state                     -- draft, approved, sent, blocked, expired, discarded
```

Зачем:

- AI context is explicit and auditable;
- redaction/policy is attached before send;
- context budget is visible;
- user can inspect what was included.

### 85. `terminal_ai_context_items`

Structured items inside an AI context package.

```text
id
package_id
item_kind                 -- user_request, command_text, terminal_output, search_snippet, summary, artifact_metadata
source_ref
session_id
pane_id
command_block_id
seq_low
seq_high
trust_level
instruction_eligibility   -- trusted_instruction, data_only, derived_summary, blocked
redaction_state
token_estimate
truncation_state
```

Зачем:

- terminal output stays separated from instructions;
- provenance survives summarization;
- risky ranges can be removed without losing the whole package;
- AI answers can cite exact terminal ranges.

### 86. `terminal_prompt_injection_findings`

Prompt-injection risk findings over terminal/history content.

```text
id
source_kind               -- stream_chunk, command_block, search_snippet, artifact, ai_summary
source_ref
seq_low
seq_high
finding_kind              -- instruction_override, tool_abuse, fake_approval, hidden_unicode, encoded_payload, malicious_link
severity
confidence
rule_id
created_at_ms
resolution                -- active, suppressed, false_positive, stripped, quoted
```

Зачем:

- injection risk is tracked with source ranges;
- findings can affect context packaging;
- red-team tests can assert detections;
- false positives are reviewable.

### 87. `terminal_ai_action_approvals`

Approval gates for AI-originated terminal/history actions.

```text
id
package_id
action_kind               -- send_input, rerun_command, paste, export_history, share_session, delete_history, read_raw
resource_kind
resource_ref
proposed_payload_ref
policy_decision_id
prompt_injection_finding_count
requested_at_ms
approved_at_ms
expires_at_ms
decision                  -- approved, denied, expired, canceled, policy_blocked
```

Зачем:

- AI cannot silently act on hostile output;
- approvals include context and risk;
- destructive actions are auditable;
- stale approvals expire.

### 88. `terminal_ai_redteam_runs`

Automated AI-context security regression runs.

```text
id
fixture_set_version
model_profile_ref
tool_policy_version
started_at_ms
finished_at_ms
fixtures_run
blocked_action_count
unexpected_action_count
leak_count
result                    -- passed, warning, failed
report_ref
```

Зачем:

- prompt-injection defenses are tested continuously;
- model/tool/policy changes have regression reports;
- failures can block release;
- fixtures become part of persistence QA.

### 89. `terminal_ai_tool_policy_checks`

Deterministic policy decisions around model-requested tool calls.

```text
id
package_id
tool_name
tool_version
requested_action
resource_scope_json
policy_version
decision                  -- allow, deny, require_user_approval, require_redaction
reason_code
checked_at_ms
```

Зачем:

- tool permission drift is visible;
- agent actions are not authorized by prompt text;
- MCP/tool integrations share the same guardrail layer;
- policy checks can be tested outside the model.

### 90. `terminal_reliability_invariants`

Executable invariant registry.

```text
id
invariant_key
description
scope_kind                -- journal, command_block, snapshot, outbox, backup, search, ai_context
severity
checker_kind              -- sql_query, rust_checker, model_check, simulation_check, restore_drill
checker_ref
enabled
created_at_ms
last_passed_at_ms
last_failed_at_ms
```

Зачем:

- reliability claims are tied to executable checks;
- release gates can refer to invariant IDs;
- production diagnostics can reuse the same concepts;
- regressions are not hidden in prose.

### 91. `terminal_fault_injection_runs`

Fault/simulation run records.

```text
id
run_kind                  -- simulation, integration_fault, chaos, crash_replay, concurrency_schedule
scenario_version
seed
fault_plan_ref
started_at_ms
finished_at_ms
result                    -- passed, failed, flaky, minimized, quarantined
failure_artifact_ref
invariant_failure_count
runtime_versions_json
```

Зачем:

- rare failures become reproducible;
- seeds are preserved;
- minimized repros can be stored;
- release gates can require scenario coverage.

### 92. `terminal_fault_points`

Named failpoints around critical persistence boundaries.

```text
id
fault_point_key
component                 -- writer, artifact_store, db, outbox, transport, redaction, search, ai
operation                 -- write, flush, rename, commit, claim, ack, replay, export
fault_kinds_json          -- error, timeout, panic, short_write, duplicate, reorder, delay
enabled_in_profiles_json
owner
created_at_ms
```

Зачем:

- fault coverage is explicit;
- new critical code paths must add failpoints;
- tests can target named boundaries;
- support can map failures to components.

### 93. `terminal_model_check_specs`

Formal/model-check spec metadata.

```text
id
spec_name
protocol_kind             -- seq_ack, idempotency, outbox, snapshot_lineage, tombstone_sync, approval_gate
tool_kind                 -- tla, apalache, stateright, custom
spec_ref
state_bound_summary
last_run_at_ms
last_result               -- passed, failed, timeout, inconclusive
counterexample_ref
```

Зачем:

- small protocols have checked state-machine specs;
- counterexamples are saved as fixtures;
- implementation changes can trigger model-check reruns;
- design assumptions are machine-readable.

### 94. `terminal_release_reliability_gates`

Release checklist and gate state.

```text
id
release_ref
gate_kind                 -- migration, crash_sim, windows_paths, restore_drill, redaction, ai_redteam, search_rebuild, backup_roundtrip
required
started_at_ms
finished_at_ms
result                    -- passed, failed, waived, skipped
waiver_reason
evidence_ref
```

Зачем:

- reliability checklist is enforced;
- skipped gates require explicit waiver;
- release notes can state real guarantees;
- historical regressions are traceable.

### 95. `terminal_failure_replay_artifacts`

Stored reproductions for failed tests/chaos runs.

```text
id
failure_kind
seed
schedule_ref
fault_plan_ref
db_fixture_ref
export_bundle_ref
event_trace_ref
screenshot_ref
created_at_ms
minimized_state           -- raw, minimized, cannot_minimize, fixed
fixed_by_ref
```

Зачем:

- rare bugs are replayable;
- developers get exact fixtures;
- fixed bugs stay in regression suite;
- support artifacts do not leak raw history by accident.

### 96. `terminal_key_hierarchy_nodes`

Key hierarchy and wrapping metadata.

```text
id
key_ref
parent_key_ref
key_kind                  -- root_ref, workspace_kek, db_key, stream_dek, artifact_dek, search_dek, export_dek
algorithm
version
wrapping_kind             -- os_store, passphrase_kdf, hpke_recipient, enterprise_escrow, none
wrapped_key_ref
created_at_ms
activated_at_ms
retired_at_ms
destroyed_at_ms
state                     -- prepared, active, retiring, retired, destroyed, lost, quarantined
```

Зачем:

- key lifecycle is explicit;
- rotation can rewrap keys without rewriting all data;
- selective cryptographic erase becomes possible;
- recovery profile can be audited.

### 97. `terminal_encrypted_artifacts`

Encryption metadata for DB-external artifacts and projections.

```text
id
artifact_id
artifact_kind             -- stream_chunk, media, search_shard, export_bundle, ai_cache, backup_chunk
dek_ref
algorithm
nonce_ref
aad_hash
ciphertext_bytes
plaintext_bytes
checksum
created_at_ms
last_verified_at_ms
state                     -- active, decrypt_verified, key_missing, corrupt, erased, quarantined
```

Зачем:

- external files are not left outside encryption policy;
- restore can verify decryptability before replay;
- AAD binds ciphertext to manifest context;
- erased/corrupt/key-missing states are visible.

### 98. `terminal_key_rotation_jobs`

Crash-safe rekey/rotation workflow.

```text
id
rotation_kind             -- db_rekey, kek_rotate, dek_rewrap, export_rotate, recovery_key_change
scope_kind
scope_ref
old_key_ref
new_key_ref
started_at_ms
finished_at_ms
state                     -- prepared, rewrapping, verifying, committed, rolling_back, failed, quarantined
items_total
items_done
failure_summary
```

Зачем:

- key rotation is resumable;
- partial rewrap is not hidden;
- crash during rekey can be recovered;
- release gates can test rekey jobs.

### 99. `terminal_crypto_erase_records`

Cryptographic deletion evidence.

```text
id
erase_scope_kind          -- session, pane, chunk, export, ai_context, search_shard
erase_scope_ref
key_ref
tombstone_ref
authorized_by_ref
requested_at_ms
completed_at_ms
result                    -- erased, pending_gc, blocked_by_hold, key_missing, failed
limitations_json          -- backups, object_versions, temp_files, legal_hold
```

Зачем:

- deletion has evidence beyond `DELETE FROM`;
- legal hold/object versions are not hidden;
- search/AI/export caches can be invalidated;
- support can explain what was and was not erased.

### 100. `terminal_key_access_audit`

Sensitive key unwrap/use records.

```text
id
key_ref
operation                 -- unwrap, decrypt_artifact, export, rotate, destroy, backup, recovery
subject_ref
resource_kind
resource_ref
policy_decision_id
occurred_at_ms
result                    -- allowed, denied, failed, challenged
reason_code
```

Зачем:

- raw/decrypt/export operations are auditable;
- unusual key access is visible;
- enterprise and support workflows can be reviewed;
- policy decisions tie to key use.

### 101. `terminal_crypto_capability_profiles`

Platform key-store and crypto capability profile.

```text
id
platform                  -- windows, macos, linux, test
profile_name
os_store_kind             -- dpapi_user, dpapi_machine, keychain, secret_service, passphrase_only, none
headless_supported
roaming_supported
hardware_backed_state     -- yes, no, unknown
requires_user_unlock
created_at_ms
last_probe_at_ms
probe_result
```

Зачем:

- OS key storage assumptions are tested;
- Windows/macOS/Linux behavior is explicit;
- CI/headless/dev modes can degrade safely;
- recovery UX can reflect real capabilities.

## Restore semantics

Нужно честно показывать несколько разных режимов.

### Native backend

Что можно гарантировать:

- tabs/panes/layout;
- cwd и shell launch spec;
- command blocks;
- output history;
- last visible screen;
- scrollback из durable journal/snapshots.

Что нельзя гарантировать:

- процесс `npm run dev` продолжает жить после полного restart;
- foreground process state восстановлен;
- TUI internal state восстановлен как live state.

Рекомендуемое значение:

```text
restores_topology = true
restores_command_blocks = true
restores_output_history = true
restores_screen_snapshot = true
replays_terminal_journal = true
preserves_process_state = false
```

### Zellij backend

Что можно гарантировать лучше:

- если zellij session live, attach сохраняет процессы;
- если session resurrected, zellij сам умеет layout/commands/viewport/scrollback;
- наша history layer может хранить transcript независимо от zellij cache.

Что сложнее:

- raw PTY stream принадлежит zellij и содержит mux UI;
- command attribution внутри panes зависит от shell integration passthrough;
- output разных panes нельзя надежно разделить только из внешнего raw stream.

Рекомендуемый подход:

- Для zellij journal primary source - `screen_delta`/pane surface snapshots.
- Command blocks - только через shell integration внутри pane.
- Raw PTY zellij output хранить как debug/secondary, не как source of truth.

## Shell integration strategy

### Protocol priority

1. `OSC 633` с nonce - лучший rich path.
2. `OSC 633` без nonce - rich, но untrusted.
3. `OSC 133` + optional cmdline extension - хороший базовый path.
4. `OSC 7` / `OSC 9;9` / `OSC 633 P Cwd` - cwd signals.
5. Shell-specific hooks - PowerShell/cmd/bash/zsh/fish.
6. Heuristics по prompt/output - только degraded mode.

### PowerShell

Нужно делать first-class:

- prompt function emits `OSC 133 A/B/D`;
- pre-exec hook или PSReadLine hook для command text;
- `$LastExitCode`, `$?`, `$Error` для exit status;
- cwd через `OSC 633 P Cwd`, `OSC 7` или `OSC 9;9`;
- disable/guard для non-interactive shells.

### cmd.exe

Ограничения:

- command lifecycle беднее;
- prompt может emit escape через `PROMPT`;
- exact command line и exit code хуже, чем PowerShell.

Подход:

- basic prompt marks через PROMPT;
- command text из UI submit, если команда отправлена command dock;
- для typed-in-terminal команд - lower confidence;
- exit code может быть unknown, если нет wrapper.

### bash/zsh/fish

Подход:

- bash/zsh через preexec/precmd hooks;
- fish через native prompt hooks;
- поддержать `ignorespace`;
- учитывать prompt themes/plugins;
- для subshell/nix-shell/ssh нужен manual setup или propagated integration.

## Privacy и безопасность

### Что должно быть по умолчанию

- Raw output capture включен.
- Raw input keystrokes выключен или redacted.
- Command text capture включен через shell integration / command dock.
- Secrets redaction включен по базовым patterns.
- Private session/history off режим должен быть видимым.
- Clear history должен чистить DB tables, snapshots, cache и localStorage.

### Что считать sensitive

- Команды с leading space, если shell использует `ignorespace`.
- Ввод при disabled echo.
- Tokens/passwords/API keys в command text и output.
- Environment variables, кроме явного allowlist.
- Shared sessions и AI context attachments.

### Spoofing

Любая программа внутри terminal может напечатать OSC sequence. Поэтому:

- `OSC 633 E` без nonce нельзя считать trusted.
- Markers из raw output должны иметь trust level.
- Команда "copy/re-run" должна использовать только trusted или confirmed blocks.
- Для untrusted block UI должен показывать lower confidence.

## Грабли, которые нельзя делать

### 1. Нельзя строить историю только из видимого текста

Grid - это результат rendering, а не source of truth. Output мог overwrite строку, быть в alternate screen, быть split между panes или не закончиться newline.

Правильно:

- raw/journal stream;
- semantic markers;
- snapshots как cache, а не единственная истина.

### 2. Нельзя хранить только последний screen snapshot

Snapshot покажет "как выглядело", но не даст:

- полную scrollback history;
- привязку output к команде;
- search по прошлому output;
- replay;
- block copy output.

Правильно:

- snapshots + journal.

### 3. Нельзя auto-run восстановленные команды

Zellij специально не запускает commands без подтверждения. Это защищает от `rm -rf`, deploy, migrations, money-moving commands.

Правильно:

- restored commands показывать как pending;
- запуск только по ENTER/explicit click;
- destructive commands дополнительно подтверждать.

### 4. Нельзя логировать raw input по умолчанию

`script(1)` прямо предупреждает: input log пишет passwords даже когда echo disabled.

Правильно:

- хранить command text, а не каждую клавишу;
- raw keystroke logging только opt-in;
- detecting no-echo regions;
- redaction.

### 5. Нельзя считать zellij/tmux raw output обычным shell output

Mux рисует свой UI и мультиплексирует panes. Снаружи raw stream уже не равен одному shell.

Правильно:

- zellij/tmux path через pane snapshots/deltas и shell integration inside pane;
- raw mux output как debug, не primary transcript.

### 6. Нельзя обещать native process persistence

Если app/runtime умер, обычный child process умер. Durable history не равна live process state.

Правильно:

- `preserves_process_state = false` для native restore;
- для live continuity использовать zellij/tmux/daemon.

### 7. Нельзя делать бесконечный scrollback в памяти

Kitty/WezTerm/tmux все имеют limits. Большой scrollback давит RAM/perf.

Правильно:

- in-memory viewport ограничен;
- durable history disk-backed;
- quota и pruning.

### 8. Нельзя писать в SQLite по одному байту

Это убьет throughput и увеличит WAL/fragmentation.

Правильно:

- single writer task;
- batching 50-100 ms или 32-64 KB;
- transactions;
- WAL;
- periodic checkpoint.

### 9. Нельзя держать длинные read transactions и забыть про WAL checkpoint

SQLite WAL может расти без конца, если readers мешают checkpoint.

Правильно:

- короткие read transactions;
- explicit checkpoint policy;
- metrics по WAL size;
- storage health indicator.

### 10. Нельзя смешать commands и output в одну таблицу

Command history - user-facing structured data. Output journal - high-volume stream.

Правильно:

- `terminal_command_blocks`;
- `terminal_stream_segments`;
- `terminal_journal_events`;
- `terminal_screen_snapshots`.

### 11. Нельзя игнорировать alternate screen

`vim`, `less`, `top`, TUIs используют alt screen. History и viewport ведут себя иначе.

Правильно:

- snapshots должны хранить active buffer;
- alt-screen enter/leave events;
- policy: сохранять last TUI frame как viewport, но не смешивать с normal scrollback.

### 12. Нельзя делать shell integration binary on/off

Качество markers бывает разное.

Правильно:

- quality levels;
- degraded UI;
- tests для no integration, basic, rich, spoofed.

### 13. Нельзя сохранять весь environment

asciinema по умолчанию пишет только `SHELL` и `TERM`. Это хороший privacy signal.

Правильно:

- env allowlist;
- user opt-in для расширенного env;
- redaction before storage.

### 14. Нельзя прятать ошибки persistence

Если journal writer сломался, пользователь должен знать, что history больше не гарантируется.

Правильно:

- degraded state in runtime;
- visible warning in UI;
- retry/backoff;
- test disk full / DB locked / corruption.

### 15. Нельзя привязывать историю только к browser localStorage

Browser cache не session source of truth и не переживает перенос/restore/runtime.

Правильно:

- DB source of truth;
- localStorage только UI cache.

### 16. Нельзя считать command submitted из dock и typed in shell одинаковыми

Dock submit можно точно записать до PTY. Typing inside terminal требует shell markers.

Правильно:

- `source = ui_submit | shell_integration | heuristic`;
- confidence/trust.

### 17. Нельзя считать `clear` удалением истории

Clear screen, clear scrollback, clear persisted history - разные операции.

Правильно:

- `clear visible`;
- `clear scrollback`;
- `clear persisted history`;
- отдельные journal events и DB deletion paths.

### 18. Нельзя replay-ить восстановленную историю как live output без маркировки

Пользователь должен понимать, где historical transcript, а где новый live process.

Правильно:

- restored content visual marker;
- live boundary marker;
- no duplicated output after attach.

### 19. Нельзя забыть про resize events

Без resize replay даст другой wrapping и другой visual transcript.

Правильно:

- persist resize events;
- snapshot rows/cols;
- replay uses recorded cols when reconstructing transcript.

### 20. Нельзя полагаться только на exit code

Ctrl+C, killed process, closed pane, shell crash и TUI exit могут не дать нормальный status.

Правильно:

- statuses: `succeeded`, `failed`, `cancelled`, `terminated`, `unknown`;
- separate signal/exit reason if available.

### 21. Нельзя считать shell native history достаточной

PSReadLine/Nushell/Atuin/fish/zsh history помогают, но они обычно не содержат:

- полный output;
- screen resize;
- alternate screen;
- exact pane/tab topology;
- restored/live boundary;
- command output ranges.

Правильно:

- shell history можно импортировать/сверять;
- session truth хранить в нашем journal.

### 22. Нельзя делать command blocks только для команд из command dock

Пользователь может печатать прямо в terminal, paste, запускать команды из shell aliases/functions, использовать TUI.

Правильно:

- command dock submit - trusted source;
- shell integration - основной source for typed commands;
- heuristic fallback - degraded.

### 23. Нельзя считать все OSC markers одинаково надежными

Любая программа может вывести `OSC 133`/`OSC 633`. Без nonce это может быть spoofing.

Правильно:

- хранить trust level;
- поддержать nonce;
- для untrusted markers не делать опасных действий;
- тестировать malicious output.

### 24. Нельзя копировать SQLite DB обычным file copy при live WAL

В WAL режиме state может быть в `.sqlite3`, `.sqlite3-wal`, `.sqlite3-shm`.

Правильно:

- backup через SQLite backup API;
- controlled export;
- checkpoint before maintenance when safe.

### 25. Нельзя позволять WAL расти бесконечно

Long readers и постоянная запись journal могут удерживать WAL.

Правильно:

- short read transactions;
- checkpoint policy;
- WAL size metrics;
- writer lag metrics.

### 26. Нельзя хранить огромный terminal output одним blob

Huge blob сложно читать частично, prune-ить, checksum-ить и replay-ить.

Правильно:

- stream segments с sequence ranges;
- segment max size;
- compression per segment;
- indexes by session/pane/seq.

### 27. Нельзя replay-ить restored OSC effects как живые side effects

Restored history может содержать OSC 52 clipboard, hyperlinks, title changes, desktop notifications или shell integration markers.

Правильно:

- replay в inert/restore mode;
- side effects disabled during historical replay;
- render links safely.

### 28. Нельзя игнорировать command aliases/functions

Command text `gs` может означать `git status`, function может сделать много действий.

Правильно:

- хранить raw command line как пользователь ввел;
- не пытаться "разворачивать" alias без shell support;
- output/result остается истинным результатом.

### 29. Нельзя удалять history только из UI cache

Atuin отдельно показывает, что deletion в synced/durable history сложнее локального clear.

Правильно:

- delete scope: block/session/workspace/all;
- удалить command blocks, stream segments, events, snapshots, local cache;
- позже для sync - tombstone semantics.

### 30. Нельзя делать restore без provenance

Пользователь должен понимать, откуда взялась история и насколько она точна.

Правильно:

- restore banner;
- timestamp;
- source: native journal / zellij attach / zellij resurrection / cached snapshot;
- integration quality;
- degraded warnings.

### 31. Нельзя смешивать output от background jobs с последней foreground command

Warp явно выделяет background blocks, потому output может прийти после завершения команды или из другого job.

Правильно:

- background/unattributed blocks;
- confidence score;
- не подмешивать в command output без marker/sequence evidence.

### 32. Нельзя считать resize невинной метаданной

Terminal wrapping зависит от cols. Один и тот же byte stream при другом width даст другой visible transcript.

Правильно:

- persist resize events;
- snapshot rows/cols;
- replay with recorded dimensions for accurate transcript.

### 33. Нельзя внедрять shell integration без rollback

Prompt themes, oh-my-zsh, starship, PowerShell profiles, corporate scripts могут ломаться.

Правильно:

- feature flag;
- disable per shell/session;
- detect marker health;
- fallback to classic terminal.

### 34. Нельзя считать command output text равным raw bytes

Terminal output содержит control sequences, carriage returns, progress bars, hyperlinks, colors.

Правильно:

- хранить raw/segment bytes для replay;
- хранить rendered/search text как derived read model;
- invalidate derived text when parser changes.

### 35. Нельзя забыть про corruption recovery

Journal DB может быть повреждена, write может оборваться посреди segment.

Правильно:

- segment checksum;
- monotonic sequence;
- ignore/quarantine bad segment;
- `PRAGMA integrity_check` in diagnostics;
- partial restore вместо полного отказа.

### 36. Нельзя записывать playback как новую history

Audit systems предупреждают про feedback loops: просмотр записей может сам попасть в запись.

Правильно:

- replay/export/player режим ставит `recording_disabled_for_replay = true`;
- restored historical output не пишет stream segments;
- live boundary явно отделяет replay from live.

### 37. Нельзя игнорировать shell private/ignore-space intent

Если shell history не сохраняет команду из-за private mode или leading space, пользователь ожидает privacy.

Правильно:

- detect/propagate private mode when possible;
- leading space command gets `privacy_hint`;
- raw input off;
- explicit setting for overriding shell privacy.

### 38. Нельзя считать hyperlinks/clipboard sequences обычным output

OSC 8 и OSC 52 могут создать ссылки или взаимодействовать с clipboard.

Правильно:

- disable side effects during historical replay;
- sanitize links;
- never execute/open links automatically;
- track `active_content_present`.

### 39. Нельзя делать backup простым архивированием папки во время работы

SQLite WAL/SHM и in-flight writer делают file copy ненадежным.

Правильно:

- backup API;
- pause/checkpoint writer when needed;
- include metadata/version.

### 40. Нельзя позволять storage latency блокировать ввод пользователя

Session recording systems прямо учитывают latency.

Правильно:

- async writer queue;
- bounded buffer;
- degraded mode when queue full;
- metrics and warning.

### 41. Нельзя replay-ить без original terminal size

Без rows/cols wrapping изменится, и output команды будет выглядеть иначе.

Правильно:

- persist resize events;
- persist segment rows/cols range;
- player can show "recorded at 120x30, current 80x24".

### 42. Нельзя считать clear/delete одним действием

Clear screen может быть частью output, а delete history - user privacy action.

Правильно:

- clear event remains in journal;
- delete creates deletion/tombstone and removes payload;
- UI distinguishes `screen cleared` from `history deleted`.

### 43. Нельзя индексировать search прямо по raw bytes

Raw bytes содержат ANSI/OSC/control sequences и partial UTF-8.

Правильно:

- parser-derived text chunks;
- index versioning;
- rebuild index after parser upgrade;
- raw remains source for replay.

### 44. Нельзя делать одну global quota без приоритетов

Одинаковое pruning для failed command, bookmarked block и huge background log плохое.

Правильно:

- smart retention;
- pin/bookmark support;
- failed commands retained longer;
- old unattributed noise pruned first.

### 45. Нельзя скрывать, что данные sensitive

Terminal transcript часто содержит paths, tokens, usernames, hostnames, production logs.

Правильно:

- clear private controls visible;
- redact before AI/share/export;
- per-session private mode;
- warnings for sync/export.

### 46. Нельзя делать один behavior при сбое записи истории

Для обычной разработки лучше продолжить с warning. Для compliance режима, наоборот, нельзя продолжать без записи.

Правильно:

- history policy: `best_effort` или `strict`;
- strict mode blocks/ends session when writer fails;
- best effort shows degraded state and resumes writing when possible;
- tests for disk full / db locked / writer crash.

### 47. Нельзя делать FTS/search index source of truth

Search index может быть stale, corrupted или построен старым parser/redaction version.

Правильно:

- raw segments/events/snapshots - source of truth;
- search chunks - derived;
- index freshness/version visible;
- rebuild index from raw journal.

### 48. Нельзя считать unlimited scrollback надежным хранением

Renderer scrollback живет в памяти, зависит от terminal emulator и может тормозить resize.

Правильно:

- UI scrollback ограничен;
- durable journal disk-backed;
- history paging из БД.

### 49. Нельзя строить search по wrapped visual lines

Resize меняет wrapping. Один и тот же output при 80 и 160 columns будет иметь разные visual lines.

Правильно:

- search по logical/parser-derived chunks;
- persist resize events for replay;
- store recorded cols/rows for visual reconstruction.

### 50. Нельзя считать encryption заменой redaction

Encrypted DB защищает at-rest файл, но не защищает export, AI context, screenshots, logs и live UI.

Правильно:

- redaction before export/AI/share;
- private mode;
- encrypted store as separate optional layer;
- key management design before cloud/sync.

### 51. Нельзя удалять sensitive data только из raw table

Та же строка может быть в snapshots, search chunks, FTS, exports, backups, local cache.

Правильно:

- deletion scopes cover raw, events, snapshots, search, exports, cache;
- tombstone/deletion audit;
- compaction/VACUUM policy for local DB;
- backup deletion policy.

### 52. Нельзя делать export из visible buffer

Visible buffer может быть truncated, wrapped, alt-screen-only или уже affected by restore.

Правильно:

- export from journal/read model;
- include provenance and redaction profile;
- export formats: markdown, asciicast-like, plain text, JSON debug.

### 53. Нельзя откладывать flush до завершения команды

Long-running command может идти часами. Crash потеряет весь output.

Правильно:

- flush by size/time;
- command boundary is extra flush trigger, not only trigger;
- partial segments valid.

### 54. Нельзя использовать только wall-clock для ordering

Clock может прыгнуть, а события из разных panes идут конкурентно.

Правильно:

- monotonic sequence per session/pane;
- timestamp only metadata;
- sequence ranges for attribution.

### 55. Нельзя считать parser immutable

ANSI/parser bugs будут фикситься, derived text/search может измениться.

Правильно:

- parser_version on derived records;
- raw bytes retained;
- rebuild derived read models.

### 56. Нельзя забыть про terminal mode state

Bracketed paste, mouse tracking, alt screen, cursor visibility и title влияют на replay.

Правильно:

- persist mode changes or snapshot terminal modes;
- replay in inert mode;
- mode-aware projection.

### 57. Нельзя записывать history viewer в ту же history

Если user смотрит transcript внутри terminal pane, output viewer может попасть в journal.

Правильно:

- viewer outside recorded terminal stream;
- recording disabled during replay;
- provenance marks for generated debug output.

### 58. Нельзя смешивать exact recording и user-friendly transcript

Exact recording должен сохранять control sequences. User-friendly transcript должен быть очищенным text view.

Правильно:

- raw replay artifact;
- rendered/search transcript;
- markdown export;
- all linked by sequence/provenance.

### 59. Нельзя начинать sync/cloud без deletion and encryption model

История терминала содержит secrets. Sync без E2E encryption и deletion semantics опасен.

Правильно:

- local-first;
- stable IDs/tombstones;
- encryption/redaction first;
- sync later.

### 60. Нельзя считать "команда завершилась" только по появлению prompt

Prompt может быть перерисован темой, subshell может вложиться, TUI может перейти в alt-screen, background jobs продолжают писать.

Правильно:

- prefer shell integration finish marker;
- fallback prompt detection is heuristic;
- background output blocks;
- status confidence.

### 61. Нельзя считать terminal text обычной строкой Unicode

Cell width, combining marks, emoji и CJK влияют на cursor и wrapping.

Правильно:

- persist parser/unicode/cell-width versions;
- raw bytes source of truth;
- derived text rebuildable;
- tests with emoji/CJK/combining marks.

### 62. Нельзя забыть про terminal images/media protocols

Kitty/iTerm2/WezTerm/Sixel output может содержать images или binary payload.

Правильно:

- stream segments are bytes-aware;
- optional media artifacts;
- export/replay policy for media;
- redaction/delete covers media.

### 63. Нельзя считать paste обычным typing

Paste может содержать много строк и secrets, а bracketed paste явно отделяет его от typing.

Правильно:

- input source attribution;
- pasted content sensitive by default;
- multi-line paste can create multiple command blocks;
- raw paste logging opt-in/redacted.

### 64. Нельзя считать checksums достаточными для audit

Checksum ловит corruption, но не доказывает, что запись не изменяли намеренно.

Правильно:

- default: checksum per segment;
- strict/audit: hash chain and optional signature;
- store previous_hash/chain_hash.

### 65. Нельзя удалять password только из command text

Password/token может быть в raw input, output echo, search index, snapshot, media artifact, export.

Правильно:

- redaction pipeline applies to all derived/read models;
- redaction versioning;
- password-aware recording similar to sudo session recording lessons.

### 66. Нельзя делать replay unsafe terminal sequences by default

Historical replay может содержать OSC 52 clipboard, images, hyperlinks, window controls.

Правильно:

- inert replay mode;
- disable clipboard/window side effects;
- image/media display behind policy;
- links require explicit click.

### 67. Нельзя считать one paste == one command

Paste может содержать here-doc, multi-line script, newlines, shell continuations.

Правильно:

- command blocks created by shell execution markers;
- input events can span multiple commands;
- multiline command handling.

### 68. Нельзя игнорировать sudo/subprocess command logging distinction

Shell command block `sudo bash` может запустить много subcommands, которые не видны shell history.

Правильно:

- command block is user shell lifecycle;
- subcommand/audit visibility requires separate integration/policy;
- don't overclaim command-level completeness without shell/subprocess support.

### 69. Нельзя хранить audit chain только в той же mutable DB без export/backup strategy

Если DB редактируемая, hash chain помогает detect tampering, но не решает trusted timestamp/storage.

Правильно:

- hash chain first;
- optional signed checkpoints;
- exported audit bundle can include root hash;
- strict mode documents threat model.

### 70. Нельзя делать markdown export "полной историей"

Markdown удобен, но теряет timing, cursor movements, ANSI, media и exact bytes.

Правильно:

- markdown export = readable summary;
- asciicast-like/export JSON = replay artifact;
- raw debug bundle = forensic artifact.

### 71. Нельзя считать raw input bytes достаточным объяснением клавиш

Keyboard protocol может быть legacy, CSI-u, kitty, modifyOtherKeys; modifiers кодируются по-разному.

Правильно:

- store input protocol/capability profile;
- for debug, store redacted input event metadata;
- command text from shell integration remains primary user-facing source.

### 72. Нельзя смешивать mouse reporting и клики по нашему UI

Mouse reporting inside terminal отправляет escape sequences в PTY, а click по command block - действие UI.

Правильно:

- separate input_source values;
- historical replay disables mouse side effects;
- UI clicks are not terminal input.

### 73. Нельзя делать parser/replay без golden fixtures

Terminal parser bugs незаметно ломают snapshots, search и output attribution.

Правильно:

- vttest-like fixtures;
- raw bytes -> expected screen;
- raw bytes -> expected events;
- parser_version on derived models.

### 74. Нельзя считать PowerShell Start-Transcript полноценной заменой

Это shell transcript, не terminal/mux/session journal.

Правильно:

- use as optional import/export reference;
- keep terminal journal as source of truth;
- don't rely on shell transcript for panes/resize/alt-screen.

### 75. Нельзя делать semantic history недоступной для keyboard/screen reader

Command blocks только как visual cards ухудшат доступность.

Правильно:

- keyboard navigation between blocks;
- accessible labels with command/status/cwd/duration;
- restore/degraded banners announced.

### 76. Нельзя не хранить capability context

Без TERM/backend/keyboard/shell integration/parser versions невозможно объяснить качество restore.

Правильно:

- terminal_capability_profiles;
- show fidelity level;
- use profile in replay/debug reports.

### 77. Нельзя replay-ить input events в живой shell автоматически

Keyboard/mouse/paste events могут быть destructive.

Правильно:

- historical input replay only in inert player;
- live rerun only command-level and user-confirmed;
- never auto-replay mouse/input stream into live process.

### 78. Нельзя считать accessibility поздним UI polish

Если architecture не хранит semantic blocks, accessibility придется делать эвристиками.

Правильно:

- block IDs and metadata in runtime API;
- accessible timeline from day one;
- search/jump commands by block.

### 79. Нельзя забыть про terminfo/TERM mismatch

Программа может выводить sequences под один `TERM`, а restore/replay происходит в другом profile.

Правильно:

- capture TERM and terminal capability profile;
- restore uses recorded parser assumptions;
- show mismatch warning in forensic replay.

### 80. Нельзя считать "save visible text" history feature

Save contents полезен, но это export snapshot, не durable journal.

Правильно:

- export from read model;
- journal for restore;
- snapshot for hydration.

### 81. Нельзя терять capture layer

Одно и то же событие может прийти из UI, shell marker, PTY, ConPTY, projection, mux API или audit layer.

Правильно:

- `capture_layer` на journal events;
- trust/confidence зависит от слоя;
- UI показывает fidelity.

### 82. Нельзя логировать input при ECHO off как обычный текст

ECHO off часто означает password prompt.

Правильно:

- redact input when ECHO off;
- raw input off by default;
- Windows/ConPTY path conservative when echo state unknown.

### 83. Нельзя считать reconnect, restore и recording одним feature

Mosh/ET/dtach решают live continuity или reattach, но не полноценный searchable transcript.

Правильно:

- separate product labels;
- separate restore semantics;
- process state matrix.

### 84. Нельзя считать ConPTY resize UI-only

Windows ConPTY получает explicit resize API, и process output зависит от этого.

Правильно:

- persist resize events;
- include initial rows/cols;
- Windows fixtures for resize/reflow.

### 85. Нельзя думать, что dtach-like attach сохранит экран

dtach сохраняет возможность reattach к process, но не полноценный scrollback/history.

Правильно:

- live attach path separate from visual restore;
- if no screen history, show it honestly.

### 86. Нельзя игнорировать line discipline modes

ICANON/raw mode меняют смысл input: shell line editing vs app/TUI direct input.

Правильно:

- capture mode changes when possible;
- command block attribution disabled/degraded in raw/TUI mode;
- input events sensitive.

### 87. Нельзя строить process persistence на одном bool

`preserves_process_state` слишком груб: attach к live mux, remote reconnect, daemon-managed native и restarted shell имеют разные гарантии.

Правильно:

- restore process mode enum;
- `live_attached`, `restarted`, `historical_only`, `remote_reconnected`, `unknown`;
- UI copy explains mode.

### 88. Нельзя считать Mosh-style state sync заменой истории

State sync быстро приводит экран к актуальному состоянию, но не дает command blocks, search и durable transcript.

Правильно:

- use snapshots for fast convergence;
- journal for history/search/replay.

### 89. Нельзя считать outer mux stream pane transcript

tmux/zellij raw output includes mux UI/control and may not represent one shell pane.

Правильно:

- use structured mux APIs where possible;
- pane-level surface/output source;
- outer stream only debug/secondary.

### 90. Нельзя считать OSC passthrough через tmux/zellij гарантированным

Mux can filter, wrap or require configuration for passthrough.

Правильно:

- detect shell integration inside pane;
- quality per pane;
- fallback to pane snapshots/deltas.

### 91. Нельзя хранить cwd как простой trusted string

CWD can include host, remote path, usernames and sensitive project names.

Правильно:

- cwd_source/cwd_host/cwd_trust;
- redact/export policy;
- parse OSC 7 as URI safely.

### 92. Нельзя считать viewport session truth при multi-client

Different clients can have different sizes and scroll positions.

Правильно:

- pane/process size separate from client viewport;
- terminal_client_views table;
- replay with recorded pane size.

### 93. Нельзя смешивать read-only viewers с active users

Viewers may see output but not input. Their clipboard/export permissions can differ.

Правильно:

- client permissions;
- audit who initiated command vs who viewed;
- read-only mode cannot create input events.

### 94. Нельзя auto-trust mux plugin/pipe metadata

Zellij/tmux plugin data can be useful but is another trust boundary.

Правильно:

- capture_layer = mux_api/plugin;
- validate schemas;
- trust level separate from shell nonce.

### 95. Нельзя replay-ить OSC 52 clipboard across shared clients

Clipboard is local to one client and privacy-sensitive.

Правильно:

- inert historical replay;
- no clipboard side effects;
- per-client event if allowed.

### 96. Нельзя считать tmux control mode доступным всегда

User may use old tmux, restricted environment, nested mux or no control mode.

Правильно:

- feature detection;
- fallback modes;
- explicit backend capabilities.

### 97. Нельзя делать command rerun без remote context

Same command in local shell vs SSH/container/WSL may do different things.

Правильно:

- terminal_remote_contexts;
- rerun only in explicit current context;
- warning on context mismatch.

### 98. Нельзя забыть про nested mux

tmux inside zellij, zellij inside SSH, shell inside container can layer semantics.

Правильно:

- context stack, not single backend string;
- capture layer on every event;
- degraded fidelity when nesting unknown.

### 99. Нельзя считать read-only history harmless

Read-only user can still see secrets in output.

Правильно:

- view permissions;
- redaction;
- private sessions excluded from sharing.

### 100. Нельзя строить one-size-fits-all restore для native/zellij/tmux/remote

Each backend has different guarantees.

Правильно:

- backend-specific restore semantics;
- common journal/read model;
- UI explains exact mode.

### 101. Нельзя считать projections source of truth

Command blocks/search chunks/snapshots могут быть stale или построены старым parser.

Правильно:

- raw segments/events are source of truth;
- projections have versions;
- rebuild path is tested.

### 102. Нельзя делать parser upgrade без projection invalidation

Новый parser может иначе обработать ANSI, Unicode width, line wrapping and search text.

Правильно:

- parser_version on projections;
- mark stale on upgrade;
- background rebuild.

### 103. Нельзя показывать hostile transcript как обычный trusted UI text

Output can contain ANSI, control chars, bidi marks, fake prompts and fake links.

Правильно:

- inert viewer;
- escape/control char visualization;
- suspicious Unicode flags;
- safe link rendering.

### 104. Нельзя считать checksum tamper-proof

Checksum detects corruption, not malicious modification.

Правильно:

- checksum for default integrity;
- hash chain/signed checkpoint for strict audit;
- validation command.

### 105. Нельзя удалять raw history and keep derived projections

Search index or markdown export can still contain deleted secrets.

Правильно:

- delete propagates to projections, exports, cache;
- deletion audit/tombstone;
- compaction policy.

### 106. Нельзя делать replay/debug без schema versions

Old debug bundle might not replay with current parser/schema.

Правильно:

- schema versions;
- upcasters;
- compatibility matrix.

### 107. Нельзя считать "safe export" simple text dump

Text dump can contain hidden bidi/control chars, forged log lines or misleading links.

Правильно:

- export sanitizer;
- optional raw debug bundle separate;
- include redaction profile.

### 108. Нельзя не иметь lifecycle jobs

History DB needs maintenance: checkpoints, optimize, prune, rebuild, verify.

Правильно:

- observable maintenance jobs;
- status in UI;
- tests for stale/rebuild/degraded.

### 109. Нельзя игнорировать retention by class

Audit logs, private sessions, bookmarked blocks and temporary debug output need different retention.

Правильно:

- per class retention;
- user visible policy;
- smart pruning.

### 110. Нельзя trust visual command text when Unicode spoofing is possible

Command may contain bidi/control/confusable chars and look different than bytes.

Правильно:

- show raw/escaped view on demand;
- flag suspicious Unicode;
- rerun requires explicit confirmation for suspicious command.

### 111. Нельзя считать shell command blocks достаточными внутри REPL

`python`, `node`, `psql`, `mysql` and similar apps have their own prompts and histories.

Правильно:

- command domains;
- outer command block plus inner transcript;
- app integrations optional/future;
- heuristic REPL blocks clearly marked.

### 112. Нельзя заменять terminal transcript app history files

IPython/Node/psql/Python history stores input, but not terminal output, resize, panes, trust or replay state.

Правильно:

- app history can enrich/import;
- terminal journal remains source of truth.

### 113. Нельзя требовать OS audit for normal history

Windows 4688, Sysmon and Linux audit need privileges/config and miss built-ins/REPL commands.

Правильно:

- process correlation optional;
- shell integration + journal baseline;
- degraded when audit unavailable.

### 114. Нельзя хранить huge output/media только external files без DB lifecycle

External artifacts complicate backup, delete, integrity and export.

Правильно:

- DB metadata for artifacts;
- content hashes;
- backup includes artifact store;
- delete propagates to artifacts.

### 115. Нельзя делать unbounded SQLite BLOBs

Huge rows hurt memory, pruning, replay and corruption recovery.

Правильно:

- bounded stream segments;
- optional incremental BLOB I/O;
- external artifact threshold.

### 116. Нельзя оставлять event attributes ad hoc forever

Unstructured JSON без stable names ломает queries, migrations and compatibility.

Правильно:

- event_name + attributes schema;
- schema version;
- OTel-like naming discipline.

### 117. Нельзя терять correlation IDs

UI submit, shell marker, process creation, output segments and exports должны связываться.

Правильно:

- command_block_id/correlation_id;
- source references;
- trace across projections.

### 118. Нельзя считать process command line equal user command

Shell expands aliases/functions, built-ins may not spawn, and subprocess command line can differ.

Правильно:

- user command text separate from process command line;
- process correlation is enrichment.

### 119. Нельзя делать REPL prompt detection high-trust по regex

Many outputs look like prompts.

Правильно:

- app integration or shell/app markers for high trust;
- regex prompt detection is heuristic;
- UI shows confidence.

### 120. Нельзя проектировать storage без artifact threshold

All-in-SQLite and all-files approaches both fail at extremes.

Правильно:

- default small/medium segments in SQLite;
- large/media external artifacts;
- consistent backup/delete/integrity.

### 121. Нельзя класть hot SQLite history DB в ненадежную cloud/network папку по умолчанию

Broken locks and sync races can corrupt SQLite.

Правильно:

- default local app data path;
- detect cloud/network/removable storage when possible;
- warn/block in strict mode.

### 122. Нельзя делать backup простым копированием `.sqlite3`

WAL/SHM and live writer make raw file copy incomplete.

Правильно:

- SQLite backup API;
- checkpoint policy;
- artifact manifest;
- restore test.

### 123. Нельзя считать corruption all-or-nothing failure

Часть segments/sessions может быть recoverable.

Правильно:

- recovery path;
- quarantine corrupt segments;
- rebuild projections;
- report lost ranges.

### 124. Нельзя хранить redaction as destructive black box

После redaction надо понимать, что, где и каким правилом было изменено.

Правильно:

- terminal_redaction_findings;
- rule/profile versions;
- no raw secret stored in finding.

### 125. Нельзя обещать, что secret scanning поймает все

Patterns have false positives and false negatives.

Правильно:

- `possibly_sensitive`, `scan_failed`, `not_scanned` states;
- user controls;
- export warnings.

### 126. Нельзя сканировать только raw output and ignore projections

Secrets can remain in search chunks, snapshots, exports and AI context.

Правильно:

- redaction applies to raw + derived + artifacts;
- projection rebuild after redaction.

### 127. Нельзя делать strict mode без storage preflight

Strict history is meaningless if storage location/locking/checkpointing is unsafe.

Правильно:

- storage preflight;
- lock/backup/checkpoint checks;
- fail before opening shell if required.

### 128. Нельзя скрывать DB health from user

Если writer lagging/corrupt/degraded, user must know history may be incomplete.

Правильно:

- visible storage health;
- diagnostics command;
- writer lag and last error.

### 129. Нельзя хранить external artifacts без garbage collection

DB rows may be deleted while files remain.

Правильно:

- artifact reference counting;
- GC job;
- backup/delete tests.

### 130. Нельзя делать export/share до redaction pass

Terminal transcript likely contains secrets.

Правильно:

- export requires redaction profile;
- warnings for raw debug bundles;
- optional block/export when high-confidence secret detected.

### 131. Нельзя полагаться на SQLite defaults

Foreign keys can be off, busy timeout absent, synchronous/journal mode may not match our durability expectations.

Правильно:

- set PRAGMAs on every connection;
- assert effective settings;
- record store profile.

### 132. Нельзя делать cascades without `foreign_keys=ON`

SQLite will not enforce foreign keys unless enabled.

Правильно:

- PRAGMA foreign_keys=ON;
- delete tests for sessions, panes, artifacts, projections.

### 133. Нельзя compress entire session as one blob

It kills random access, partial recovery, pruning and command-level export.

Правильно:

- bounded segment compression;
- seekable format only for cold archival if needed;
- index by sequence.

### 134. Нельзя использовать один compression profile для hot and cold data

Live output needs low latency; old logs may prefer ratio.

Правильно:

- hot profile: none/LZ4/small zstd;
- cold profile: zstd higher ratio;
- background recompression job.

### 135. Нельзя делать migrations без old DB fixtures

Schema will evolve; untested migrations break user history.

Правильно:

- fixtures for previous schema versions;
- migrate forward tests;
- projection stale/rebuild tests.

### 136. Нельзя забыть, что Diesel migrations не заменяют DB invariants

Typed ORM does not automatically guarantee PRAGMAs, foreign keys, WAL or retention.

Правильно:

- store initialization tests;
- repository invariants;
- integration tests with real SQLite file.

### 137. Нельзя использовать DEFERRED transaction blindly for writer batches

Contention may surface mid-batch.

Правильно:

- consider BEGIN IMMEDIATE for writer;
- degraded state on lock contention;
- retry/backoff policy.

### 138. Нельзя делать WAL checkpoint manually only

Long-running app needs observable checkpoint behavior.

Правильно:

- checkpoint policy;
- WAL size metrics;
- last_checkpoint_at_ms.

### 139. Нельзя менять compression/parser without compatibility metadata

Old segments must still replay.

Правильно:

- algorithm/version stored per segment;
- parser_version on projections;
- compatibility tests.

### 140. Нельзя считать strict durability free

FULL synchronous/strict mode can add latency.

Правильно:

- best_effort vs strict profiles;
- UI explains tradeoff;
- writer latency metrics.

### 141. Нельзя хранить ключ шифрования рядом с DB plain text

Encrypted DB with key in the same app folder is mostly theater.

Правильно:

- OS-backed secret storage;
- key rotation metadata;
- dev-only insecure provider clearly marked;
- backup/export excludes raw keys.

### 142. Нельзя смешивать data, state, cache и runtime

Если DB, projection cache, sockets and temp exports лежат в одной куче, backup/prune/cleanup начнут ломать историю.

Правильно:

- OS folder policy;
- disposable cache;
- durable state separately;
- runtime files outside backups.

### 143. Нельзя говорить "надежно" без SLI/SLO

Надежность history должна измеряться, а не ощущаться.

Правильно:

- durability target;
- restore latency target;
- recovery target;
- privacy/redaction target.

### 144. Нельзя игнорировать WebSocket backpressure

Browser can buffer outbound/inbound data while UI looks alive. Это скрывает lag and memory pressure.

Правильно:

- track buffered bytes;
- slow/stop producers before memory blowup;
- mark stream degraded on overflow;
- test long-output bursts.

### 145. Нельзя считать writer queue бесконечной

Unbounded writer queue eventually becomes memory leak or delayed data loss.

Правильно:

- bounded channels;
- explicit overflow behavior;
- metrics;
- strict mode that prefers slowdown over loss.

### 146. Нельзя тестировать terminal parser только happy path

ANSI/OSC/UTF-8/control streams are adversarial by nature.

Правильно:

- cargo-fuzz targets;
- malicious OSC fixtures;
- partial UTF-8 segments;
- alternate screen and resize sequences.

### 147. Нельзя запускать redaction без property/fuzz tests

Secrets often cross segment boundaries or get transformed by control sequences.

Правильно:

- segment-boundary fixtures;
- redaction profile versions;
- search/export indexes scanned too;
- no raw secret in derived artifacts.

### 148. Нельзя принимать crash recovery без chaos tests

SQLite/WAL/projection logic can pass unit tests and still lose edge cases on crash.

Правильно:

- kill process during batch;
- corrupt projection cache;
- lock DB during burst;
- verify rebuild/quarantine path.

### 149. Нельзя хранить sync/export tokens в обычном config file

Terminal history sync/export tokens are as sensitive as many app passwords.

Правильно:

- Credential Manager/DPAPI/Keychain/Secret Service;
- token rotation;
- missing-token recovery UX;
- no tokens in logs/history exports.

### 150. Нельзя считать derived cache loss data loss

Projection snapshots and serialized buffers are rebuildable. Canonical journal is not.

Правильно:

- label derived artifacts;
- rebuild projections from journal;
- quarantine broken cache;
- user sees degraded restore only when canonical data is incomplete.

### 151. Нельзя делать export format внутренней DB-моделью

asciicast/jsonseq удобны для переноса, но internal history needs richer indexes, trust, policy, redaction and projections.

Правильно:

- internal normalized journal;
- export adapters;
- import quarantine;
- schema validation.

### 152. Нельзя хранить события как ad-hoc JSON без envelope

Через год станет непонятно, откуда событие, какой schema version, как его correlate and replay.

Правильно:

- `event_id`;
- `event_type`;
- `source`;
- schema version;
- correlation IDs.

### 153. Нельзя включать sync до определения conflict semantics

Multi-device history может конфликтовать по retention, deletion, redaction, session identity and clock skew.

Правильно:

- local restore first;
- backup/PITR separately;
- sync with explicit merge/delete rules later.

### 154. Нельзя считать PITR session restore

Point-in-time DB recovery возвращает файлы БД, а не live process state and not necessarily UX context.

Правильно:

- separate labels;
- restore from backup into quarantine first;
- user chooses sessions to recover;
- no auto-run from recovered history.

### 155. Нельзя полагаться на default SQLite limits для hostile imports

Defaults are generous and suitable for trusted local use, not arbitrary transcript imports.

Правильно:

- set runtime limits;
- cap import/search/export payloads;
- stream large files;
- reject or quarantine oversized records.

### 156. Нельзя забывать SQLite defensive/trusted schema config

Imported or attacker-controlled DB files can abuse schema-level features if opened carelessly.

Правильно:

- defensive mode where possible;
- trusted schema off for untrusted DB handling;
- never load extensions for imports;
- copy/validate into our schema.

### 157. Нельзя превращать PowerShell command text в generic argv

PowerShell parsing is not POSIX shell parsing and not Windows `CommandLineToArgvW` parsing.

Правильно:

- store shell/domain;
- store exact text;
- store parsing context;
- rerun through same shell with confirmation if uncertain.

### 158. Нельзя смешивать displayed command и executed command

Aliases, functions, wrappers, shell expansion and native argument passing can diverge.

Правильно:

- command text source;
- process correlation separately;
- trust score;
- UI labels for "entered" vs "observed process".

### 159. Нельзя шифровать external artifacts без authentication

Plain encryption may hide content but not reliably detect truncation/reorder/tampering.

Правильно:

- AEAD/secretstream-style chunks;
- final marker;
- chunk sequence;
- metadata hash in DB.

### 160. Нельзя считать grapheme = char = byte

Search/copy/redaction can split emoji, combining marks and complex scripts if based on bytes/scalars only.

Правильно:

- grapheme-aware text views;
- byte offsets for raw;
- display offsets for UI;
- cell width policy for terminal grid.

### 161. Нельзя делать redaction only on visible text

Secret can exist in raw segment, FTS index, export, artifact, screenshot or serialized projection.

Правильно:

- scan raw and derived stores;
- record redaction findings;
- rebuild derived indexes after redaction;
- export only after redaction pass.

### 162. Нельзя импортировать чужой transcript прямо в active session

Imported history is untrusted historical data, not live terminal output.

Правильно:

- import into quarantine;
- validate schemas/checksums;
- render inert;
- never execute imported commands automatically.

### 163. Нельзя позволять search query съесть UI thread

Huge FTS query, wildcard abuse or oversized paste can freeze terminal UX.

Правильно:

- query length limit;
- timeout/cancel;
- background search worker;
- partial results.

### 164. Нельзя делать schema migration без projection invalidation

Если event schema/parser/redaction изменились, old search chunks and snapshots may be stale.

Правильно:

- projection version;
- rebuild queue;
- stale markers;
- migration tests with old fixtures.

### 165. Нельзя считать backup privacy-neutral

Backups can retain deleted/redacted terminal secrets longer than active DB.

Правильно:

- backup retention policy;
- tombstones/deletion propagation;
- encrypted backups;
- user-visible export/backup inventory.

### 166. Нельзя считать `DELETE FROM` приватным удалением

Logical delete can leave sensitive bytes in free pages, WAL, backups, FTS and artifacts.

Правильно:

- deletion workflow;
- checkpoint/compaction policy;
- derived rebuild;
- backup/export retention check.

### 167. Нельзя забывать temp files

SQLite temp files, export temp dirs and decoded media caches can contain secrets.

Правильно:

- temp artifact registry;
- OS cache/runtime paths;
- cleanup job;
- temp quota and failure reporting.

### 168. Нельзя хранить SQLCipher key in process config

Encrypted DB loses value if passphrase sits in plaintext next to config/logs.

Правильно:

- keyring/DPAPI/Credential Manager;
- secret refs;
- rotation/rekey plan;
- no Debug/log of key material.

### 169. Нельзя логировать secrets через `Debug`

Typed wrappers and ORM models can accidentally print tokens/keys/commands.

Правильно:

- secret wrapper types;
- redacted debug output;
- log review tests;
- tracing fields classified.

### 170. Нельзя обещать perfect zeroization

Zeroizing buffers helps, but runtime copies, OS paging and libraries can still retain data.

Правильно:

- best-effort zeroize;
- minimize lifetime;
- document limits;
- rely on OS secret storage and encryption at rest too.

### 171. Нельзя делать terminal history inaccessible canvas

Если history exists only in rendered grid/canvas, screen reader and keyboard navigation suffer.

Правильно:

- semantic command-block log;
- accessible status messages;
- keyboard navigation;
- restore boundary labels.

### 172. Нельзя trap keyboard внутри terminal pane

Terminal wants raw keys, but web app must still let keyboard users leave/focus history/search.

Правильно:

- explicit escape shortcut;
- visible focus;
- no-keyboard-trap tests;
- raw mode vs app navigation mode.

### 173. Нельзя auto-preview binary terminal artifacts

Images/files from terminal output can be huge, sensitive or malicious.

Правильно:

- quarantine unknown protocols;
- size caps;
- lazy preview;
- checksum and scan metadata.

### 174. Нельзя считать media output plain text redaction-safe

Secrets can be inside images, QR codes, hyperlinks, filenames and binary metadata.

Правильно:

- artifact redaction state;
- export warnings;
- optional OCR/metadata scanning later;
- user confirmation for raw artifact export.

### 175. Нельзя делать retention policy только по дням

Terminal history risk depends on secrets, workspace, bookmarks, failed commands and output size.

Правильно:

- multi-dimensional policy;
- session/workspace scopes;
- bookmark exceptions;
- possible-secret priority.

### 176. Нельзя удалять active running command history mid-stream silently

Prune during running command can break sequence ranges and attribution.

Правильно:

- never prune active ranges;
- mark pending prune;
- close block/segment first;
- then apply retention.

### 177. Нельзя использовать одинаковые metric buckets для всего

Writer commit, replay catchup, checkpoint and redaction have different latency expectations.

Правильно:

- product-specific buckets;
- percentiles per operation;
- SLO-linked alerts;
- degraded UI state.

### 178. Нельзя скрывать backup/export inventory от пользователя

User may delete active history but old exports/backups still contain secrets.

Правильно:

- export manifest;
- backup inventory;
- retention review;
- warnings before "deleted everywhere" claims.

### 179. Нельзя считать accessible announcements free

Live output can spam assistive tech if announced raw.

Правильно:

- dedupe;
- polite/assertive priority;
- summarize high-volume changes;
- user settings for verbosity.

### 180. Нельзя превращать artifact missing в silent broken restore

DB may reference external media/chunks that disappeared.

Правильно:

- artifact integrity check;
- missing artifact state;
- partial restore UI;
- repair/quarantine path.

### 181. Нельзя делать projection rebuild через memory-only task

App restart can lose the job and leave stale search/snapshots forever.

Правильно:

- durable background jobs;
- idempotent workers;
- retry/backoff;
- visible failed jobs.

### 182. Нельзя отправлять export/sync after commit без outbox

Если commit прошел, а process умер before network/export job, state becomes inconsistent.

Правильно:

- transactional outbox;
- worker claims;
- result rows;
- retry/quarantine.

### 183. Нельзя делать Tantivy/FTS canonical source

Search index can be stale, partial, redacted differently or deleted.

Правильно:

- raw journal canonical;
- search index derived;
- rebuild by version;
- redaction invalidates index.

### 184. Нельзя выбирать external search service для local-first MVP

It adds auth, network, ops, privacy and failure modes before product needs it.

Правильно:

- SQLite/FTS baseline;
- Tantivy as local derived scale-up;
- external service only for server product.

### 185. Нельзя применять CRDT к raw transcript blindly

Terminal output order, privacy deletion and audit integrity do not behave like collaborative text.

Правильно:

- append-only event log for transcript;
- CRDT only for metadata/annotations/layout;
- explicit delete/tombstone semantics.

### 186. Нельзя хранить only wall-clock timestamps

Wall clock can jump, skew or differ across machines.

Правильно:

- wall time for UX;
- monotonic deltas for duration/replay;
- source clock metadata;
- imported/remote quality flags.

### 187. Нельзя persist raw Rust `Instant`

`Instant` is process-local/opaque and not portable across restarts.

Правильно:

- persist elapsed durations/deltas;
- anchor to session start;
- store wall-time anchor separately.

### 188. Нельзя давать AI raw terminal history без policy

Terminal output can prompt-inject model and expose secrets.

Правильно:

- redaction first;
- provenance/ranges;
- trust labels;
- bounded context packet.

### 189. Нельзя терять provenance при AI/RAG export

Without session/pane/seq/block IDs, AI answers cannot be audited.

Правильно:

- context export records;
- exact stream ranges;
- redaction profile;
- consumer kind and expiry.

### 190. Нельзя мигрировать unknown SQLite file как нашу DB

Wrong file/open path can corrupt unrelated user data.

Правильно:

- `application_id`;
- `user_version`;
- read-only quarantine on mismatch;
- migration only through owned store.

### 191. Нельзя игнорировать tokenizer choice

Commands, paths, flags, URLs, Unicode and logs need different tokenization.

Правильно:

- search profiles;
- path-aware/trigram options;
- tokenizer tests;
- query limits.

### 192. Нельзя считать substring search бесплатным

Trigram/substring indexing increases storage and update cost.

Правильно:

- enable per profile;
- size metrics;
- rebuild jobs;
- cap query complexity.

### 193. Нельзя делать background jobs non-idempotent

Retries after crash can duplicate artifacts, corrupt projections or double-delete.

Правильно:

- target refs;
- deterministic outputs;
- unique constraints;
- compare-and-swap state transitions.

### 194. Нельзя смешивать app event sync and DB backup

They solve different recovery problems and have different conflicts.

Правильно:

- separate checkpoint kinds;
- separate UI labels;
- separate restore flows;
- no live-process promises.

### 195. Нельзя считать command history AI-ready by default

AI needs curated, redacted, cited context, not raw scrollback dump.

Правильно:

- command block summaries;
- source citations;
- redaction/trust metadata;
- explicit user action for sensitive sessions.

### 196. Нельзя убивать Windows session только по child PID

Child PID does not represent the full process tree, console group or ConPTY lifecycle.

Правильно:

- Job Objects where appropriate;
- graceful console control first;
- force terminate as explicit fallback;
- lifecycle events in journal.

### 197. Нельзя путать CTRL+C and force terminate

They have different user meaning and data-loss risk.

Правильно:

- separate event types;
- timeout/deadline;
- user-visible status;
- exit reason stored.

### 198. Нельзя закрывать ConPTY без shutdown semantics

Closing transport can differ from graceful process cancellation.

Правильно:

- ConPTY close event;
- process/job follow-up;
- orphan detection;
- Windows-specific tests.

### 199. Нельзя считать VSS semantic backup

Volume snapshot captures files, not app-level redaction, tombstones or export manifest semantics.

Правильно:

- app-level export/backup manifest;
- SQLite backup API for DB;
- artifact manifest;
- VSS only as external disaster-recovery layer.

### 200. Нельзя доверять file watcher as source of truth

Watcher events can be lost, coalesced, reordered or overflow.

Правильно:

- watcher as hint;
- periodic manifest scan;
- checksum verification;
- overflow triggers full rescan.

### 201. Нельзя делать content-addressed artifacts без deletion model

Deduped immutable blobs complicate privacy deletion and backups.

Правильно:

- reference counts;
- tombstones;
- backup retention checks;
- encrypted blobs for sensitive payloads.

### 202. Нельзя expose raw content hash as public share ID

Hashes can leak equality and identify known content.

Правильно:

- separate share IDs;
- access control;
- salted/private manifests where needed;
- threat review before sharing.

### 203. Нельзя обещать process checkpoint restore on Windows native baseline

CRIU-like checkpointing is not portable baseline behavior.

Правильно:

- honest native restore semantics;
- mux attach path separately;
- experimental checkpoint backend later;
- no auto-rerun to fake process restore.

### 204. Нельзя смешивать mux-continuum restore and native history restore

tmux/zellij can restore layout/processes differently from our journal.

Правильно:

- backend-specific guarantees;
- attach/resurrect state;
- transcript remains separate;
- UI shows which layer restored.

### 205. Нельзя именовать события как попало

Random event names make analytics, migrations and docs brittle.

Правильно:

- stable event taxonomy;
- OTel-like attributes;
- schema registry;
- compatibility tests.

### 206. Нельзя считать watcher-detected delete user intent

External cleanup, antivirus or sync conflict can remove files without user intent.

Правильно:

- mark missing/quarantined;
- never delete DB metadata immediately;
- ask/repair when needed;
- retain tombstone logic separate.

### 207. Нельзя считать product restore examples equal guarantees

Wave/WindTerm/Termius/iTerm2 features differ by backend, OS, mobile backgrounding and sync.

Правильно:

- document our exact guarantees;
- native vs mux matrix;
- degraded states;
- test each path.

### 208. Нельзя передавать ANSI/OSC raw into AI context

Terminal escape sequences can hide or reshape text and side effects.

Правильно:

- sanitize/neutralize;
- plain semantic text view;
- provenance metadata;
- untrusted-content boundary.

### 209. Нельзя считать historical output live state

AI/user can mistake restored old output for current process status.

Правильно:

- historical/live boundary;
- timestamps;
- restore badges;
- context labels for AI.

### 210. Нельзя делать artifact integrity only at export time

Broken artifacts should be detected before user needs restore/export.

Правильно:

- startup/scheduled checks;
- watcher-triggered scans;
- restore preflight;
- diagnostics UI.

### 211. Нельзя считать WebSocket reliable history channel

WebSocket preserves order on a live connection, but does not restore missed events after reconnect by itself.

Правильно:

- stream seq;
- client ack;
- replay from DB/window;
- visible unrecoverable gaps.

### 212. Нельзя путать DB durability and client delivery

Data can be durably saved but not visible in browser after disconnect.

Правильно:

- separate writer state;
- separate client delivery state;
- reconnect replay;
- UI gap markers.

### 213. Нельзя auto-send buffered input after offline reconnect

Buffered input may be stale or dangerous after context changed.

Правильно:

- input expiry;
- user confirmation;
- drop stale queued actions;
- journal offline state.

### 214. Нельзя считать cwd достаточным remote context

Remote/WSL paths require domain, host, distro and path style.

Правильно:

- execution domain table;
- domain-aware cwd;
- rerun in same domain;
- path translation metadata.

### 215. Нельзя смешивать WSL Linux command and Win32 command

WSL interop can run Windows tools from Linux and vice versa.

Правильно:

- `wsl_linux` vs `win32_from_wsl`;
- WSL distro/config;
- path translation state;
- provenance labels.

### 216. Нельзя экспортировать SSH config metadata blindly

Host aliases, jump hosts, forwarded ports and users are sensitive.

Правильно:

- remote metadata classification;
- redaction profiles;
- share/export warnings;
- local-only full details by default.

### 217. Нельзя запускать Persistence v2 global flip

Writer/migration/privacy bugs would affect every user/session.

Правильно:

- feature flags;
- gradual rollout;
- ops kill switch;
- diagnostics include flag state.

### 218. Нельзя оставлять stale feature flags навсегда

Old flags create impossible-to-reason execution paths.

Правильно:

- owner;
- expiry;
- cleanup ticket;
- tests for active flag combinations only.

### 219. Нельзя тестировать reconnect только refresh browser

Real failures include latency, loss, server restart, browser reload and mux disconnect.

Правильно:

- Toxiproxy/netem fixtures;
- explicit chaos scenarios;
- multi-pane reconnect tests;
- artifact capture.

### 220. Нельзя считать replay gap harmless

Even small output gap can hide errors/secrets/prompts.

Правильно:

- gap marker;
- export includes gap;
- full restore option;
- no AI context over unknown gaps by default.

### 221. Нельзя trust client ack for server retention

Client may ack visible data that is not safely persisted or later lost locally.

Правильно:

- retention based on durable store;
- client ack only for delivery state;
- replay window independent from DB retention.

### 222. Нельзя игнорировать SSH ControlMaster/mux details

SSH multiplexing can change connection lifecycle and attribution.

Правильно:

- record ssh domain/config alias;
- connection/mux metadata;
- redaction;
- reconnect tests.

### 223. Нельзя считать WSL filesystem same as Windows filesystem

Performance, case sensitivity and path semantics differ.

Правильно:

- path style metadata;
- filesystem domain;
- avoid naive path normalization;
- Windows/WSL test fixtures.

### 224. Нельзя отдавать AI context из remote sessions без remote privacy pass

Remote host/user/path/proxy details can leak infrastructure.

Правильно:

- remote metadata redaction;
- explicit user selection;
- provenance labels;
- safe defaults.

### 225. Нельзя считать transport chaos "later"

Reconnect reliability is part of history reliability for web terminal UX.

Правильно:

- chaos runs in milestone;
- CI/manual profiles;
- metrics for replay gaps;
- regression artifacts.

### 226. Нельзя менять event payload schema без upcaster

Old history will become unreadable or misinterpreted.

Правильно:

- schema registry;
- upcaster tests;
- old export/DB fixtures;
- compatibility state per codec.

### 227. Нельзя удалять protobuf-like fields без reservation policy

Reusing old field IDs/names can corrupt old data interpretation.

Правильно:

- reserved fields/names;
- migration docs;
- reader compatibility tests;
- reject unknown incompatible schema.

### 228. Нельзя считать binary format automatically better

Binary payloads save space but hurt debugging and migrations if schema discipline is weak.

Правильно:

- JSON first for churn;
- binary only for hot/high-volume payloads;
- schema/version/upcaster mandatory.

### 229. Нельзя добавлять index без query-owner

Indexes speed reads but slow writes and increase DB size.

Правильно:

- query rationale;
- write cost measured;
- partial indexes where useful;
- migration performance test.

### 230. Нельзя менять critical query without plan baseline

Large history can regress from index lookup to scan silently.

Правильно:

- `EXPLAIN QUERY PLAN` fixtures;
- large DB benchmark;
- plan change review;
- duration/rows scanned budget.

### 231. Нельзя забывать ANALYZE/optimizer maintenance

Planner stats can become stale after large import/delete.

Правильно:

- maintenance job;
- post-import/post-prune optimize;
- query latency metrics;
- stale stats diagnostics.

### 232. Нельзя отправлять crash dump before privacy gate

Crash diagnostics may include command/output/path/token data.

Правильно:

- local-only default;
- redaction first;
- user approval for raw bundle;
- private sessions excluded.

### 233. Нельзя класть raw command text in tracing baggage

Baggage propagates to downstream systems and vendors.

Правильно:

- IDs/hashes only;
- no cwd/host/user/token;
- allowlist attributes;
- telemetry scrub tests.

### 234. Нельзя считать logs less sensitive than history

Logs can contain SQL errors, paths, commands and snippets.

Правильно:

- classify log fields;
- scrub before persist/upload;
- support bundle manifest;
- retention policy for diagnostics.

### 235. Нельзя применять legal hold silently

User must understand why delete/prune does not remove data.

Правильно:

- visible hold state;
- scope/reason/expiry;
- audit record;
- release workflow.

### 236. Нельзя смешивать developer privacy mode and enterprise audit mode

They optimize for opposite outcomes.

Правильно:

- explicit profiles;
- policy matrix;
- UI language differs;
- tests for each profile.

### 237. Нельзя удалять data under legal hold through retention job

Retention job must check holds before purge.

Правильно:

- legal hold table;
- purge preflight;
- blocked purge record;
- tests with overlapping scopes.

### 238. Нельзя считать browser cache durable

Browser storage can be evicted; OPFS/IndexedDB are not canonical DB.

Правильно:

- browser cache is projection;
- server/local DB is truth;
- cache rebuild;
- quota warnings.

### 239. Нельзя ждать disk full before cleanup

At disk full, writer/export/SQLite can fail at worst possible moment.

Правильно:

- quota samples;
- soft warning;
- proactive cleanup;
- strict mode preflight.

### 240. Нельзя удалять pinned/bookmarked history under pressure

Pinned data is explicit user intent.

Правильно:

- pin exceptions;
- cleanup preview;
- ask user;
- preserve active/legal/audit data.

### 241. Нельзя rely on `beforeunload` для persistence

Browser may not fire it on mobile kill, tab discard or process crash.

Правильно:

- server-side durable journal;
- periodic ack/replay state;
- reconnect restore;
- unload only as best-effort hint.

### 242. Нельзя считать hidden tab active renderer

Hidden/frozen tabs can pause timers and rendering.

Правильно:

- visibility state;
- background throttling policy;
- replay on visible;
- no critical UI-only buffers.

### 243. Нельзя разрешать двум tabs быть input owner silently

Duplicate tabs can send conflicting input to one pane.

Правильно:

- server-side input owner;
- Web Locks/BroadcastChannel as helper;
- explicit takeover UX;
- read-only viewer mode.

### 244. Нельзя считать Web Locks DB lock

Browser locks coordinate tabs, not runtime/SQLite correctness.

Правильно:

- DB writer ownership server-side;
- browser lock only for UI leadership;
- expired lock recovery;
- diagnostics.

### 245. Нельзя replay OSC52 clipboard historically

Restored output must not write to local clipboard.

Правильно:

- side-effect suppression in replay;
- OSC52 policy;
- consent for live clipboard writes;
- event audit.

### 246. Нельзя считать paste same as typing

Paste can include multi-line commands/secrets and different user intent.

Правильно:

- paste source metadata;
- bracketed paste awareness;
- private/redaction handling;
- confirmation for stale queued paste.

### 247. Нельзя смешивать docker exec and docker attach

Exec starts a new command; attach connects to existing main process streams.

Правильно:

- container context mode;
- attach attribution lower confidence;
- logs as separate source;
- rerun only for exec-like commands.

### 248. Нельзя считать kubectl logs command output

Logs are application output, not necessarily terminal command lifecycle.

Правильно:

- import/read model;
- cluster/pod/container metadata;
- no command block unless exec session exists;
- redaction/export policy.

### 249. Нельзя игнорировать Windows codepage

Wrong decode can corrupt search, redaction and restore text.

Правильно:

- raw bytes canonical;
- encoding profile;
- decode error metrics;
- rebuild text projections.

### 250. Нельзя считать PowerShell encoding uniform

PowerShell version/cmdlet/native program behavior differs.

Правильно:

- PowerShell version;
- shell encoding policy;
- tests for 5.1/7+;
- export encoding choice.

### 251. Нельзя считать sleep/resume network disconnect only

Suspend can interrupt DB writes, SSH sessions, exports and artifact writes.

Правильно:

- power events;
- writer flush/checkpoint attempt;
- resume integrity check;
- reconnect/gap markers.

### 252. Нельзя prevent sleep silently

Blocking sleep affects user/system power behavior.

Правильно:

- only strict/export/backup critical sections;
- visible status;
- timeout;
- failure fallback.

### 253. Нельзя auto-open terminal hyperlinks in replay

Links from historical output are side effects and can be malicious.

Правильно:

- inert links in replay;
- user action required;
- domain/trust labels;
- URL redaction.

### 254. Нельзя считать container metadata non-sensitive

Cluster, namespace, image and pod names can expose infrastructure.

Правильно:

- classify container context;
- export/AI redaction;
- local-only full metadata by default;
- share preview.

### 255. Нельзя build frontend cache that outlives policy

Browser snapshots/search cache can retain deleted/redacted content.

Правильно:

- cache version tied to policy;
- purge on redaction/delete;
- no private sessions in browser durable cache;
- quota/eviction tolerant.

### 256. Нельзя считать localhost безопасным по умолчанию

Browser pages can reach local services through several attack paths.

Правильно:

- loopback bind;
- origin/host validation;
- high-entropy token;
- message authorization.

### 257. Нельзя полагаться на WebSocket token without Origin check

Cross-site WebSocket hijacking and token leakage paths still matter.

Правильно:

- validate Origin;
- short-lived scoped token;
- reject cookies-only auth;
- audit bad handshakes.

### 258. Нельзя ставить wildcard CORS на control API

Terminal control API is too privileged for broad origins.

Правильно:

- exact allowed origins;
- no wildcard with credentials;
- Fetch Metadata checks where useful;
- PNA-aware preflight handling.

### 259. Нельзя игнорировать Host header on local gateway

DNS rebinding can make hostile hostnames point at loopback.

Правильно:

- expected Host allowlist;
- reject unknown Host;
- token still required;
- bind to loopback only.

### 260. Нельзя хранить gateway token in localStorage forever

Persistent browser storage can outlive session and policy.

Правильно:

- per-launch/session token;
- expiry/revocation;
- memory/session storage preference;
- clear on close/logout/redaction policy.

### 261. Нельзя считать named pipe automatically authenticated

Named pipes need security descriptors and client policy.

Правильно:

- ACL per user;
- client identity check;
- scoped pipe name;
- audit connections.

### 262. Нельзя extract debug bundle directly into live store

Archive entries can traverse paths, overwrite files or create hostile artifacts.

Правильно:

- quarantine directory;
- path normalization;
- size/entry caps;
- schema/redaction scan before import.

### 263. Нельзя trust archive filenames

Filenames can contain absolute paths, `..`, weird Unicode or reserved device names.

Правильно:

- canonicalize;
- reject absolute/parent traversal;
- normalize Unicode carefully;
- generate internal storage names.

### 264. Нельзя открывать HTML transcript as trusted app page

Transcript output can be hostile active content if rendered unsafely.

Правильно:

- escaped text;
- CSP;
- sandboxed iframe;
- no script/service worker by default.

### 265. Нельзя auto-open links/files from exported history

Historical output is untrusted and may contain phishing/local-file paths.

Правильно:

- inert links;
- user confirmation;
- URL/file redaction;
- domain/trust labels.

### 266. Нельзя смешивать desktop webview IPC and terminal gateway auth

Native shell IPC and browser WebSocket have different threat models.

Правильно:

- separate endpoint registry;
- separate tokens/scopes;
- shared authorization layer;
- audit denied IPC calls.

### 267. Нельзя включать Node/native APIs in untrusted terminal viewer

Terminal output should not have access to desktop/native APIs.

Правильно:

- context isolation;
- navigation allowlist;
- command allowlist;
- no untrusted content with native privileges.

### 268. Нельзя позволять transcript viewer register service worker

Service worker/cache can retain or mutate history views beyond policy.

Правильно:

- service worker disabled for viewer;
- Clear-Site-Data on policy reset;
- cache-busting export views;
- no private sessions in durable browser cache.

### 269. Нельзя считать import schema validation enough

Valid schema can still contain hostile payloads or oversized artifacts.

Правильно:

- schema validation;
- path validation;
- size limits;
- redaction scan;
- inert import state.

### 270. Нельзя скрывать gateway security events

Bad origin/host/token attempts are important diagnostics.

Правильно:

- security event table;
- UI/support diagnostics;
- rate-limit metrics;
- tests for rejected attempts.

### 271. Нельзя считать atomic rename durable write

Rename/replace can be atomic for name visibility, but not necessarily durable after power loss.

Правильно:

- temp file in same directory;
- flush/fsync file before replace;
- flush/fsync directory where supported;
- record write transaction state.

### 272. Нельзя писать artifact напрямую в final path

Crash in the middle can leave a corrupt file that looks real.

Правильно:

- write temp path;
- verify length/checksum;
- replace final path only after successful write;
- quarantine partial temp files.

### 273. Нельзя делать cross-volume atomic replace

Moving between volumes can degrade into copy/delete behavior.

Правильно:

- temp file in target directory;
- reject cross-device replace;
- store volume/root identity;
- test Windows and Unix separately.

### 274. Нельзя считать file lock portable truth

Locks differ across OS, filesystem and process lifetime.

Правильно:

- DB state is truth;
- lock is runtime guard;
- heartbeat/process identity;
- stale lock recovery path.

### 275. Нельзя оставлять stale lock unhandled

После crash export/backup/repair can stay blocked forever.

Правильно:

- lock records with owner;
- expiry/heartbeat;
- safe force-recover;
- UI diagnostic.

### 276. Нельзя запускать unreviewed regex over huge transcript

A bad regex can freeze redaction/export/search.

Правильно:

- linear regex engine;
- bounded chunk size;
- rule tests;
- runtime timeout/metrics.

### 277. Нельзя использовать backtracking regex for untrusted output

Terminal output is attacker-controlled input.

Правильно:

- reject unsafe pattern classes;
- exact matching first;
- Rust regex/RE2-style engine;
- fuzz crafted payloads.

### 278. Нельзя делать redaction without per-rule metrics

Без метрик нельзя понять, какая rule slow/noisy/broken.

Правильно:

- match count;
- latency histogram;
- bytes scanned;
- false-positive review state.

### 279. Нельзя сканировать secrets только одним engine

Regex-only misses context and provider validation.

Правильно:

- exact patterns;
- linear regex;
- entropy checks;
- provider validators where possible.

### 280. Нельзя делать authorization client-side only

UI permission checks are hints, not enforcement.

Правильно:

- server/runtime policy boundary;
- deny-by-default;
- action/resource checks;
- audit all decisions.

### 281. Нельзя хранить share permissions as boolean

`is_shared=true` cannot express raw/export/rerun/expiry/redaction.

Правильно:

- explicit resources;
- explicit actions;
- caveats;
- revocation state.

### 282. Нельзя давать long-lived share token без caveats

Long-lived broad tokens leak too much.

Правильно:

- expiry;
- audience/resource scope;
- redaction profile;
- token hash and revoke.

### 283. Нельзя audit policy decision без inputs

Decision without policy version and attributes cannot be explained later.

Правильно:

- subject/resource/action/environment;
- policy version;
- reason code;
- evaluated timestamp.

### 284. Нельзя смешивать viewer permission and export permission

View in UI is not the same as durable export/download/share.

Правильно:

- separate actions;
- separate redaction profile;
- export manifest;
- policy decision per action.

### 285. Нельзя делать deny reasons invisible

Silent denial looks like a bug and slows support.

Правильно:

- safe reason codes;
- user-friendly message;
- support/audit details;
- no sensitive policy leakage.

### 286. Нельзя считать path string identity

Path can be renamed, redirected or point to a different object later.

Правильно:

- generated storage key;
- canonical/final path metadata;
- file ID/volume where available;
- verifier scan.

### 287. Нельзя использовать command text as export filename

Commands can contain secrets, invalid characters, path traversal or reserved names.

Правильно:

- generated filenames;
- metadata in manifest;
- sanitized display labels;
- redaction before export.

### 288. Нельзя забывать Windows reserved device names

Names like `CON`, `NUL`, `PRN` and related variants are special.

Правильно:

- reject reserved basenames;
- generate artifact names;
- test reserved names with extensions;
- keep original only as escaped metadata.

### 289. Нельзя игнорировать alternate data streams

Colon syntax can address NTFS alternate streams.

Правильно:

- reject ADS-like names in import/export storage paths;
- model streams explicitly if ever supported;
- avoid colon in generated filenames;
- test `name:stream` cases.

### 290. Нельзя считать MAX_PATH solved globally

Long path behavior depends on OS/app manifest/config and path namespace.

Правильно:

- long-path test fixtures;
- generated shallow storage layout;
- explicit Windows path APIs;
- graceful error messages.

### 291. Нельзя follow reparse points in live artifact store

Junction/symlink/mount point can redirect outside root.

Правильно:

- reject/quarantine reparse points by default;
- verify after open for critical writes;
- store reparse state;
- full rescan on suspicious changes.

### 292. Нельзя делать check-then-open security

Path can change between validation and file open.

Правильно:

- open with strict flags;
- inspect final handle path/identity;
- commit manifest after verification;
- retry/quarantine on mismatch.

### 293. Нельзя игнорировать CreateFile sharing modes

Windows readers can block replace/delete depending on sharing flags.

Правильно:

- define reader/writer sharing policy;
- test antivirus/indexer-like readers;
- handle sharing violation retries;
- surface blocked artifact writes.

### 294. Нельзя считать case sensitivity uniform on Windows

WSL/per-directory settings can make case behavior differ.

Правильно:

- generated case-stable storage names;
- case sensitivity metadata;
- collision tests;
- display names separate from keys.

### 295. Нельзя сравнивать display paths for security

Pretty paths can be lossy, normalized or localized.

Правильно:

- compare normalized internal refs;
- keep display path as UI-only;
- escape lossy conversions;
- log path identity fields.

### 296. Нельзя использовать UTF-8 `String` as universal path type

Windows paths are OS strings, not guaranteed safe UTF-8 business data.

Правильно:

- use `PathBuf`/`OsStr`;
- serialize explicitly;
- separate display conversion;
- test non-UTF-like paths where possible.

### 297. Нельзя считать watcher events complete

Filesystem watchers can drop, coalesce or miss events.

Правильно:

- watcher marks dirty;
- verifier scans;
- overflow detection;
- full rescan fallback.

### 298. Нельзя считать USN journal permanent truth

USN can reset, wrap or be unavailable depending on volume.

Правильно:

- store cursor/range;
- detect reset/gap;
- full rescan on gap;
- DB manifest remains truth.

### 299. Нельзя хранить artifact store in arbitrary user folder без checks

User-selected folders can be network drives, synced folders, protected locations or contain reparse points.

Правильно:

- capability probe;
- write/replace/flush test;
- filesystem profile;
- warn/degrade unsupported stores.

### 300. Нельзя смешивать storage names and user-visible names

Safe storage keys and nice UI names have different requirements.

Правильно:

- generated storage keys;
- escaped display names;
- manifest metadata;
- redaction/export policy.

### 301. Нельзя обещать exactly-once transport

Networks, browsers, workers and sync all retry/drop/duplicate.

Правильно:

- idempotency keys;
- event IDs;
- sequence numbers;
- dedup records;
- replay from durable journal.

### 302. Нельзя trust client sequence as source of truth

Client can reconnect, duplicate, lag or be hostile.

Правильно:

- runtime writer assigns authoritative seq;
- client seq only as correlation;
- reject impossible gaps;
- audit mismatches.

### 303. Нельзя делать side-effect job outside DB transaction

Search/export/sync can miss committed history if job enqueue happens after commit and process crashes.

Правильно:

- transactional outbox;
- worker retry;
- poison quarantine;
- rebuild from journal.

### 304. Нельзя делать retries non-idempotent

Retry after timeout can duplicate export/share/delete/input action.

Правильно:

- scoped idempotency key;
- request hash;
- stored result;
- mismatch detection.

### 305. Нельзя хранить idempotency keys forever без policy

Unbounded key table grows and may retain sensitive request metadata.

Правильно:

- TTL per operation kind;
- hash keys;
- compact completed rows;
- keep audit summary separately.

### 306. Нельзя делать browser offline input queue unbounded

Reconnect can flush stale/destructive commands.

Правильно:

- queue caps;
- stale confirmation;
- per-pane ownership;
- drop policy with user-visible status.

### 307. Нельзя merge divergent terminal streams as text

PTY output order is semantic and side-effectful.

Правильно:

- branch/conflict record;
- keep both histories;
- manual resolution;
- never silently interleave bytes.

### 308. Нельзя использовать CRDT для raw transcript ordering

CRDT text merge creates a false terminal timeline.

Правильно:

- CRDT only for metadata/notes;
- append-only log for transcript;
- single writer authority;
- conflict branches for divergence.

### 309. Нельзя считать object storage filesystem

Object stores have versions, ETags, lifecycle and different rename semantics.

Правильно:

- immutable objects;
- manifest last;
- version/ETag recorded;
- provider-specific tests.

### 310. Нельзя overwrite history artifacts in-place during sync

In-place overwrite destroys rollback and conflict evidence.

Правильно:

- content-addressed artifacts;
- immutable chunks;
- new manifest version;
- garbage collection by reachability.

### 311. Нельзя prune chunks without reachability graph

Backups/snapshots/exports may still reference old chunks.

Правильно:

- manifest graph;
- mark-and-sweep;
- retention/legal hold check;
- dry-run report.

### 312. Нельзя считать backup successful без restore drill

Copied files can still be unusable.

Правильно:

- temp restore;
- schema/artifact verification;
- projection rebuild;
- sample replay.

### 313. Нельзя хранить only latest snapshot

Latest snapshot can be corrupt or incompatible with new parser.

Правильно:

- snapshot lineage;
- multiple checkpoints;
- high-water seq;
- fallback replay.

### 314. Нельзя делать snapshot без manifest

Opaque snapshot cannot explain missing chunks or projection version.

Правильно:

- artifact refs;
- parser/projection/redaction versions;
- parent snapshot;
- checksums.

### 315. Нельзя считать ETag checksum universally

Object storage ETag semantics differ by provider/upload mode.

Правильно:

- store our checksum;
- store provider version/etag separately;
- verify downloaded bytes;
- document provider behavior.

### 316. Нельзя delete local data before sync tombstone is durable

Deletion can be undone or data can reappear from another device.

Правильно:

- tombstone row;
- sync outbox;
- conflict policy;
- final purge after retention window.

### 317. Нельзя ignore object versions in privacy deletion

Old object versions can still contain deleted transcript.

Правильно:

- version-aware deletion;
- lifecycle policy;
- legal hold state;
- user-visible caveat.

### 318. Нельзя hide outbox lag

History may be persisted but search/export/sync can be behind.

Правильно:

- outbox dashboard;
- lag metrics;
- per-feature freshness;
- retry/quarantine UI.

### 319. Нельзя hide sync conflicts

Silent conflict handling makes users distrust history.

Правильно:

- conflict timeline;
- keep both branches;
- reason codes;
- resolution audit.

### 320. Нельзя order distributed history only by wall clock

Clocks drift and imported sessions may have bad timestamps.

Правильно:

- per-stream seq;
- monotonic deltas where local;
- wall time as display;
- clock quality metadata.

### 321. Нельзя делать command text high-cardinality label

Index labels with unique values explode storage and query cost.

Правильно:

- command text in searchable content;
- low-cardinality labels only;
- query planner over chunk catalog;
- cardinality metrics.

### 322. Нельзя делать cwd глобальным индексным label

Full paths are sensitive and high cardinality.

Правильно:

- cwd as redacted attribute;
- path tokens in controlled search index;
- policy before snippet display;
- never label by full cwd.

### 323. Нельзя хранить search index как source of truth

Indexes can be stale, corrupt, redacted differently or rebuilt.

Правильно:

- canonical journal/chunks;
- derived index metadata;
- rebuild jobs;
- freshness state.

### 324. Нельзя возвращать search snippet без policy check

Index may contain text user can no longer view/export.

Правильно:

- authorize query scope;
- verify match against redacted projection;
- attach policy decision;
- hide raw snippet if denied.

### 325. Нельзя считать Bloom filter доказательством совпадения

Bloom filters have false positives.

Правильно:

- use only as prefilter;
- verify actual content;
- track false-positive rate;
- rebuild bad filters.

### 326. Нельзя cold search делать silently expensive

Cold/object-store search can be slow and costly.

Правильно:

- show tier;
- query budget;
- partial results;
- continue/cancel controls.

### 327. Нельзя делать unbounded regex search over compressed chunks

Broad regex over years of output can decompress too much.

Правильно:

- chunk budget;
- regex safety;
- prefilter where possible;
- async job with progress.

### 328. Нельзя забывать tokenizer version

Search results change when tokenization changes.

Правильно:

- tokenizer version in index;
- rebuild on change;
- test query fixtures;
- stale warning.

### 329. Нельзя менять redaction profile без index invalidation

Old index/snippets can expose previously visible secrets.

Правильно:

- redaction profile in index metadata;
- invalidate/rebuild affected indexes;
- clear cached snippets;
- audit profile changes.

### 330. Нельзя put raw secrets into analytics/columnar exports by accident

Analytics copies are another data surface.

Правильно:

- redacted analytics projection;
- schema allowlist;
- export policy;
- retention alignment.

### 331. Нельзя rely on primary DB for all historical search

One huge local DB can make restore/search/cleanup fight each other.

Правильно:

- hot DB scope;
- warm/cold chunk artifacts;
- derived search shards;
- maintenance windows.

### 332. Нельзя делать lifecycle tier move invisible

User sees slow/missing search without understanding why.

Правильно:

- lifecycle events;
- tier badge;
- search scope UI;
- restore/search fallback.

### 333. Нельзя делать merge/optimize без backpressure

Search compaction can compete with live terminal capture.

Правильно:

- worker priority;
- IO budget;
- pause under active capture;
- lag metrics.

### 334. Нельзя считать partial search failure empty result

No matches and not searched are different.

Правильно:

- result_state;
- scanned chunk count;
- skipped tier reason;
- resume cursor.

### 335. Нельзя индексировать hostile terminal control sequences as plain text

ANSI/OSC/control bytes can poison snippets and search display.

Правильно:

- parser-derived text projection;
- escaped snippets;
- control sequence classification;
- raw view only in debug mode.

### 336. Нельзя считать terminal output инструкцией для AI

Output is untrusted data from programs, logs and remote systems.

Правильно:

- data-only context items;
- provenance labels;
- prompt-injection findings;
- action gates.

### 337. Нельзя paste raw transcript as one prompt

Raw paste destroys boundaries between user intent, command and output.

Правильно:

- structured context package;
- item kinds;
- source ranges;
- redaction report.

### 338. Нельзя полагаться только на system prompt против injection

Prompt text is not a security boundary.

Правильно:

- deterministic policy;
- scoped tools;
- approval gates;
- red-team fixtures.

### 339. Нельзя позволять AI auto-rerun command from output suggestion

Output can suggest destructive commands.

Правильно:

- show command diff;
- require user approval;
- check source trust;
- block if derived from risky range.

### 340. Нельзя давать AI raw history без redaction decision

Terminal history often contains tokens, paths and secrets.

Правильно:

- redaction profile;
- sensitive findings;
- raw access policy;
- audit context package.

### 341. Нельзя смешивать user command and command output in one role

The model may treat output as instruction.

Правильно:

- separate fields;
- data-only markers;
- no instruction eligibility for output;
- source citations.

### 342. Нельзя считать prompt-injection detector proof of safety

Detectors miss attacks and have false positives.

Правильно:

- detector as risk signal;
- policy still enforces;
- approvals remain required;
- red-team regression tests.

### 343. Нельзя скрывать source ranges from AI answer

Without provenance, user cannot audit why AI suggested something.

Правильно:

- cite command blocks;
- cite seq ranges;
- link snippets;
- mark summaries as derived.

### 344. Нельзя давать model broad terminal tool access

Broad tool access turns context injection into action.

Правильно:

- least privilege;
- per-action scope;
- time-bound permissions;
- user approval for risky tools.

### 345. Нельзя хранить AI summaries as canonical history

Summaries can omit, hallucinate or redact differently.

Правильно:

- summary is derived artifact;
- canonical journal remains truth;
- source ranges required;
- rebuild on parser/redaction change.

### 346. Нельзя ignore hidden Unicode in AI context

Bidi/confusables can change visible command meaning.

Правильно:

- Unicode risk classification;
- escaped display;
- risky range finding;
- approval preview uses safe rendering.

### 347. Нельзя let ANSI hyperlinks become AI-trusted links

Terminal links can point to hostile/local resources.

Правильно:

- inert link metadata;
- user confirmation;
- URL redaction;
- no auto-open from AI.

### 348. Нельзя отправлять cold-history search results to AI without query budget

Huge context increases cost, leakage and latency.

Правильно:

- token/byte/chunk budget;
- selected ranges;
- partial state;
- user preview.

### 349. Нельзя давать MCP resource expose raw DB by default

MCP/resource consumers can become another exfiltration path.

Правильно:

- redacted resources by default;
- explicit scopes;
- schema versions;
- audit reads.

### 350. Нельзя ignore tool capability drift

Tool updates can add new actions or broader permissions.

Правильно:

- tool version pinning;
- permission diff;
- policy re-evaluation;
- red-team run after changes.

### 351. Нельзя approve AI action without showing exact payload

User cannot consent to hidden command/file/export details.

Правильно:

- full payload preview;
- dangerous diff highlight;
- source context;
- expiration.

### 352. Нельзя let AI delete/export/share based on untrusted output

Output can instruct the model to exfiltrate or destroy data.

Правильно:

- strong user intent requirement;
- separate policy decisions;
- deny if source is data-only injection range;
- audit.

### 353. Нельзя count "model refused" as product security

Refusal behavior can change by model/version/context.

Правильно:

- runtime policy blocks;
- tool layer denies;
- tests assert no side effect;
- model response is secondary.

### 354. Нельзя send approval text generated by terminal output as real approval

Logs can contain fake "User approved" text.

Правильно:

- approvals only from UI event;
- signed/recorded decision;
- source output cannot set approval state;
- audit decision path.

### 355. Нельзя release AI terminal feature without adversarial fixtures

Normal happy-path tests miss prompt-injection chains.

Правильно:

- fixture set version;
- automated red-team run;
- unexpected action count;
- release gate.

### 356. Нельзя заявлять reliability без executable invariants

Prose guarantees do not catch regressions.

Правильно:

- invariant registry;
- automated checkers;
- release gates;
- production diagnostics.

### 357. Нельзя считать happy-path tests enough for persistence

Storage bugs live in crash, retry, race and partial failure paths.

Правильно:

- fault injection;
- crash tests;
- restore drills;
- concurrency schedules.

### 358. Нельзя оставлять simulation failure без seed

Rare failures become unfixable without reproduction data.

Правильно:

- store seed;
- store fault plan;
- store schedule;
- store minimized fixture.

### 359. Нельзя inject faults only at high-level APIs

Real failures happen between write, flush, rename, commit and ack.

Правильно:

- named failpoints;
- boundary-level faults;
- short write/flush fail;
- crash after side effect.

### 360. Нельзя model-check весь терминал сразу

Over-large specs become unusable and unmaintained.

Правильно:

- small protocols;
- seq/ack, outbox, tombstones;
- bounded state;
- save counterexamples.

### 361. Нельзя игнорировать async schedule races

Stress tests rarely reproduce exact interleaving.

Правильно:

- Loom/Shuttle-style tests;
- deterministic scheduler seeds;
- shared state minimized;
- race fixtures.

### 362. Нельзя считать outbox retry safe without crash-after-side-effect test

Worker can crash after creating export/share but before marking done.

Правильно:

- idempotent side effects;
- result refs;
- retry detects existing result;
- fault at every worker boundary.

### 363. Нельзя считать WAL/DB commit enough for external artifact safety

DB can commit a path to artifact that failed to flush/replace.

Правильно:

- artifact transaction ledger;
- fault injection around file operations;
- verifier before restore;
- quarantine mismatch.

### 364. Нельзя делать chaos testing без steady-state hypothesis

Random failure without expected behavior creates noise.

Правильно:

- expected invariant;
- expected user-visible state;
- recovery budget;
- success criteria.

### 365. Нельзя скрывать flaky persistence tests

Flakiness in persistence often means real race or timing bug.

Правильно:

- quarantine with owner;
- seed capture;
- repeat/minimize;
- release gate impact.

### 366. Нельзя release migration without old-DB fixtures

Long-lived history must survive schema changes.

Правильно:

- fixture DBs by version;
- migration simulation;
- projection rebuild;
- rollback/forward-only notes.

### 367. Нельзя считать restore drill optional

Backup/export cannot be trusted until restored.

Правильно:

- temp restore;
- checksum verification;
- sample replay;
- report stored.

### 368. Нельзя тестировать Windows only through Linux assumptions

Windows path/share/ConPTY behavior has separate failure modes.

Правильно:

- Windows-native fault fixtures;
- sharing violation tests;
- reparse/ADS/long path tests;
- ConPTY lifecycle tests.

### 369. Нельзя ignore clock/suspend faults

Sleep/resume and clock jumps affect timers, TTL, replay and idempotency expiry.

Правильно:

- fake clock;
- suspend/resume simulation;
- monotonic duration invariants;
- wall-clock quality metadata.

### 370. Нельзя use production user history as raw failure artifact

Failure repros can leak secrets and transcripts.

Правильно:

- sanitized/minimized fixtures;
- access-controlled artifacts;
- redaction before sharing;
- retention policy.

### 371. Нельзя считать generated test data enough

Terminal reality includes weird ANSI, Unicode, shells and Windows paths.

Правильно:

- corpus from real sanitized cases;
- fuzz cases;
- golden fixtures;
- regression from bugs.

### 372. Нельзя skip fault tests for "small refactors"

Small persistence changes can alter ordering and durability.

Правильно:

- changed-path gate;
- affected invariant selection;
- minimal simulation suite;
- release note if skipped.

### 373. Нельзя ignore model-check counterexample

Counterexample is a design bug until disproven.

Правильно:

- translate to fixture;
- fix spec or implementation;
- document assumption;
- rerun gate.

### 374. Нельзя rely on manual QA for reconnect correctness

Reconnect has too many state combinations.

Правильно:

- seeded transport simulator;
- ack/replay invariants;
- duplicate/stale action tests;
- screenshot/browser check only as final layer.

### 375. Нельзя claim "no data loss" without defining loss

Input, output, command block, snapshot, search index and AI context have different guarantees.

Правильно:

- guarantee matrix;
- explicit degraded states;
- tested scopes;
- user-visible reliability badge.

### 376. Нельзя считать encryption replacement for redaction

Encrypted history still becomes plaintext in UI, export, AI context and logs.

Правильно:

- encryption + redaction;
- private mode;
- export policy;
- AI context preview.

### 377. Нельзя хранить DB key рядом с DB

Key next to encrypted data turns encryption into obfuscation.

Правильно:

- OS key store;
- passphrase KDF;
- wrapped keys;
- access audit.

### 378. Нельзя использовать one global key forever

Global key blocks selective erase and increases blast radius.

Правильно:

- key hierarchy;
- scoped DEKs;
- rotation;
- destruction records.

### 379. Нельзя rotate by rewriting all chunks when DEK wrapping is enough

Full rewrite is slow and failure-prone.

Правильно:

- DEK per artifact;
- KEK rewrap;
- rotation job state;
- verify decrypt after rewrap.

### 380. Нельзя забывать associated data

Ciphertext can be replayed in wrong context if metadata is not authenticated.

Правильно:

- AAD with session/pane/artifact/seq/schema;
- manifest hash;
- verify before replay;
- reject mismatch.

### 381. Нельзя reuse nonce with same key

AEAD nonce reuse can break confidentiality/integrity.

Правильно:

- per-key nonce registry/derivation;
- tested crypto wrapper;
- no manual ad-hoc nonce logic;
- fixtures for duplicate nonce rejection.

### 382. Нельзя decrypt before authenticating

Unauthenticated plaintext can lead to parser or output attacks.

Правильно:

- AEAD verification first;
- streaming auth boundaries;
- fail closed;
- quarantine corrupt chunks.

### 383. Нельзя хранить key version only in config

Old chunks need their original key/version metadata.

Правильно:

- key version per artifact;
- wrapped key ref;
- migration records;
- compatibility tests.

### 384. Нельзя делать rekey без crash recovery

Crash during rekey can lose data or key references.

Правильно:

- rotation job table;
- prepared/committed states;
- verification stage;
- rollback/quarantine.

### 385. Нельзя шифровать только canonical journal

Search indexes, snapshots, AI caches and exports can duplicate secrets.

Правильно:

- encryption scope inventory;
- encrypted derived artifacts;
- redaction-aware indexes;
- cache invalidation.

### 386. Нельзя export backup unencrypted by default

Backup/export is a common leakage path.

Правильно:

- encrypted export bundles;
- recipient/passphrase choice;
- manifest policy;
- verify decrypt drill.

### 387. Нельзя делать key recovery unclear

Users must know whether lost key means lost history.

Правильно:

- recovery profile;
- warnings;
- recovery test;
- documented tradeoffs.

### 388. Нельзя use DPAPI machine scope by accident

Machine scope can allow broader access than intended.

Правильно:

- explicit user vs machine scope;
- profile probe;
- audit setting;
- least-privilege default.

### 389. Нельзя assume OS keychain exists in headless/CI

Linux servers and CI may not have Secret Service or user session unlock.

Правильно:

- capability profile;
- passphrase/test backend;
- explicit degraded mode;
- no silent plaintext fallback.

### 390. Нельзя call cryptographic erase on unencrypted data

Destroying a key does nothing if data was plaintext.

Правильно:

- encryption state check;
- key scope check;
- tombstone;
- limitation report.

### 391. Нельзя считать secure_delete full physical erasure

Filesystems, SSDs, WAL, temp files and backups can retain copies.

Правильно:

- honest limitation text;
- cryptographic erase;
- VACUUM/checkpoint where useful;
- backup/object version policy.

### 392. Нельзя оставлять plaintext temp files

Temp export/search/rebuild files can leak secrets.

Правильно:

- encrypted temp artifacts;
- controlled temp directory;
- cleanup registry;
- crash cleanup.

### 393. Нельзя логировать keys, nonces, passphrases or raw decrypted paths

Logs can outlive encryption controls.

Правильно:

- secrecy wrappers;
- redacted logs;
- lint/tests;
- crash report scrubber.

### 394. Нельзя overpromise memory zeroization

Zeroization helps but cannot fully control copies, paging or compiler/runtime behavior.

Правильно:

- use zeroize/secrecy where practical;
- reduce plaintext lifetime;
- document limits;
- avoid unnecessary copies.

### 395. Нельзя claim encryption protects against same-user malware

If malware runs as the user while app is unlocked, it can often read plaintext.

Правильно:

- threat model;
- OS isolation;
- approval gates;
- no false security messaging.

### 396. Нельзя use weak passphrase KDF settings

User passphrases need expensive, versioned derivation.

Правильно:

- modern KDF;
- stored parameters;
- migration path;
- UX for weak passphrase warning.

### 397. Нельзя let recovery key become invisible secret debt

Recovery keys are sensitive and need lifecycle.

Правильно:

- creation/rotation/revocation;
- backup verification;
- access audit;
- enterprise policy if escrowed.

### 398. Нельзя decrypt raw chunks for search when redacted index is enough

Raw decrypt expands exposure.

Правильно:

- redacted search projection;
- raw access policy;
- audited key access;
- query budget.

### 399. Нельзя ignore crypto agility

Algorithms and parameters age.

Правильно:

- algorithm/version fields;
- migration jobs;
- test vectors;
- deprecation policy.

### 400. Нельзя ship crypto without test vectors

Crypto wrappers fail silently without known-answer tests.

Правильно:

- official test vectors;
- roundtrip fixtures;
- tamper tests;
- duplicate nonce tests.

## Интересные идеи для продукта

### History Reliability Badge

В каждом pane показывать компактный state:

- `Rich history` - trusted shell integration + journal writer OK.
- `Basic history` - output journal OK, command boundaries partial.
- `Visual restore only` - snapshots есть, command blocks неточные.
- `History degraded` - DB writer failed / quota / private mode.

Это снимает ложные обещания и помогает дебажить.

### Command Provenance Inspector

Для любого block показывать:

- command source: UI submit / OSC 633 E / OSC 133 / heuristic;
- cwd source;
- exit code source;
- output range source;
- trust level;
- sequence range.

Так мы не спрячем сложность, но сделаем ее управляемой.

### Command Timeline

Показывать commands как timeline:

- status icon;
- duration;
- cwd;
- exit code;
- output size;
- copied/shared/bookmarked state;
- "rerun in same cwd";
- "open output in search/pager".

### Block Attachments for AI

Как Warp Blocks as Context:

- attach previous command output to AI;
- attach only selected output range;
- redact secrets before attach;
- show exact provenance: session/pane/command/time.

### Durable Search

Поиск должен работать не только по visible buffer:

- command text;
- output text;
- cwd;
- exit code;
- timestamp;
- backend;
- session;
- pane;
- status failed only.

### Background Output Blocks

Если output приходит вне foreground command:

- создать `background_block`;
- показывать как "unattributed output";
- не привязывать к последней команде без confidence.

### Restore Banner

После restore:

- "History restored from 2026-04-29 14:32";
- "Live process was restarted" или "Attached to live zellij session";
- "Shell integration quality: rich/trusted/basic/none";
- "Some command boundaries are heuristic".

### Safety Rerun

Для restored commands:

- normal command - rerun button;
- destructive-looking command - confirmation;
- production-looking cwd/env - stronger confirmation;
- commands from untrusted OSC - require edit before run.

### Private Mode

Session-level toggle:

- do not persist output;
- do not persist commands;
- do not attach blocks to AI;
- clear in-memory scrollback on close.

### History Export

Export as:

- markdown command blocks;
- asciicast-like event stream;
- plain text transcript;
- JSON for debug.

### Forensic Replay Mode

Отдельный debug/research режим:

- воспроизвести stream с timing;
- показать raw bytes/escape sequences;
- показать resize points;
- показать command block boundaries;
- сравнить rendered snapshot до/после parser changes.

Это особенно полезно для Windows/ConPTY и zellij bugs.

### Smart Retention

Не один глобальный лимит, а policy:

- keep failed command outputs longer;
- keep bookmarked blocks forever until explicit delete;
- keep recent sessions full fidelity;
- compress cold segments;
- prune unattributed huge logs first;
- never prune while command is running.

### Session Resurrection Checklist

Перед restore показывать технически честный summary:

- layout restored;
- history restored;
- output replayed;
- live process attached/restarted;
- commands waiting for confirmation;
- shell integration quality.

### History Flight Recorder

Developer-facing diagnostics view for one pane/session:

- last committed stream seq;
- last projected seq;
- writer queue depth;
- snapshot age;
- current durability profile;
- last DB error;
- redaction/export status.

Цель: когда пользователь говорит "после restart история пропала", support/dev can see whether failure was writer, projection, restore, policy or UI.

### Export Quarantine Viewer

Imported/debug history should open in a safe viewer first:

- schemas validated;
- unknown events listed;
- suspicious ANSI/OSC neutralized;
- secrets scan status visible;
- commands are copyable but not runnable by default.

Цель: import/export is useful without turning transcript files into active terminal content.

### Command Identity Inspector

For a command block show all known identities:

- entered text;
- shell marker text;
- observed process command line;
- cwd/source;
- argv if known;
- trust/confidence;
- rerun mode.

Цель: on Windows/PowerShell/zellij it becomes clear why a command can or cannot be safely rerun.

### History Rebuild Center

Maintenance UI for derived layers:

- rebuild search index;
- rebuild projections from journal;
- verify checksums;
- quarantine corrupt artifacts;
- compact cold segments;
- re-run redaction with newer profile.

Цель: long-lived history DB can survive parser/schema/redaction upgrades without manual surgery.

### Privacy Retention Simulator

Before applying retention policy, show:

- sessions that will be deleted;
- command blocks that will be redacted;
- artifact bytes that will be removed;
- backups/exports that may still contain data;
- estimated disk impact.

Цель: user understands privacy/storage tradeoff before destructive cleanup.

### Accessible History Navigator

Keyboard and screen-reader first timeline:

- jump command by command;
- announce exit code/duration/cwd;
- jump to first failed command;
- jump to redacted/sensitive blocks;
- copy command/output through accessible controls.

Цель: command blocks become useful beyond visual UI.

### Artifact Quarantine Inbox

A safety surface for terminal media/file-transfer output:

- unknown artifact protocols;
- blocked oversize payloads;
- images requiring preview confirmation;
- missing artifact repair;
- export warnings.

Цель: media protocols do not silently become security/privacy holes.

### Background Jobs Monitor

Small operational panel:

- queued/retrying/failed jobs;
- projection rebuild progress;
- redaction queue;
- artifact GC status;
- compaction/checkpoint status;
- last worker heartbeat.

Цель: long-running history maintenance is visible and recoverable.

### Search Profile Switcher

User/dev diagnostics for search:

- basic derived chunks;
- FTS5 if enabled;
- Tantivy cold/global index later;
- tokenizer preview;
- index freshness state.

Цель: search quality and performance can evolve without changing canonical storage.

### AI Context Capsule

Before sending history to AI, show a compact capsule:

- command blocks included;
- byte/token budget;
- redaction state;
- trust levels;
- excluded sensitive findings;
- exact source ranges.

Цель: AI gets useful terminal context without becoming a silent transcript leak.

### Timeline Clock Inspector

Debug view for replay timing:

- wall clock;
- monotonic deltas;
- import/remote clock quality;
- reconnect anchors;
- replay speed.

Цель: explain why replay timing differs from user-visible timestamps.

### Windows Process Lifecycle Inspector

For native Windows panes, show:

- ConPTY handle state;
- process PID and job object state;
- graceful signal attempts;
- force-kill fallback;
- orphan detection;
- last exit reason.

Цель: Windows session reliability becomes observable instead of mysterious.

### Artifact Integrity Dashboard

For external/cold artifacts:

- missing/modified/hash mismatch count;
- last full scan;
- watcher overflow events;
- quarantine list;
- repair/rebuild actions.

Цель: external artifacts are safe to use for scalable history without silent restore failures.

### Restore Guarantee Matrix

Per backend/session show:

- history restored;
- layout restored;
- live process attached;
- process checkpoint restored;
- mux resurrected;
- output replayed;
- AI context safe.

Цель: product language remains honest as native/zellij/tmux/future checkpoint paths diverge.

### Reconnect Gap Inspector

For each pane/client:

- last server seq sent;
- last browser ack;
- replayed seq range;
- unrecoverable gaps;
- duplicate suppression count;
- reconnect reason.

Цель: "после reconnect пропал output" becomes debuggable.

### Execution Domain Badge

Show compact domain labels:

- Windows native;
- WSL distro;
- SSH remote;
- zellij/tmux;
- container;
- imported replay.

Цель: user sees where command actually ran before rerun/export/AI.

### Remote Privacy Review

Before sharing/exporting remote session:

- host/user/path fields detected;
- forwarded ports/agent forwarding state;
- WSL distro/path mappings;
- redaction preview;
- local-only fields.

Цель: remote infrastructure metadata does not leak by accident.

### Transport Chaos Lab

Developer QA panel/scripts:

- disconnect;
- latency spike;
- packet loss;
- server restart;
- browser reload;
- mux detach;
- replay-gap verification.

Цель: reconnect guarantees are tested like core persistence, not manually guessed.

### Schema Evolution Workbench

Developer tool for payload/schema changes:

- load old DB/export fixtures;
- run upcasters;
- diff projections before/after;
- show incompatible fields;
- regenerate query/search snapshots.

Цель: schema evolution is a tested workflow, not a migration hope.

### Query Plan Regression Dashboard

For critical history queries:

- current plan;
- expected plan;
- rows scanned;
- duration on large fixture;
- missing/unused indexes.

Цель: large-history performance stays measurable before users feel it.

### Crash Privacy Gate

Before support upload:

- raw command/output included or not;
- private sessions excluded;
- remote metadata redacted;
- crash dump type;
- approval state.

Цель: diagnostics help support without becoming a transcript leak.

### Legal Hold and Retention Review

Enterprise/audit UI:

- active holds;
- blocked deletions;
- retention expiry;
- immutable records;
- release workflow.

Цель: audit mode is explicit and does not silently override developer privacy expectations.

### Storage Pressure Forecast

Show:

- DB/WAL/artifact/cache growth;
- estimated days until soft quota;
- cleanup candidates;
- pinned/legal hold blockers;
- strict-mode risk.

Цель: user sees storage pressure before writer enters degraded mode.

### Browser Client Ownership Panel

For each session/pane:

- current input owner tab;
- visible/background clients;
- last ack seq per client;
- takeover history;
- stale clients.

Цель: multi-tab web terminal behavior is clear and controllable.

### Side-Effect Consent Center

Central policy for:

- OSC52 clipboard;
- paste into live terminal;
- file transfers;
- link opening;
- media preview;
- historical replay suppression.

Цель: user understands when terminal output can affect local machine.

### Container Context Badge

Show:

- Docker/Kubernetes/Podman;
- exec/attach/logs mode;
- namespace/pod/container;
- TTY/stdin state;
- redaction state.

Цель: commands and logs from containers are not mistaken for local shell history.

### Encoding Diagnostics View

For Windows panes:

- input/output codepage;
- PowerShell version;
- UTF-8 mode;
- decode errors;
- projection rebuild option.

Цель: mojibake/search/redaction bugs become diagnosable.

### Sleep Resume Timeline

Timeline events:

- suspend requested;
- writer flush result;
- reconnect required;
- resume;
- replay/gap result;
- export/backup interrupted.

Цель: laptop sleep stops being an invisible source of history gaps.

### Gateway Security Console

Show:

- active IPC/WebSocket endpoints;
- allowed origins/hosts;
- token scopes and expiry;
- rejected Origin/Host/token attempts;
- rate-limit events.

Цель: local terminal gateway becomes observable and auditable.

### Safe Import Wizard

Step-by-step import:

- quarantine bundle;
- validate paths;
- validate schema;
- scan secrets;
- show unknown files;
- import as inert historical data.

Цель: debug/support bundles cannot mutate live history or execute anything.

### Transcript Viewer Security Mode

Profiles:

- inert text-only;
- safe links with confirmation;
- media preview after scan;
- debug raw view local-only;
- AI preview redacted.

Цель: history viewing/exporting has explicit security posture.

### Desktop Wrapper Threat Matrix

For Electron/Tauri/native shell:

- navigation allowlist;
- native IPC permissions;
- gateway token handling;
- local file access;
- external link handling;
- devtools/debug mode.

Цель: desktop packaging does not weaken the web-terminal security model.

### Artifact Write Inspector

Show per artifact:

- DB manifest state;
- temp/final paths;
- byte counts and checksum;
- file flush and directory flush state;
- last verifier result.

Цель: "history disappeared" can be debugged as a write transaction, not guessed from logs.

### Redaction Rule Profiler

Show for each rule:

- scanned bytes;
- matches;
- average/p95 runtime;
- false-positive markers;
- last fixture pass/fail.

Цель: secret protection stays measurable and does not silently slow exports/search.

### Share Policy Simulator

Before creating a share/export:

- choose subject/role;
- choose session/pane/command range;
- choose raw vs redacted;
- preview allowed actions;
- show deny reasons.

Цель: policy becomes understandable before sensitive history leaves the local context.

### Capability Grant Ledger

Show:

- active grants;
- resource/action scopes;
- caveats;
- last use;
- revoke button;
- audit trail.

Цель: every shared history access has a visible lifecycle and can be revoked.

### Lock/Stale Writer Doctor

Detect:

- stale writer locks;
- interrupted exports;
- orphan temp artifacts;
- blocked compaction jobs;
- conflicting process owners.

Цель: crash recovery and Windows file-sharing issues become a guided repair flow.

### Windows Path Safety Scanner

Scan active store/export/import paths for:

- reserved device names;
- alternate data streams;
- trailing dot/space;
- long path risk;
- reparse points;
- case-collision pairs.

Цель: Windows-specific path bugs become visible before they corrupt history/export.

### Artifact Store Identity Inspector

Show for each critical artifact:

- storage key;
- display path;
- final path;
- volume/file ID;
- reparse state;
- last identity verification result.

Цель: support can distinguish missing file, replaced file, moved store and path redirect.

### Export Filename Sanitizer Preview

Before export:

- show generated filenames;
- show original display labels;
- show rejected unsafe names;
- show redacted sensitive path parts;
- show Windows compatibility warnings.

Цель: export stays portable and does not leak command text through filenames.

### Watcher Resync Console

Show:

- watcher backend;
- last cursor/USN;
- overflow/reset status;
- dirty roots;
- full rescan progress;
- mismatches repaired.

Цель: filesystem watchers remain an optimization, not invisible correctness magic.

### Open Handle Conflict Inspector

For Windows artifact write failures:

- show sharing violation counts;
- show retry policy;
- show suspected readers;
- show fallback/quarantine action;
- expose last successful replace.

Цель: antivirus/indexer/parallel-reader issues are diagnosable without corrupting data.

### Delivery Guarantees Inspector

Per pane/client show:

- last persisted seq;
- last sent seq;
- last acked seq;
- replay window;
- unrecoverable gaps;
- duplicate drops.

Цель: reconnect/missed-output bugs become observable instead of anecdotal.

### Outbox Queue Dashboard

Show:

- pending projection/search/export/sync jobs;
- oldest pending age;
- retry counts;
- poison/quarantined jobs;
- worker heartbeat;
- freshness per derived layer.

Цель: user and support can see when history is durable but derived features lag.

### Idempotency Replay Viewer

For sensitive operations:

- operation kind;
- idempotency key scope;
- request hash;
- first result;
- retry count;
- mismatch warnings.

Цель: duplicate export/share/delete/submit behavior is explainable.

### Snapshot Manifest Browser

Show:

- snapshot lineage;
- base/high-water seq;
- parser/projection/redaction versions;
- artifact references;
- missing/corrupt status;
- fallback replay path.

Цель: restore uses explainable checkpoints, not invisible blobs.

### Backup Restore Drill Center

Controls:

- run temp restore;
- verify DB identity;
- verify artifacts;
- rebuild projections/search;
- sample replay panes;
- export drill report.

Цель: backup reliability is proven regularly, not assumed.

### Sync Conflict Timeline

For divergent histories:

- local branch;
- remote branch;
- base seq;
- conflict reason;
- affected command blocks;
- keep both/archive/promote controls.

Цель: terminal history sync remains conservative and transparent.

### Object Store Consistency Probe

For configured backup/sync target:

- upload/read/delete probe;
- version/etag behavior;
- retention/legal hold detection;
- lifecycle warning;
- provider capability profile.

Цель: object storage is treated as a provider with semantics, not as a folder.

### Search Tier Inspector

Show for a query:

- hot/warm/cold tiers included;
- chunks considered;
- chunks skipped by prefilter;
- bytes decompressed;
- partial/complete status;
- policy/redaction mode.

Цель: search performance and privacy become visible instead of mysterious.

### Chunk Catalog Explorer

For developers/support:

- session/pane seq ranges;
- chunk sizes;
- compression codec;
- storage tier;
- checksum state;
- index/prefilter freshness.

Цель: restore/search bugs can be traced to concrete chunks.

### Label Cardinality Analyzer

Show:

- top label keys;
- cardinality per key;
- growth trend;
- unsafe candidate labels;
- recommended demotion to attribute.

Цель: prevent Loki-style high-cardinality explosions before they hit users.

### Search Index Rebuild Center

Controls:

- rebuild by session/pane/range;
- rebuild after tokenizer change;
- rebuild after redaction profile change;
- pause/resume optimize;
- compare old/new query fixtures.

Цель: search can evolve without corrupting canonical history.

### Cold Search Job Runner

For slow historical searches:

- background job;
- progress by chunks/bytes;
- cancel/resume;
- partial results;
- cost/tier warning.

Цель: years of terminal history stay searchable without freezing the app.

### Snippet Policy Preview

Before showing/exporting search results:

- raw/redacted mode;
- matched rule IDs;
- hidden snippet count;
- policy reason codes;
- provenance ranges.

Цель: search does not become a secret-leak side channel.

### AI Context Inspector

Before sending terminal history to AI, show:

- included command blocks;
- data-only output items;
- redaction profile;
- token/byte budget;
- sensitive finding count;
- source ranges.

Цель: user can see exactly what terminal history becomes model context.

### Prompt Injection Findings Panel

Show:

- risky ranges;
- finding kind;
- severity/confidence;
- source command/output;
- action impact;
- suppress/false-positive workflow.

Цель: prompt-injection risk is visible and testable, not hidden in logs.

### AI Action Approval Gate

For model-requested actions:

- exact payload preview;
- command/file/export diff;
- source context;
- risk findings;
- allow once/deny/edit controls.

Цель: AI can help, but the product keeps deterministic control over side effects.

### Terminal AI Red-Team Lab

Developer QA surface:

- run fixture set;
- compare model/tool policy versions;
- count blocked/unexpected actions;
- inspect leaked snippets;
- export failure report.

Цель: prompt-injection defenses become regression-tested.

### MCP Resource Permission Matrix

For MCP/agent integrations:

- resource names;
- raw/redacted mode;
- allowed actions;
- schema version;
- last access;
- permission changes.

Цель: terminal history exposed to tools is scoped and auditable.

### AI Provenance Timeline

For each AI answer/action:

- user request;
- selected context items;
- command/output ranges;
- summaries used;
- tool calls proposed;
- approvals and policy decisions.

Цель: user can audit how terminal history turned into an AI suggestion or action.

### Persistence Invariant Dashboard

Show:

- invariant list;
- last pass/fail;
- affected component;
- severity;
- checker type;
- linked failure artifact.

Цель: reliability claims become inspectable and tied to real checks.

### Fault Injection Scenario Runner

Developer/support tool:

- choose scenario;
- choose seed;
- choose fault profile;
- run simulation;
- inspect invariant failures;
- save minimized repro.

Цель: crash/retry/reconnect bugs become reproducible.

### Release Reliability Checklist

For each release:

- migration fixtures;
- crash simulations;
- Windows path tests;
- restore drills;
- AI red-team;
- search rebuild;
- backup roundtrip.

Цель: persistence changes cannot ship with hidden untested guarantees.

### Failure Replay Library

Catalog:

- seed;
- schedule;
- fault plan;
- DB fixture;
- event trace;
- screenshots/logs;
- fixed-by commit.

Цель: every weird reliability bug becomes a permanent regression test.

### Model Check Counterexample Viewer

Show:

- protocol spec;
- state trace;
- violated invariant;
- suggested fixture translation;
- linked implementation test.

Цель: formal bugs become actionable for normal engineering workflow.

### Chaos Experiment Scorecard

For controlled chaos runs:

- hypothesis;
- faults injected;
- user-visible state;
- recovery time;
- data-loss result;
- follow-up bugs.

Цель: chaos testing measures reliability, not just breakage.

### Encryption Status Badge

Show per workspace/session:

- DB encryption state;
- artifact encryption state;
- search/AI cache encryption state;
- key profile;
- recovery profile;
- last crypto verification.

Цель: encryption state is visible and not assumed.

### Key Hierarchy Inspector

Developer/support view:

- root/KEK/DEK tree;
- key versions;
- wrapped key states;
- rotation jobs;
- destroyed/retired keys;
- affected artifacts.

Цель: key lifecycle bugs are diagnosable without exposing key material.

### Crypto Erase Simulator

Before deleting sensitive history:

- show chunks/projections/exports affected;
- show keys to destroy;
- show backups/object versions/legal holds;
- show what remains physically possible;
- produce deletion report.

Цель: deletion promises become honest and auditable.

### Key Rotation Wizard

Guided flow:

- create new key version;
- rewrap DEKs;
- verify decrypt samples;
- switch active version;
- retire old key;
- schedule destruction.

Цель: rotation/rekey is safe and observable.

### Recovery Readiness Check

Show:

- selected recovery profile;
- last recovery test;
- recovery key age;
- escrow/passphrase status;
- risk if OS account/device is lost.

Цель: users understand whether history is recoverable before disaster.

### Encrypted Export Verifier

For export bundles:

- verify manifest;
- verify recipient/passphrase;
- verify decrypt sample;
- verify redaction profile;
- show raw vs redacted contents.

Цель: backup/export encryption is tested before user relies on it.

## Recommended implementation plan

### Milestone 0 - Architecture invariants

Оценка: 🎯 10   🛡️ 10   🧠 5  
Объем: примерно 400-800 строк docs/tests

- Зафиксировать restore semantics:
  - native restores history but not process state;
  - zellij/tmux attach can preserve process state only when live mux session exists.
- Зафиксировать privacy defaults:
  - raw input off;
  - command text via trusted sources;
  - private mode;
  - redaction before export/AI.
- Зафиксировать side-effect policy:
  - historical replay cannot trigger clipboard/window/link side effects.
- Зафиксировать writer policy:
  - single writer;
  - batching;
  - WAL checkpoint;
  - degraded state visible.

## Open design questions

### Где резать stream segment

Варианты:

1. По размеру: 32-64 KB.  
   🎯 9   🛡️ 9   🧠 4  
   Самый простой и надежный batching для SQLite.

2. По времени: 50-100 ms.  
   🎯 8   🛡️ 8   🧠 5  
   Хорошо для crash recovery и live UI, но может дать много маленьких segments при low-volume output.

3. Гибрид: flush по первому наступившему size/time/command-boundary.  
   🎯 10   🛡️ 10   🧠 6  
   Лучший вариант.

Рекомендация: гибрид.

### Что считать output команды

Варианты:

1. От `pre_exec` до `command_finish`.  
   🎯 9   🛡️ 8   🧠 5  
   Хорошо при shell integration.

2. От UI submit до следующего prompt.  
   🎯 7   🛡️ 6   🧠 6  
   Работает для command dock, но ломается на interactive/TUI.

3. Sequence range + confidence/trust.  
   🎯 10   🛡️ 9   🧠 7  
   Лучший вариант: разные источники дают разную точность.

Рекомендация: sequence range + confidence.

### Как хранить rendered text для поиска

Варианты:

1. Только raw journal, поиск строить replay-on-demand.  
   🎯 5   🛡️ 8   🧠 5  
   Просто в БД, но медленно.

2. Derived `terminal_search_index` с text chunks.  
   🎯 9   🛡️ 8   🧠 7  
   Быстрый поиск, но нужна invalidation/versioning.

3. FTS5 virtual table.  
   🎯 8   🛡️ 8   🧠 8  
   Мощно, но надо проверить SQLite bundled feature/build.

Рекомендация: начать с derived chunks, FTS5 позже после проверки bundled SQLite.

### Milestone 1 - Diesel persistence skeleton

Оценка: 🎯 9   🛡️ 9   🧠 7  
Объем: примерно 1200-2000 строк

- Add `diesel` + `diesel_migrations`.
- Add migrations for blocks/events/segments/snapshots/policy.
- Add repository API.
- Add batch writer API.
- Unit tests:
  - open DB;
  - insert commands;
  - insert stream segments;
  - reopen DB;
  - query by session/pane;
  - prune by policy.

### Milestone 2 - Native journal writer

Оценка: 🎯 8   🛡️ 9   🧠 8  
Объем: примерно 1500-2500 строк

- Capture PTY output segments.
- Capture UI-submitted commands.
- Persist resize/title/clear/exit events.
- Periodic snapshots from projection.
- Degraded state on write failures.
- Browser smoke: run commands, restart daemon/browser, verify history visible.

### Milestone 3 - Shell integration

Оценка: 🎯 8   🛡️ 9   🧠 9  
Объем: примерно 1800-3200 строк

- Parse `OSC 133`.
- Parse `OSC 633`.
- Parse cwd protocols: `OSC 7`, `OSC 9;9`, `OSC 633 P Cwd`.
- PowerShell integration.
- cmd.exe fallback.
- trust/nonce handling.
- spoofing tests.

### Milestone 4 - Restore UX

Оценка: 🎯 9   🛡️ 9   🧠 8  
Объем: примерно 1500-2800 строк

- Hydrate from snapshots.
- Replay journal after snapshot.
- Show command blocks and transcript.
- Mark historical/live boundary.
- Rerun restored commands only by explicit user action.

### Milestone 5 - Zellij path

Оценка: 🎯 7   🛡️ 8   🧠 9  
Объем: примерно 1500-3000 строк

- Persist pane surface deltas/snapshots.
- Detect shell integration through zellij pane.
- Preserve zellij attach vs resurrection semantics.
- Never treat zellij raw PTY as single-pane shell transcript.

### Milestone 6 - Hardening, export/import and recovery

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3200 строк

- Add event envelope and schema registry.
- Add SQLite defensive/limit initialization.
- Add import quarantine and export manifest.
- Add checksum/AEAD artifact metadata.
- Add writer health metrics and SLO tests.
- Add recovery/rebuild UI hooks for projections and search indexes.

### Milestone 7 - Privacy, accessibility and artifact lifecycle

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1600-3000 строк

- Add temp artifact registry and cleanup job.
- Add secret/key lifecycle events.
- Add retention review/simulator.
- Add accessible history announcements.
- Add keyboard escape/navigation tests for terminal pane.
- Add artifact quarantine states for images/files/binary payloads.

### Milestone 8 - Durable jobs, scalable search and AI context

Оценка: 🎯 9   🛡️ 9   🧠 8  
Объем: примерно 1800-3400 строк

- Add `terminal_background_jobs`.
- Add search profile/version model.
- Add FTS5 derived index path with rebuild jobs.
- Add DB identity guard with `application_id`/`user_version`.
- Add time anchors and monotonic replay deltas.
- Add AI context export records and redaction gate.

### Milestone 9 - Windows lifecycle, artifact integrity and backup consistency

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3200 строк

- Add Windows process lifecycle events.
- Add Job Object/ConPTY shutdown tests.
- Add artifact integrity checks and watcher hint table.
- Add content-addressed artifact manifest rules.
- Add consistency snapshot records for backup/export.
- Add restore guarantee matrix for native/mux/checkpoint paths.

### Milestone 10 - Transport recovery, WSL/SSH domains and rollout controls

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3300 строк

- Add client delivery state and ack/replay protocol.
- Add reconnect gap UI and tests.
- Add execution domain model for Windows/WSL/SSH/mux.
- Add remote metadata redaction profile.
- Add feature flag state capture and kill switches.
- Add transport chaos fixtures for WebSocket/SSH/zellij paths.

### Milestone 11 - Schema evolution, query performance and governance

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3400 строк

- Add payload codec registry and upcaster tests.
- Add query plan baselines for restore/search/prune/jobs.
- Add diagnostic report privacy gate.
- Add storage quota samples and pressure policy.
- Add legal hold/enterprise retention model.
- Add schema evolution workbench fixtures.

### Milestone 12 - Browser lifecycle, side effects and container domains

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3300 строк

- Add browser client ownership state.
- Add side-effect event model for clipboard/paste/link/file/media.
- Add container execution context model.
- Add Windows encoding profiles and projection rebuild tests.
- Add sleep/resume power event handling.
- Add Playwright/browser lifecycle tests for hidden/reload/multi-tab ownership.

### Milestone 13 - Local gateway, import safety and viewer sandboxing

Оценка: 🎯 10   🛡️ 10   🧠 8  
Объем: примерно 1800-3200 строк

- Add gateway token scope/expiry model.
- Add Origin/Host/PNA validation tests.
- Add gateway security event audit table.
- Add named pipe/local IPC endpoint registry.
- Add safe import quarantine pipeline.
- Add transcript viewer CSP/sandbox profiles.
- Add desktop wrapper security checklist for Electron/Tauri/native shell.

### Milestone 14 - Atomic artifacts, safe redaction and authorization policy

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3400 строк

- Add artifact write transaction ledger.
- Add same-directory temp+flush+atomic replace path.
- Add Windows ReplaceFile/MoveFileEx/FlushFileBuffers tests.
- Add stale lock diagnostics and recovery.
- Add ReDoS-safe redaction rule engine.
- Add rule-set versioning, fixtures and per-rule metrics.
- Add centralized subject/object/action/environment policy boundary.
- Add policy decision audit table.
- Add attenuated capability grants for share/export/support/AI access.

### Milestone 15 - Windows artifact path safety and identity verification

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 1800-3600 строк

- Add generated storage keys separated from display names.
- Add Windows reserved-name/ADS/long-path/reparse fixtures.
- Add canonical root guard and path resolution audit table.
- Add handle/final-path/file-id verification for critical artifacts.
- Add case sensitivity and collision tests.
- Add watcher/USN dirty-root records and full-rescan fallback.
- Add export filename sanitizer and preview.
- Add sharing-violation retry/quarantine diagnostics.

### Milestone 16 - Delivery semantics, outbox and backup restore drills

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 2200-4400 строк

- Add per-pane delivery offsets and browser ack/replay tests.
- Add scoped idempotency key table for submit/export/share/delete/rerun.
- Add transactional outbox for projection/search/export/sync jobs.
- Add inbox dedup records for import/sync.
- Add snapshot manifests with parent/high-water sequence lineage.
- Add object-store sync capability profile and version/ETag recording.
- Add backup restore drill runner with temp restore and sample replay.
- Add sync conflict branches instead of silent transcript merges.

### Milestone 17 - Scalable search, chunk catalog and hot/warm/cold lifecycle

Оценка: 🎯 9   🛡️ 9   🧠 8  
Объем: примерно 2200-4600 строк

- Add chunk catalog over stream segments.
- Add low-cardinality label policy and tests.
- Add chunk prefilters for time/seq/token summaries.
- Add search index metadata with tokenizer/redaction/parser versions.
- Add query budgets, partial result state and resume cursor.
- Add hot/warm/cold lifecycle tier events.
- Add search authorization/snippet policy checks.
- Add index rebuild/merge/optimize worker diagnostics.

### Milestone 18 - AI context safety and prompt-injection resistant terminal agents

Оценка: 🎯 9   🛡️ 10   🧠 8  
Объем: примерно 2200-4800 строк

- Add structured AI context package model.
- Mark terminal output/search snippets as data-only by default.
- Add prompt-injection finding records over stream ranges.
- Add deterministic AI tool/action policy checks.
- Add user approval records for terminal/file/share/export/delete actions.
- Add context preview with token/byte/sensitive finding budgets.
- Add AI red-team fixture runner for terminal output attacks.
- Add MCP/resource permission matrix and tool capability drift checks.

### Milestone 19 - Reliability proof, deterministic simulation and release gates

Оценка: 🎯 9   🛡️ 10   🧠 9  
Объем: примерно 2600-5600 строк

- Add executable persistence invariant registry.
- Add named failpoints around DB, writer, artifact, outbox, transport and AI boundaries.
- Add seeded deterministic simulator for writer/reconnect/outbox/snapshot protocols.
- Add model-check specs for seq/ack, idempotency, outbox and tombstone flows.
- Add concurrency schedule tests for Rust async writer/worker races.
- Add failure replay artifact storage and minimization workflow.
- Add release reliability checklist gates.
- Add chaos experiment scorecards with steady-state hypotheses.

### Milestone 20 - Encryption, key lifecycle and cryptographic erase

Оценка: 🎯 9   🛡️ 10   🧠 9  
Объем: примерно 2800-6200 строк

- Add key hierarchy and wrapped key metadata.
- Add SQLCipher/DB encryption initialization profile.
- Add encrypted external artifact metadata and AAD verification.
- Add OS key-store capability probes for Windows/macOS/Linux.
- Add crash-safe key rotation/rekey jobs.
- Add cryptographic erase records and limitation reports.
- Add encrypted export/backup verification.
- Add crypto test vectors, tamper tests and duplicate nonce tests.

## Final recommendation

Делать нужно не "history feature", а `Terminal Persistence v2`.

Минимально правильный scope:

- Diesel tables for journal/history.
- Per-session/per-pane command blocks.
- Append-only stream segments.
- Screen snapshots.
- Shell integration quality model.
- Privacy/retention policies.
- Explicit restore semantics.

Самая важная продуктовая цель:

- Пользователь после перезапуска видит не просто "последние команды", а полный рабочий контекст: что вводил, что получил, где запускал, чем завершилось, что можно повторить, и где начинается новая live-сессия.
