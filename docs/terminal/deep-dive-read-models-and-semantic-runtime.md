# Deep Dive - Read Models and Semantic Runtime

**Проверено**: 2026-04-19  
**Фокус**: `OpenCove` terminal read-model surfaces и `Hermes IDE` semantic runtime / analyzer boundary

## Почему этот слой важен

После нескольких deep dive стало видно, что хороший terminal feature почти всегда распадается не на "renderer + PTY", а минимум на 4 разные вещи:

- live PTY stream
- durable scrollback / recovery state
- UI read models вроде local find, links, visible transcript
- semantic runtime поверх output

Если это слить в один слой, почти неизбежно появляются:

- лишние re-render и dirty churn
- путаница между "что видел пользователь" и "что приходит из PTY"
- search и links, которые завязаны на случайные детали renderer
- AI/context logic, которая начинает загрязнять terminal truth

`OpenCove` и `Hermes IDE` здесь особенно полезны, потому что оба проекта уже явно разрезают эти слои.

## `OpenCove` - terminal как набор read models, а не один buffer

### 1. Terminal-local find отделён от global workspace search

Это видно не только по UI-коду, но и по e2e:

- [`TerminalNodeFindBar.tsx`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/TerminalNodeFindBar.tsx)
- [`useTerminalFind.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useTerminalFind.ts)
- [`workspace-canvas.terminal-find.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/workspace-canvas.terminal-find.spec.ts)

Что важно:

- `Cmd/Ctrl+F`, когда фокус внутри terminal, открывает именно terminal-local find
- global workspace search при этом не должен перехватывать shortcut
- find bar держит собственный state: `query`, `resultIndex`, `resultCount`, `caseSensitive`, `useRegex`

Это хороший product rule:

- local terminal find и global workspace search должны быть разными surfaces с разной ownership model

### 2. Search addon включается только если foundation реально поддерживает нужный hook

[`searchAddonSupport.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/searchAddonSupport.ts) делает простую, но очень правильную вещь:

- не предполагает, что terminal всегда готов к `SearchAddon`
- сначала проверяет наличие `onWriteParsed`
- только потом грузит `SearchAddon`

То есть search рассматривается как capability-bound extension, а не как unconditional часть terminal core.

Это полезный архитектурный урок:

- optional terminal surfaces лучше вешать через feature detection, а не через слепую инициализацию

### 3. Incremental search уже оформлен как отдельный runtime policy

`useTerminalFind.ts` показывает хороший shape search behavior:

- пока query пустой, decorations очищаются
- при вводе используется `findNext(..., { incremental: true })`
- result counters живут отдельно от raw query
- decorations theme-aware и адаптируются под light/dark terminal mode

Это небольшой, но важный паттерн:

- search UX должен иметь собственную state machine, а не быть просто вызовом `addon.findNext()`

### 4. `OpenCove` уже использует несколько terminal read models одновременно

По коду видно как минимум 4 read model слоя:

1. live xterm buffer
2. durable scrollback snapshot
3. visible-text transcript mirror
4. search results / selection surface

Это видно в:

- [`useScrollback.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useScrollback.ts)
- [`useScrollbackStore.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/store/useScrollbackStore.ts)
- [`useTerminalTestTranscriptMirror.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useTerminalTestTranscriptMirror.ts)

Самый важный вывод:

- terminal feature не должен заставлять один и тот же buffer решать сразу и restore, и search, и debug transcript, и persistence

### 5. Durable scrollback ownership отделена от live runtime

`useTerminalScrollback()` использует rolling text buffer и отдельный publish scheduling:

- scrollback хранится в bounded buffer
- publish дебаунсится
- pending state не сразу пробрасывается наружу
- pointer resize может временно остановить publish

А `useScrollbackStore.ts` хранит уже нормализованный durable scrollback `by nodeId`.

Это подтверждает хороший boundary:

- scrollback persistence должна быть read model над terminal output, а не прямой owner renderer lifecycle

### 6. Visible transcript mirror - это отдельная проекция, не scrollback

