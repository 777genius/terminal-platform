# Deep Dive: Recovery Truth Ownership And Terminal Intelligence

**Проверено**: 2026-04-19

Этот файл объединяет два очень полезных, но разных слоя:

- `OpenCove` - как не сломать recovery, если у terminal/agent/workspace много owner-ов
- `Hermes IDE` - как строить terminal intelligence так, чтобы она не мешала shell и TUI

Вместе они дают сильную картину:

- recovery нужно строить вокруг owner-specific durable truth
- intelligence нужно строить вокруг runtime gating, а не around optimistic UI hacks

## 1. `OpenCove` - recovery model у них уже мыслит owner tables, а не "перезапустим и посмотрим"

### Какие материалы особенно полезны

- `docs/RECOVERY_MODEL.md`
- `docs/PERSISTENCE.md`
- `src/contexts/workspace/presentation/renderer/components/terminalNode/hydrationRouter.ts`
- `tests/unit/terminalNode/hydrationRouter.spec.ts`
- `tests/unit/terminalNode/hydrationReplacement.spec.ts`
- `tests/unit/shared/mergePersistedAppStates.spec.ts`

### Что стало понятнее

#### A. Recovery truth is explicitly split by owner

`RECOVERY_MODEL.md` у OpenCove очень силён тем, что не говорит "сохраним всё в state".

Он разводит:

- user intent
- durable fact
- runtime observation
- UI projection

И дальше прямо задаёт owner table:

- workspace owns layout / viewport
- task owns task fields and task-agent link
- agent owns launch intent and resume binding
- terminal owns session lifecycle and scrollback

🔥 Это, возможно, один из самых ценных найденных architectural patterns вообще.

Если потом сделать terminal feature в вашем репо без owner tables, recovery почти неизбежно станет гаданием по runtime сигналам.

#### B. Watchers are not allowed to overwrite durable truth

Из `RECOVERY_MODEL.md` следует очень жёсткое правило:

- watcher может сообщать observation
- но watcher не должен сам по себе:
  - сбрасывать verified binding
  - переводить resumable truth в unavailable
  - переписывать task/agent ownership

Это полезно не только для agent windows.

Для terminal feature это прямо переносится на:

- PTY alive/exited
- late async metadata
- remote attach success/failure
- transient lookup misses

#### C. Hydration router protects recovered UI from destructive redraw noise

`hydrationRouter.ts` и связанные тесты оказались очень показательными.

После hydration у них есть отдельные сценарии:

- placeholder можно заменить сразу, если buffered output уже visibly replaceable
- destructive redraw chunks могут быть отложены
- control-only chunks не должны сразу уничтожать восстановленный baseline
- visible follow-up output триггерит реальное replacement

Из тестов особенно важно:

- `\u001b[2J\u001b[H` сам по себе не считается "готово, можно снести placeholder"
- `\u001b[2J\u001b[Hready` уже считается meaningful
- buffered exit code сам по себе может быть replacement-worthy

🔥 Это сильный practical rule:

recovery UI нельзя обновлять по первым попавшимся control sequences.

#### D. Deferred redraw is aware of real user interaction

`hydrationRouter.ts` ещё делает важную вещь:

- отложенные redraw control chunks могут копиться
- но если был recent user interaction, они сбрасываются немедленно

Это хороший баланс:

- не ломать recovered baseline на cold start
- но и не держать stale восстановленный экран, когда пользователь уже начал реально работать

#### E. Persistence merge is snapshot-aware, not naive last-write-wins

`mergePersistedAppStates.spec.ts` особенно полезен не implementation detail-ами, а shape merge semantics.

Из тестов видно, что merge:

- использует base snapshot как reference point
- умеет сохранять rect/link из base, если local его не менял
- умеет сохранять local change, если base его не менял
- корректно обрабатывает deletions

Это уже похоже на маленький owner-aware three-way merge.

⚠️ Это важный вывод для terminal feature:

durable recovery state не всегда надо просто "overwrite latest copy".  
Иногда нужен snapshot-aware merge semantics.

### Что утащить как идею

- owner table for recovery truth
- watcher/adapter observations must not directly rewrite durable recovery state
- control-only redraw chunks should not instantly replace recovered baseline
- defer destructive redraw until meaningful follow-up output
- allow real user interaction to short-circuit deferred redraw buffering
- snapshot-aware merge for persisted terminal/workspace state

---

## 2. `Hermes IDE` - intelligence строится как gated runtime, а не как always-on overlay

### Какие исходники особенно полезны

- `src/terminal/TerminalPool.ts`
- `src/terminal/ghostText.ts`
- `src/terminal/intelligence/suggestionEngine.ts`
- `src/terminal/intelligence/historyProvider.ts`
- `src/terminal/intelligence/contextAnalyzer.ts`
- `src/terminal/intelligence/shellEnvironment.ts`
- `src/terminal/intelligence/commandIndex.ts`
- `src/terminal/intelligence/SuggestionOverlay.tsx`

