# Deep Dive - Control Surface and Context Orchestration

**Проверено**: 2026-04-19  
**Фокус**: `OpenCove` control surface / topology protocol и `Hermes IDE` backend-authoritative context orchestration

## Почему этот слой критичен

После предыдущих deep dive стало понятно, что terminal feature быстро упрётся не только в renderer/runtime, но и в два системных вопроса:

- как внешние surfaces вообще разговаривают с terminal/workspace runtime
- кто владеет orchestration вокруг shell-ready, agent launch, context apply и input-needed phases

Именно здесь продукты обычно начинают расползаться:

- один и тот же business rule копируется в desktop IPC, web shell и remote worker
- prompt/context logic утекает в frontend
- remote/local differences прячутся за "магическим" sessionId
- retries и resync делаются ad-hoc

`OpenCove` и `Hermes IDE` полезны тем, что оба уже явно борются именно с этим уровнем сложности.

## `OpenCove` - control surface как typed boundary, а не набор случайных IPC calls

### 1. Core abstraction предельно простая: `register` + `invoke`

В [`controlSurface.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/controlSurface.ts) вся база намеренно тонкая:

- handlers регистрируются по `id`
- каждый handler знает свой `kind`
- `invoke(ctx, request)` сначала валидирует payload, потом вызывает handler
- ошибки возвращаются в typed envelope, а не через разные ad-hoc exception styles

Это очень сильный architectural choice:

- boundary делается маленьким и uniform, а сложность живёт в handler layers

### 2. Протокол уже typed и envelope-based

`OpenCove` держит contracts отдельно:

- [`request.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/shared/contracts/controlSurface/request.ts)
- [`result.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/shared/contracts/controlSurface/result.ts)
- [`protocol.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/shared/contracts/controlSurface/protocol.ts)
- [`dto/controlSurface.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/shared/contracts/dto/controlSurface.ts)

Полезные свойства этого shape:

- `kind` чётко различает `query` и `command`
- `ok/value` и `ok/error` живут в одном envelope
- protocol version explicit

Это хороший pattern для terminal feature:

- command/query surface надо проектировать как explicit contract, а не как набор preload methods

### 3. Capability negotiation уже встроена в DTO shape

В `ControlSurfaceCapabilitiesResult` видно, что авторы заранее думают про negotiated feature set:

- `protocolVersion`
- `appVersion`
- `features.webShell`
- `features.sync.state/events`
- `features.sessionStreaming.enabled`
- `ptyProtocolVersion`
- `replayWindowMaxBytes`
- `roles.viewer/controller`
- web auth capabilities

🔥 Это очень сильная идея:

- remote/web/embedded clients должны спрашивать capabilities, а не угадывать поведение сервера по версии приложения

### 4. `OpenCove` держит topology как first-class subsystem

По:

- [`topologyHandlers.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/handlers/topologyHandlers.ts)

видно, что remote/local execution model у них не замазана в session logic.

Есть отдельные операции вокруг:

- `endpoint.list/register/remove/ping`
- `endpoint.homeDirectory`
- `endpoint.readDirectory`
- `mount.list/create/remove/promote`
- mount target resolution

Причём local и remote case не симулируются одним и тем же кодом:

- `local` endpoint умеет короткие fast-path ответы
- часть операций explicit only-for-remote
- local approved roots регистрируются отдельно

Это хороший общий вывод:

- remote topology и terminal session lifecycle не надо сплавлять в один object model

### 5. Execution context возвращается вместе с session launch

В [`sessionStreamingHandlers.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/handlers/sessionStreamingHandlers.ts):

- `session.spawnTerminal`
- `pty.spawn`
- `session.snapshot`

уже оформлены как explicit operations.

Особенно полезно то, что launch response возвращает не только `sessionId`, но и:

- `startedAt`
- `cwd`
- `command/args`
- `executionContext`

А `ExecutionContextDto` сам по себе уже содержит:

- `projectId`
- `spaceId`
- `mountId`
- `targetId`
- `endpoint`
- `target`
- `scope`
- `workingDirectory`

Это очень сильный donor pattern:

- session identity недостаточно
- execution context должен быть first-class payload, иначе дальше сложно делать restore, review, remote routing и UX рядом с terminal

### 6. Sync channel живёт отдельно от command/query channel

[`syncSse.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/http/syncSse.ts) показывает ещё один зрелый choice:

- есть отдельный SSE event channel
- payload types пока маленькие и понятные:
  - `app_state.updated`
  - `resync_required`

Очень полезный вывод:

- control plane и state sync plane лучше держать отдельно

Иначе command responses быстро начинают нести на себе responsibility за реактивную синхронизацию всего UI.

### 7. E2E показывает, что control center и workspace search - это реальные product surfaces

По:

- [`control-center.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/control-center.spec.ts)
- [`workspace-search.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/workspace-search.spec.ts)

видно важное:

- control center уже выступает как unified entry point для theme/sidebar/minimap/settings
- workspace search не просто фильтрует текст, а умеет:
  - открываться с `Cmd/Ctrl+F`
  - держать focus
  - фокусировать найденные nodes
  - фильтровать results по category
  - показывать metadata around spaces/branches

Это напоминает, что terminal feature полезна не сама по себе, а внутри control surface / workspace shell.

## `Hermes IDE` - backend-authoritative orchestration вокруг phase transitions

### 1. Session phase machine already encodes product lifecycle

В [`models.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/models.rs) фазы уже явно выражают orchestration:

- `Creating`
- `Initializing`
- `ShellReady`
- `LaunchingAgent`
- `Idle`
- `Busy`
- `NeedsInput`
- `Error`
- `Disconnected`
- `Destroyed`

Это уже не просто "terminal is running".

Это полезный pattern:

- terminal feature должна моделировать orchestration phases, а не только process alive/dead

### 2. Context/launch orchestration живёт в backend PTY command layer

Самый полезный файл здесь:

- [`commands.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/commands.rs)

Из него видно, что orchestration не лежит в UI:

- phase updates приходят из analyzer/runtime
- backend сам решает, когда auto-launch agent
- backend сам решает, когда auto-inject context
- backend сам доставляет deferred nudge при `NeedsInput`

🔥 Это один из сильнейших architectural signals во всём ресёрче.

### 3. `NeedsInput` используется как безопасная точка для deferred nudge delivery

В `commands.rs` есть очень важный кусок:

- когда analyzer переводит session в `NeedsInput`
- backend вызывает `deliver_pending_nudge_with_writer(...)`

Это сильно лучше, чем слать nudge "при первом удобном UI моменте".

Полезный вывод:

- app-originated guidance лучше доставлять в явных runtime phases, а не в случайный момент между PTY chunks

### 4. Agent launch и context injection разделены, но оркестрируются вместе

`Hermes` не делает одну giant action "launch agent with all magic".

По коду видно 3 отдельных сценария:

1. shell reaches `ShellReady`
2. backend auto-launches agent if `pending_ai_launch`
3. context can be:
   - baked into CLI launch prompt for some providers
   - injected later on prompt detection
   - skipped / marked differently for SSH

Это очень зрелый shape.

### 5. Если provider умеет CLI prompt injection, `Hermes` предпочитает её PTY timing hacks

В `commands.rs` есть важный tradeoff:

- для `claude` и `gemini` context instruction может быть добавлена прямо в launch command
- тогда `context_injected = true` ставится сразу
- phase переводится в `LaunchingAgent`

Это хороший pattern:

- если provider даёт надёжный launch-time seam, лучше использовать его вместо поздней PTY injection

### 6. Fallback context injection всё равно phase-gated

Если CLI prompt injection недоступна:

- `pending_context_inject` выставляется analyzer/runtime logic
- backend ждёт suitable prompt boundary
- потом пишет instruction через writer
- `context_injected` ставится только если write действительно удался