[`useTerminalTestTranscriptMirror.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useTerminalTestTranscriptMirror.ts) очень полезен как pattern:

- читает именно visible text из active buffer
- отдельно держит persisted transcript per `nodeId`
- синхронизируется через `requestAnimationFrame`
- экспортирует debug API в `window` только для test tooling

Это сильно отличается от durable scrollback:

- transcript mirror отвечает на вопрос "что сейчас видно"
- scrollback mirror отвечает на вопрос "что можно восстановить"

🔥 Это одна из самых полезных найденных границ.

### 7. Restore policy зависит от kind of node

Интеграционный тест:

- [`useHydrateAppState.scrollback-ownership.spec.tsx`](https://github.com/DeadWaveWave/opencove/blob/main/tests/integration/recovery/useHydrateAppState.scrollback-ownership.spec.tsx)

показывает ещё один важный product rule:

- terminal nodes получают durable scrollback preload до завершения runtime hydration
- agent nodes не должны blindly получать stale durable scrollback

Причина понятная:

- stale agent history выглядит как ложное восстановление чужого semantic state

Это хороший reminder:

- restore/read-model policy может зависеть от session role, не только от transport mechanics

### 8. Link detection у `OpenCove` - это целый subsystem

По коду:

- [`multi-line-link-provider.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/multi-line-link-provider.ts)
- [`file-path-link-provider.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/file-path-link-provider.ts)
- [`url-link-provider.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/url-link-provider.ts)
- [`link-parsing.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/link-parsing.ts)

там уже видно зрелый design:

- базовый `MultiLineLinkProvider` умеет ranges across wrapped lines
- file path parser портирован из VS Code terminal link parsing
- есть fallback matchers для специальных форматов
- URL provider умеет расширять hard-wrapped URL через несколько line continuations
- activation file/url links gated на `Cmd/Ctrl + Click`

Здесь главный урок:

- useful terminal links почти никогда не решаются одной регуляркой по одной строке

### 9. Link subsystem осознанно пытается избегать ложных positive matches

Даже по одному `file-path-link-provider.ts` видно, что авторы явно вычищают мусор:

- пропускают URLs, если это не file path
- отсекают version strings вроде `v1.2.3`
- отсекают npm package references
- декодируют URL-encoded paths
- умеют разбирать line/column suffixes

Это полезный product rule:

- link subsystem должен иметь false-positive policy, а не только detection power

## `Hermes IDE` - semantic runtime не равен app context

### 1. Analyzer живёт на PTY hot path и это осознано

В `Hermes` semantic analysis сидит в Rust, рядом с PTY:

- [`src-tauri/src/pty/analyzer.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/analyzer.rs)
- [`src-tauri/benches/analyzer.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/benches/analyzer.rs)

Это важно не только технически.  
Они прямо бенчат analyzer как hottest path:

- `process()` на realistic 4KB chunks
- `to_metrics()`
- throughput and eviction paths

То есть analyzer у них не "дополнительная магия", а first-class runtime subsystem с perf budget.

### 2. Analyzer - это bounded semantic cache, а не бесконечный event log

`OutputAnalyzer` хранит много сигналов, но почти везде с bounded caps:

- `output_lines`
- token usage + short token history
- bounded `tool_calls`
- bounded `files_ordered`
- bounded `recent_actions`
- bounded `memory_facts`
- bounded `completed_nodes`
- stripped buffer capped примерно до `16KB`

Это очень хороший pattern:

- semantic runtime должен быть small, bounded, query-friendly cache
- не надо превращать его в неограниченное хранилище всей terminal истории

### 3. Provider registry отделяет generic parsing от provider-specific parsing

В:

- [`adapters.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/adapters.rs)
- [`patterns.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/patterns.rs)

виден правильный shape:

- `ProviderAdapter` trait
- detection, `analyze_line`, `is_prompt`, `known_actions`
- fallback generic analysis, если provider ещё не detected

Это означает:

- Claude/Codex/Gemini/Aider specifics не вшиваются в один giant regex pile без границ

Для нашей feature это очень полезный переносимый pattern.

### 4. Analyzer строит именно execution summaries, а не только metrics

`NodeBuilder` и `CompletedNode` в `analyzer.rs` особенно интересны:

- захватывают `input`
- копят ограниченный `output_summary`
- хранят `working_dir`
- считают `duration_ms`
- классифицируют node как `command` или `ai_interaction`

Это почти готовая база для:

- execution timeline
- command blocks
- recent commands
- lightweight semantic history

То есть timeline можно строить не только по shell markers, а ещё и по analyzer-side summaries.

### 5. Phase machine связана с analyzer, а не только с UI

В `models.rs` фазы довольно зрелые:

- `Creating`
- `Initializing`
- `ShellReady`
- `LaunchingAgent`
- `Idle`
- `Busy`
- `NeedsInput`
- `Error(...)`
- `Disconnected`

А в `analyzer.rs` `PhaseHint` живёт рядом с parsing:

- `PromptDetected`
- `WorkStarted`
- `InputNeeded`

Это сильный вывод:

- часть truth о phase реально живёт в runtime analyzer, а не в renderer/UI-only store

### 6. Silence handling сделана как separate policy

`check_silence()` в `analyzer.rs` очень показателен:

- если prompt не распознан, но shell вероятно ready, они умеют перейти в `ShellReady`
- если prompt среди последних линий не найден, а agent уже detected, выбирают `NeedsInput`

Это полезный pattern:

- "нет нового output" тоже должно иметь явную semantic policy, а не быть просто timeout без interpretation

### 7. Analyzer tracking у `Hermes` уже богатый, но не лезет в app identity context

`SessionMetrics` / `SessionData` в:

- [`src/types/session.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/types/session.ts)