### Что стало понятнее

#### A. Suggestion engine itself is intentionally small and synchronous

`suggestionEngine.ts` довольно дисциплинированный:

- sync compute target `<5ms`
- dedupe across sources
- scoring combines:
  - history recency/frequency
  - static index
  - context relevance
  - exact prefix bonus
  - length penalty

Это хороший shape:

- intelligence core should stay cheap
- heavy async work needs to happen outside the hot scoring path

#### B. History provider merges shell history with session history

`historyProvider.ts` делает useful blend:

- shell history file
- session commands from execution log DB
- unified recency list
- frequency map

Это значит, что suggestions у них не purely global и не purely per-session.

Они используют оба горизонта:

- long-term shell memory
- local session memory

#### C. Context relevance is cached per cwd and failures are not sticky

`contextAnalyzer.ts` даёт очень хороший маленький pattern:

- project context cache keyed by cwd
- explicit invalidate on cwd change
- failure fallback is **not cached**

🔥 Это surprisingly important.

Если intelligence context fallback закешировать навсегда, временный backend failure quietly убьёт project-aware suggestions на весь session lifetime.

#### D. Shell environment is part of the suggestion policy

`shellEnvironment.ts` - один из самых полезных найденных intelligence files.

Там suggestions зависят не только от config, но и от:

- shell type
- native autosuggest presence
- whether shell integration successfully disabled conflicts
- augment vs replace mode

Из этого следуют сильные rules:

- ghost text не надо всегда показывать
- Tab не надо всегда перехватывать
- shell integration changes the safety envelope for smart UX

Это очень практичный product lesson.

#### E. TerminalPool gates suggestions on multiple runtime conditions at once

`TerminalPool.ts` особенно полезен тем, что overlay/suggestion UX у них привязан не к одному флагу, а к нескольким runtime guards:

- intelligence not disabled
- session phase is effectively prompt-like
- `lastStablePhase` is `idle` or `shell_ready`
- alt-screen is not active
- user is not scrolled up
- shell is foreground process
- overlay visibility handled separately from suggestion eligibility

🔥 Это один из лучших найденных anti-slop patterns для terminal intelligence.

Не "если есть inputBuffer, покажи suggestions", а:

- only if shell really owns the terminal
- only if prompt-like state is stable
- only if cursor position is trustworthy

#### F. Overlay lifecycle and key interception are separated from compute eligibility

В `TerminalPool.ts` есть ещё один subtle, but important rule:

- even if current phase briefly flips, visible overlay still intercepts navigation keys until dismissed
- showing logic and interception logic are not the same thing

Это помогает не сломать keyboard behavior из-за race между shell echo и overlay dismissal.

#### G. Ghost text is just a lightweight projection, not the source of truth

`ghostText.ts` рисует overlay абсолютно как projection layer:

- reads actual terminal font settings
- positions against live xterm dimensions
- can be cleared aggressively on size/theme/input changes

Это полезно потому, что:

- ghost text intentionally remains disposable
- no durable or semantic state should depend on it

#### H. Intent commands are a separate lane from normal suggestions

`TerminalPool.ts` uses colon-prefixed intent commands as a different flow:

- `:something`
- separate intent suggestions
- separate resolve flow

Это хороший pattern:

не всё умное terminal UX должно быть squeezed into one suggestion system.

### Что утащить как идею

- keep suggestion scoring synchronous and cheap
- merge shell history and session history
- context cache per cwd with explicit invalidation and non-sticky failures
- shell-aware policy for ghost text and Tab interception
- gate suggestions on stable prompt state, foreground ownership, no alt-screen, no user scroll-up
- separate overlay lifecycle from suggestion eligibility
- keep ghost text as disposable projection
- keep intent-command lane distinct from generic completion

---

## 3. Practical synthesis for our future feature

После этого deep dive картина стала ещё жёстче:

### A. Recovery and intelligence should not share the same truth layer

- recovery works off owner-specific durable facts
- intelligence works off runtime-gated projections and caches

Если это смешать, transient suggestion/runtime signals начнут ломать restore semantics.

### B. "Visible" is not the same as "meaningful"

Для recovery и overlay logic очень полезно различать:

- control-only output
- destructive redraw
- meaningful visible content
- user interaction

### C. Smart terminal UX needs multiple guards, not one boolean

Минимальный robust gating set уже выглядит примерно так:

- prompt-like stable phase
- shell owns foreground
- not alt-screen
- not scrolled away from cursor
- no conflict with shell-native suggestions unless integration disabled them

### D. Merge semantics may matter as much as session protocol

Если feature хранит:

- layout
- scrollback
- task/session links
- recovery binding

то часть state, скорее всего, потребует snapshot-aware merge semantics, а не naive replace.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
