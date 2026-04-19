# Deep Dive: Stream Recovery, Durable Scrollback, And Semantic Analysis

**Проверено**: 2026-04-19

Этот файл про слой, который отделяет "терминал что-то показывает" от "terminal feature действительно переживает reconnect, overflow и долгие сессии".

Здесь особенно полезны два типа проектов:

- `OpenCove` - как строить replay/resync/durable scrollback вокруг PTY stream
- `Hermes IDE` - как строить semantic session state поверх raw PTY bytes

## 1. `OpenCove` - replay и persistence у них уже похожи на protocol layer

### Какие исходники особенно полезны

- `src/app/main/controlSurface/ptyStream/ptyStreamHub.ts`
- `src/app/main/controlSurface/remote/remotePtyStreamMessageHandler.ts`
- `src/app/main/ipc/ptyScrollbackMirror.ts`

### Что стало понятнее

#### A. PTY stream treated as bounded replay log, not just broadcast

`ptyStreamHub.ts` делает несколько важных вещей сразу:

- батчит pending output с `PTY_DATA_FLUSH_DELAY_MS = 32`
- форсит flush при `PTY_DATA_MAX_BATCH_CHARS = 256_000`
- поднимает `seq` на каждый flushed chunk
- хранит bounded replay window по байтам
- при вытеснении старых chunks выставляет `truncated = true`

🔥 Это очень хороший shape.

Не "держим весь scrollback в памяти", а:

- live pending buffer
- bounded replay log
- явный `truncated`

#### B. Attach protocol is explicit about replay gaps

В `attach()` клиент приходит с `afterSeq`.

Дальше логика очень правильная:

- если у клиента offset ещё внутри replay window, ему докидывают пропущенные chunks
- если offset уже старее `earliestSeq - 1`, сервер шлёт `overflow`
- роли `controller` и `viewer` живут прямо в attach semantics

Полезный вывод:

- reconnect не должен быть "blind replay"
- gap detection нужно делать на уровне протокола
- input-control semantics удобно держать рядом с stream attach

#### C. Overflow turns into snapshot resync, not silent data loss

`remotePtyStreamMessageHandler.ts` особенно ценен тем, что `overflow` не трактуется как тупик.

Он:

- держит `attachedSessions.lastSeq`
- обрабатывает сообщения `hello_ack`, `attached`, `data`, `exit`, `overflow`, `control_changed`, `error`
- на `overflow` вызывает `snapshot(sessionId)`
- если snapshot получен, пушит его подписчикам как новый `ptyData`

🔥 Это очень сильный recovery pattern:

- overflow не обязан означать broken session
- можно иметь explicit resync path
- resync path лучше делать отдельно от обычного replay

#### D. Durable scrollback lives in a separate mirror adapter

`ptyScrollbackMirror.ts` показывает очень зрелый pattern:

- bindings задают, какие `sessionId -> nodeId[]` сейчас надо зеркалить
- mirror не сидит в hot path PTY stream
- он раз в `5000ms` делает snapshot и пишет его в persistence store
- snapshot dedupe делается не полным compare, а fingerprint-ом:
  - `length`
  - `tail` последних `128` символов
- на rebinding и dispose он делает final flush
- запись сериализуется через `operationChain`

Это уже практически blueprint для durable scrollback adapter.

Полезные детали:

- отдельная mirror subsystem лучше, чем тащить persistence прямо в transport
- fingerprint dedupe сильно уменьшает лишние записи
- flush on binding change не даёт потерять последний state при перестройке UI

### Что утащить как идею

- bounded replay window with explicit `truncated`
- `afterSeq` attach semantics instead of blind replay
- overflow should trigger explicit snapshot resync path
- durable scrollback mirror as separate persistence adapter
- fingerprint-based snapshot dedupe
- periodic flush plus flush-on-rebind/dispose

---

## 2. `Hermes IDE` - semantic session state живёт не в renderer, а в analyzer layer

### Какие исходники особенно полезны

- `src-tauri/src/pty/mod.rs`
- `src-tauri/src/pty/models.rs`
- `src-tauri/src/pty/adapters.rs`
- `src-tauri/src/pty/analyzer.rs`
- `src-tauri/src/pty/shell_integration.rs`

### Что стало понятнее

#### A. Session phase is a first-class contract

`models.rs` оформляет phase как нормальный enum:

- `Creating`
- `Initializing`
- `ShellReady`
- `LaunchingAgent`
- `Idle`
- `Busy`
- `NeedsInput`
- `Error`
- `Closing`
- `Disconnected`
- `Destroyed`

Это полезно потому, что UX дальше можно строить не от "focused / unfocused terminal", а от explicit runtime state.

#### B. Provider-specific semantics are isolated behind adapters

`adapters.rs` показывает простой, но сильный seam:

- `detect_agent(line)`
- `analyze_line(line)`
- `is_prompt(line)`
- `known_actions()`

То есть provider-specific parsing не размазан по UI и не зашит в общий reader loop.

Это правильный уровень абстракции для:

- Claude Code / Codex / Aider / Gemini
- future provider quirks
- token usage parsing
- prompt detection
- tool/action extraction

#### C. Output analyzer owns semantic state, not only busy/idle

`analyzer.rs` оказался богаче, чем казалось по README.

`OutputAnalyzer` держит:

- detected agent/provider
- `token_usage` и `token_history`
- `tool_calls` и `tool_call_summary`
- `files_touched`
- `recent_actions`
- `memory_facts`
- latency samples
- `current_cwd` / `pending_cwd`
- `node_builder` и `completed_nodes`
- `recent_commands`
- input line buffer
- idle timers
- flags типа `shell_ready`, `pending_ai_launch`, `pending_context_inject`

⚠️ Это важно: analyzer у них это уже не "regex around output", а почти semantic runtime cache.

#### D. Phase changes are derived from hints, not hardcoded into renderer

Поток у Hermes примерно такой:

1. strip ANSI один раз
2. mark visible output as `Busy`
3. parse `OSC 7` для cwd tracking
4. detect agent/model
5. прогнать lines через provider adapter
6. получить `PhaseHint`
   - `PromptDetected`
   - `WorkStarted`
   - `InputNeeded`
7. перевести session phase и execution node state

Особенно полезно, что:

- prompt может завершать active execution node
- `InputNeeded` отдельно отличается от `Idle`
- cwd tracking живёт в analyzer, а не в shell wrapper-only коде

#### E. Deferred nudges are phase-gated

`mod.rs` даёт ещё один очень хороший pattern:

- если agent detected, но текущая фаза не `NeedsInput`, контекстный nudge не шлётся сразу
- он кладётся в `pending_nudge`
- потом доставляется, когда сессия реально переходит в input-acceptable state

Это сильный вывод для future automation/agent hooks:

- app-side writes не должны blindly ломиться в PTY
- многие writes логичнее gate-ить фазой

#### F. Shell integration is also a compatibility layer

`shell_integration.rs` полезен тем, что там shell integration понимается шире, чем просто экспорт env vars.

Они:

- подмешивают temp shell init files
- экспортируют `HERMES_TERMINAL=1`
- отключают конфликтующие autosuggestion/completion plugins
  - `zsh-autosuggestions`
  - `zsh-autocomplete`
  - fish autosuggestions
  - `ble.sh`
- отдельно чинят startup resize race через `SIGWINCH`

🔥 Это хороший reminder:

shell integration - это ещё и compatibility layer between app UX and user shell ecosystem.

### Что утащить как идею

- phase enum as product contract
- provider adapter registry for output semantics
- analyzer-owned semantic state cache
- explicit `NeedsInput` state separate from `Idle`
- phase-gated deferred writes/nudges
- shell integration as compatibility layer, not only env injection

---

## 3. Practical synthesis for our future feature

После этого deep dive картина стала ещё чётче:

### A. Live stream, replay, and durable scrollback should be three different concerns

- live stream - hot path to visible terminal
- replay window - reconnect helper
- durable scrollback mirror - persistence concern

Если их смешать в один слой, получится хрупкая subsystem.

### B. Recovery should be explicit

Нужны отдельные state transitions для:

- normal replay
- overflow
- snapshot resync
- disconnected / reattached

### C. Semantic state should not depend on renderer implementation

Session phase, cwd, detected agent, recent commands, last important events и similar signals стоит держать в runtime/analyzer layer, а не вычислять из React component state.

### D. Smart writes should be phase-aware

Automation hooks, future agent nudges, quick actions и contextual injections лучше gate-ить по session phase, иначе легко ломать реальный user workflow внутри PTY.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
