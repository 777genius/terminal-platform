# Deep Dive - Hydration and Prompt Lifecycle

**Проверено**: 2026-04-19  
**Фокус**: `OpenCove` hydration/recovery pipeline и `Hermes IDE` prompt/intelligence lifecycle

## Почему именно это важно

У современных terminal products самые неприятные баги обычно не в renderer, а в двух местах:

- restore / hydration после remount, reconnect, workspace switch
- prompt UX поверх живого shell и TUI

Именно здесь обычно появляется архитектурная грязь:

- duplicated output
- потерянный full-screen frame
- suggestions поверх `vim` / `less`
- сломанные shell queries
- race между resize, attach, replay и prompt overlays

`OpenCove` и `Hermes IDE` интересны тем, что оба проекта уже явно признают эти проблемы и решают их отдельными runtime-policy слоями.

## `OpenCove` - hydration как pipeline, а не один restore call

### 1. У restore есть три разных истины

По коду видно, что `OpenCove` не сводит restore к одному "снимку терминала":

- `persistedSnapshot` - durable scrollback baseline
- `cachedSerializedScreen` - committed screen state через xterm SerializeAddon
- live PTY snapshot / buffered output - runtime continuation

Это видно в:

- [`hydrateFromSnapshot.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrateFromSnapshot.ts)
- [`committedScreenState.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/committedScreenState.ts)
- [`screenStateCache.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/screenStateCache.ts)

Самый ценный вывод:

- committed screen state и raw PTY tail нельзя считать одной и той же истиной

Это особенно критично для `alt-screen` и full-screen TUI.

### 2. Unmount cache специально не равен "сериализуй всё что видишь"

В [`cacheTerminalScreenState.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/cacheTerminalScreenState.ts) на unmount:

- cache вообще не пишется, если terminal invalidated или ещё не hydrated
- сначала берётся `latestCommittedScreenState`
- serialize fallback разрешён только если `!hasPendingWrites`

Это очень сильный practical rule:

- если есть pending writes, не надо притворяться, что serialize прямо сейчас точно отражает последний стабильный экран

И ещё полезно, что cache хранится по `nodeId`, но с проверкой `sessionId` и отдельной invalidation map.  
То есть stale cache не просто перетирается last-write-wins логикой.

### 3. Alt-screen restore требует отдельной политики replay