включают:

- token usage
- tool calls
- files touched
- recent actions
- available actions
- memory facts
- latency
- token history

Но дальше очень важно, что tests around context layer отдельно запрещают смешивать это с context truth.

### 8. `ContextState` намеренно excludes ephemeral execution data

Это видно в:

- [`src/types/context.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/types/context.ts)
- [`context-layer-invariants.test.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-layer-invariants.test.ts)

Они прямо фиксируют архитектурный инвариант:

- `errorResolutions`, `filesTouched`, `recentErrors` не должны жить в `ContextState`
- не должны попадать в formatted context markdown
- не должны вызывать context version churn

🔥 Это очень сильный pattern для нашей будущей feature:

- identity context и ephemeral execution context обязаны быть разными owner tables

### 9. Session reducer отдельно режет noisy updates

Даже в [`SessionContext.tsx`](https://github.com/hermes-hq/hermes-ide/blob/main/src/state/SessionContext.tsx) видно, что они пытаются не дёргать UI без смысла:

- `SESSION_UPDATED` скипается, если поля не изменились meaningfully
- workspace dirty flag живёт отдельно
- layout/session restore guarded от лишних повторных срабатываний

Это поддерживает общую идею:

- semantic runtime может быть богатым, но UI projection должен быть intentionally lossy и stable

### 10. Bug tests у них фактически документируют architectural contracts

Очень полезны:

- [`terminal-intelligence-bugs.test.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/terminal-intelligence-bugs.test.ts)
- [`session-context-bugs.test.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/session-context-bugs.test.ts)

Там видно, что они сознательно защищают:

- trimming and dedupe in history/command lookup
- context cache invalidation on cwd change
- shell environment cleanup
- отсутствие drift между session removal и per-session state

То есть tests у них реально закрепляют architecture boundaries, а не только пользовательские сценарии.

## Сводка - что отсюда стоит утащить

### Из `OpenCove`

- terminal-local find как отдельный surface
- capability-check для optional addon layers
- durable scrollback, visible transcript и search как разные read models
- role-aware restore policy для `terminal` vs `agent`
- wrapped link subsystem с false-positive policy

### Из `Hermes`

- bounded semantic analyzer рядом с PTY hot path
- provider registry для agent-specific parsing
- execution node summaries как база для timeline
- separate identity context vs ephemeral execution context
- architectural tests на runtime/context boundaries

## Главный общий вывод

🔥 Один из самых важных выводов всего ресёрча сейчас такой:

- **terminal feature должна иметь отдельные read models**
- **semantic runtime не должен автоматически становиться app context**

Если это не разделить, дальше почти неизбежно получится смесь из:

- restore truth
- UI projection
- search state
- analytics/AI metadata
- context injection payload

Именно это обычно делает terminal feature хрупкой и плохо расширяемой.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [OpenCove - useTerminalFind.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useTerminalFind.ts)
- [OpenCove - searchAddonSupport.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/searchAddonSupport.ts)
- [OpenCove - useScrollback.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useScrollback.ts)
- [OpenCove - useTerminalTestTranscriptMirror.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/useTerminalTestTranscriptMirror.ts)
- [OpenCove - file-path-link-provider.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/file-path-link-provider.ts)
- [OpenCove - url-link-provider.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/linkProviders/url-link-provider.ts)
- [OpenCove - workspace-canvas.terminal-find.spec.ts](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/workspace-canvas.terminal-find.spec.ts)
- [Hermes IDE - analyzer.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/analyzer.rs)
- [Hermes IDE - analyzer benchmark](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/benches/analyzer.rs)
- [Hermes IDE - models.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/models.rs)
- [Hermes IDE - adapters.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/adapters.rs)
- [Hermes IDE - ContextState types](https://github.com/hermes-hq/hermes-ide/blob/main/src/types/context.ts)
- [Hermes IDE - Session types](https://github.com/hermes-hq/hermes-ide/blob/main/src/types/session.ts)
- [Hermes IDE - context-layer-invariants.test.ts](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-layer-invariants.test.ts)