И если write не удался:

- flag cleared
- `context_injected` остаётся false
- следующий prompt detection может повторить попытку

Это очень здоровый retry model:

- success/failure фиксируются по реальному effect, а не по intent

### 7. SSH - это не просто ещё один session type, а отдельная policy boundary

В том же orchestration path `Hermes` не притворяется, что SSH behaves like local PTY:

- если session SSH-based, `$HERMES_CONTEXT` remote-side недоступен как local path
- поэтому `pending_context_inject` обрабатывается отдельной веткой

Это очень полезный lesson:

- context/app shell behavior должно зависеть от runtime mode, а не только от session role

### 8. Analyzer-side silence fallback встроен в orchestration loop

`commands.rs` связывает analyzer silence detection и phase transitions:

- если output замолчал
- analyzer может перевести session в `ShellReady`, `Idle` или `NeedsInput`
- потом orchestration loop уже решает launch/injection/delivery actions

То есть state interpretation и command orchestration у них связаны, но не слиты в один giant method.

### 9. Cheap phase emits и expensive metrics emits разведены

Очень полезный pattern в `commands.rs`:

- при phase changes они делают cheap `SessionUpdate`
- `to_metrics()` не клонируют на каждый сигнал
- heavy metrics emit throttled примерно раз в 5 секунд

Это сильный operational insight:

- semantic richness не должна автоматически означать expensive full-state projection on every event

### 10. Tests подтверждают backend-authoritative model

Даже без полного backend test suite полезны:

- [`context-injection.test.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-injection.test.ts)
- [`context-injection-sync.test.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-injection-sync.test.ts)

Главный сигнал из этих тестов:

- context lifecycle рассматривается как backend-authoritative model
- execution mode propagation и version state transitions считаются отдельной ответственностью
- session state и context state не должны drift-ить друг относительно друга

## Что из этого стоит утащить

### Из `OpenCove`

- typed command/query control surface
- capability negotiation для clients
- separate sync event channel
- execution context as first-class launch payload
- explicit topology subsystem для local/remote/mount routing
- control-center/search surfaces как часть product shell, а не как случайные вспомогательные панели

### Из `Hermes`

- backend-authoritative phase orchestration
- deferred nudge delivery only at safe phases like `NeedsInput`
- transport-aware context injection strategy
- prefer launch-time provider seams over fragile PTY injection when possible
- cheap phase projection vs throttled heavy metrics projection

## Главный общий вывод

🔥 Один из самых важных выводов после этого прохода:

- terminal feature рано или поздно потребует **control surface**
- и отдельно потребует **orchestration layer**

Если этого не сделать явно, дальше появится смесь из:

- preload methods
- UI-only retries
- magic session ids
- hidden remote/local branching
- context injection side effects в неправильных местах

Именно это обычно и превращает сильный terminal runtime в хрупкий product shell.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [OpenCove - controlSurface.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/controlSurface.ts)
- [OpenCove - sessionStreamingHandlers.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/handlers/sessionStreamingHandlers.ts)
- [OpenCove - topologyHandlers.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/handlers/topologyHandlers.ts)
- [OpenCove - syncSse.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/http/syncSse.ts)
- [OpenCove - control center e2e](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/control-center.spec.ts)
- [OpenCove - workspace search e2e](https://github.com/DeadWaveWave/opencove/blob/main/tests/e2e/workspace-search.spec.ts)
- [OpenCove - control surface DTO](https://github.com/DeadWaveWave/opencove/blob/main/src/shared/contracts/dto/controlSurface.ts)
- [Hermes IDE - commands.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/commands.rs)
- [Hermes IDE - models.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/models.rs)
- [Hermes IDE - context-injection.test.ts](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-injection.test.ts)
- [Hermes IDE - context-injection-sync.test.ts](https://github.com/hermes-hq/hermes-ide/blob/main/src/__tests__/context-injection-sync.test.ts)