В [`hydrateFromSnapshot.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrateFromSnapshot.ts) есть очень важное правило:

- если восстановили `cachedSerializedScreen`, а он находится в alternate buffer
- и live delta не содержит явного `ESC[?1049l`
- raw delta вообще не надо replay-ить поверх committed screen

Иначе prompt/redraw output, который пришёл пока pane был detached, легко сотрёт последний user-visible full-screen frame.

Это отдельно подтверждается в:

- [`docs/TERMINAL_ANSI_SCREEN_PERSISTENCE.md`](https://github.com/DeadWaveWave/opencove/blob/main/docs/TERMINAL_ANSI_SCREEN_PERSISTENCE.md)

Главный вывод:

- full-screen restore нельзя проектировать как "всегда replay tail bytes поверх snapshot"

### 4. Agent terminals иногда лучше не блокировать на live snapshot

В том же `hydrateFromSnapshot.ts` есть ещё один взрослый tradeoff:

- для `kind === 'agent'` hydration по умолчанию не ждёт live PTY snapshot
- attach не блокируется целиком

Причина очень практичная:

- некоторые CLI сразу после attach шлют terminal feature probes
- если reply задержать, CLI может отключить color
- или можно увидеть echoed escape sequences после выхода из raw/noecho mode

Это хороший reminder:

- hydration latency меняет terminal semantics, а не только perceived UX

### 5. `hydrationRouter` - это policy engine, а не просто buffer

В [`hydrationRouter.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrationRouter.ts) и tests видно несколько сильных правил:

- placeholder replacement живёт отдельно от hydrated redraw deferral
- automatic terminal queries не должны застревать в deferral и идут сразу в output scheduler
- destructive redraw chunks можно временно копить отдельно
- real user interaction может форсированно выпустить deferred redraw

Подтверждается тестами:

- [`hydrationRouter.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/unit/terminalNode/hydrationRouter.spec.ts)

Особенно ценно вот что:

- control-only redraw не считается самодостаточным сигналом "экран можно заменить"

### 6. Replacement policy основана на содержимом, а не на regex-магии

В [`hydrationReplacement.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrationReplacement.ts) логика пытается понять:

- есть ли destructive control sequences
- есть ли после stripping control codes реально meaningful visible content
- оправдывает ли `exitCode` replacement

Тесты это подтверждают:

- [`hydrationReplacement.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/unit/terminalNode/hydrationReplacement.spec.ts)

Очень полезный pattern:

- classifier должен различать control-only chunk и chunk, который реально меняет user-visible truth

### 7. `finalizeHydration` очень аккуратен в порядке операций

В [`finalizeHydration.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/finalizeHydration.ts) заметны правильные ordering rules:

- считается overlap между baseline snapshot и buffered output
- при baseline replacement terminal reset делается явно
- перед replay buffered output сначала flush-ится `ptyWriteQueue`
- потом идёт replay buffered output
- reveal терминала происходит только после size sync

А [`revealHydratedTerminal.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/revealHydratedTerminal.ts) делает это через double `requestAnimationFrame`.

Итоговый practical rule:

- hydration correctness часто зависит именно от order of operations, не от выбора terminal renderer

### 8. Output scheduler знает про viewport interaction

[`outputScheduler.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/outputScheduler.ts) показывает зрелый shape output policy:

- direct writes отдельно считаются in-flight
- позже пришедшие chunks queue-ятся, пока direct write не commit-нулся
- во время viewport interaction budget снижается
- есть отдельный flush timer для interaction mode
- есть `maxPendingChars` guard

И tests явно проверяют pending semantics:

- [`terminalNode.output-scheduler.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/unit/contexts/terminalNode.output-scheduler.spec.ts)

Это хороший middle-ground:

- не стопорить scrollback/streaming полностью
- но и не thrash-ить renderer, пока пользователь скроллит

### 9. Durable scrollback mirror живёт отдельно от hot path

В [`scrollbackSchedule.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/utils/persistence/scrollbackSchedule.ts):

- scrollback write дебаунсится
- in-flight write не дублируется
- если во время flush пришёл новый state, ставится `flushRequested`
- финальный flush решает, писать ли ещё раз

Это маленький, но очень полезный pattern:

- persistence scheduling должен быть отдельным adapter layer, а не побочным эффектом каждого PTY chunk

## `Hermes IDE` - prompt UX как lifecycle policy

### 1. Pool отделён от UI component lifecycle

`Hermes` держит terminal runtime в двух слоях:

- [`pool.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/terminal/pool.ts) - lifecycle, attach/detach, resize, focus, shell polling
- [`TerminalPool.ts`](https://github.com/hermes-hq/hermes-ide/blob/main/src/terminal/TerminalPool.ts) - input handling, suggestions, ghost text, intent commands

Это сильный design choice:

- pane remount не должен быть владельцем terminal truth

### 2. PTY и xterm стартуют с согласованными размерами

В `pool.ts` `estimateInitialDimensions()` используется не просто для красоты.

Они явно создают xterm с теми же initial rows/cols, что и backend PTY.  
Причина хорошо объяснена в комментарии:

- если PTY стартует примерно на 160 cols, а xterm buffer стартует на 80
- shell уже форматирует output под широкий PTY
- позже никакой resize/reflow уже не "починит" испорченный cursor positioning и wrapping

Это один из самых полезных low-level insights из всего deep dive.

### 3. Attach pipeline защищает от layout races

В `pool.ts` attach сделан аккуратно:

- terminal re-parent-ится, если нужно
- shell foreground polling стартует только для focused session
- fit делается через double `requestAnimationFrame`
- перед `fit()` проверяются proposed dimensions, чтобы не уронить buffer до мусорных `1xN`
- после fit идёт `resizeSession(...)`

То есть resize policy уже осознаёт:

- layout timing
- xterm irreversible buffer resize
- shell width mismatch

### 4. `sessionPhase` и `lastStablePhase` важнее, чем просто focused state

В `pool.ts` и `TerminalPool.ts` suggestions и overlays не завязаны только на focus.

Есть разделение:

- `sessionPhase`
- `lastStablePhase`

`busy` считается transient echo-flicker и не должен постоянно ломать prompt UX.  
Поэтому suggestion gating опирается на `lastStablePhase`, а не на raw текущую фазу.

Это очень сильный reusable pattern.

### 5. Suggestion UX gated несколькими guard-ами сразу

В `TerminalPool.ts` `computeSuggestions()` не показывает overlay, если:

- intelligence disabled
- overlay policy не разрешает показ
- `lastStablePhase` не `idle` и не `shell_ready`
- active buffer это `alternate`
- пользователь scrolled up
- shell не владеет foreground process group

И отдельно:

- если overlay уже visible, navigation keys продолжают интерсептиться даже если phase кратко мигнул в `busy`

Это правильный компромисс:

- gating для показа и gating для уже видимого overlay не должны быть одной и той же логикой

### 6. Foreground-process check сделан как отдельный runtime signal

В `pool.ts` есть polling `isShellForeground(sessionId)` каждые `300ms`, но только для focused terminal.

Это полезно по двум причинам:

- prompt overlays не верят только cursor position и local input buffer
- expensive OS-level check не делается для всех session одновременно

Именно этот guard делает prompt UX заметно честнее рядом с agent CLIs и full-screen tools.

### 7. Resize re-send на `shell_ready`

Когда `setSessionPhase()` видит переход в `shell_ready`, `Hermes` ещё раз шлёт `resizeSession(...)`.

Причина взрослая:

- attach мог отправить resize слишком рано
- shell мог ещё не установить `SIGWINCH` handler
- первый resize мог потеряться

Это кажется мелочью, но как раз такие детали сильно влияют на "terminal feels correct".

### 8. Shell integration не ломает user config, а оборачивает её

[`shell_integration.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/shell_integration.rs) очень полезен именно подходом:

- сначала запускается обычный user shell config
- потом уже накладываются Hermes overrides

Что они реально делают:

- `zsh` - временный `ZDOTDIR` с прокси-файлами
- `bash` - `--rcfile`
- `fish` - `-C`
- отключают конфликтующие autosuggestion systems
- экспортируют `HERMES_TERMINAL=1`
- для zsh включают `HIST_IGNORE_SPACE`
- форсят `WINCH`, чтобы shell перечитал размеры

Главный урок:

- shell integration это не просто OSC markers
- это ещё и conflict policy с чужими plugins и shell startup order

### 9. Copy и input quality treated as product work, не как мелочь

В `pool.ts` ещё видны полезные детали:

- capture-phase `Ctrl+C`, потому что хост может перехватить shortcut раньше xterm
- `Shift+Enter` отправляет `CSI u` sequence
- `cleanSelection()` склеивает и soft wraps, и program wraps
- copied text чистится через buffer metadata, а не обычным `trim()`

Это напоминает важную вещь:

- удобный IDE-like terminal состоит из десятков маленьких correctness fixes

## Что из этого reusable, а что host-specific

### Реально reusable patterns

- separate committed screen state from raw PTY continuation
- overlap-aware replay during hydration
- explicit destructive-redraw policy
- focused-session foreground polling
- `lastStablePhase` для prompt UX
- re-send resize on `shell_ready`
- viewport-aware output scheduling
- terminal pool above UI components

### Скорее host-specific adaptation

- WKWebView/macOS dead-key handling из `Hermes`
- конкретные Tauri native shortcut workarounds
- OpenCove-specific spatial workspace plumbing

Это важно не перепутать.  
Надо забирать design pattern, а не слепо копировать platform hack.

## Что особенно стоит утащить в нашу feature

1. Hydration policy как отдельный runtime layer, а не побочный эффект mount.
2. Committed screen snapshot для alt-screen restore.
3. Overlap-aware replay между durable snapshot и live PTY continuation.
4. Destructive redraw classifier с visible-content semantics.
5. Viewport-aware output scheduler с explicit in-flight tracking.
6. Terminal pool выше renderer panes.
7. `sessionPhase + lastStablePhase + shellIsForeground` как базовый prompt lifecycle contract.
8. Shell integration как отдельный subsystem с conflict policy.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [OpenCove - TERMINAL_ANSI_SCREEN_PERSISTENCE.md](https://github.com/DeadWaveWave/opencove/blob/main/docs/TERMINAL_ANSI_SCREEN_PERSISTENCE.md)
- [OpenCove - hydrateFromSnapshot.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrateFromSnapshot.ts)
- [OpenCove - hydrationRouter.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/hydrationRouter.ts)
- [OpenCove - finalizeHydration.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/finalizeHydration.ts)
- [OpenCove - outputScheduler.ts](https://github.com/DeadWaveWave/opencove/blob/main/src/contexts/workspace/presentation/renderer/components/terminalNode/outputScheduler.ts)
- [Hermes IDE - pool.ts](https://github.com/hermes-hq/hermes-ide/blob/main/src/terminal/pool.ts)
- [Hermes IDE - TerminalPool.ts](https://github.com/hermes-hq/hermes-ide/blob/main/src/terminal/TerminalPool.ts)
- [Hermes IDE - shell_integration.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/shell_integration.rs)
