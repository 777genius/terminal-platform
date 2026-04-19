# Deep Dive: Key Terminal Projects

**Проверено**: 2026-04-19

Этот файл не повторяет общий shortlist.  
Он нужен, чтобы понять **shape** самых интересных terminal projects:

- где у них заканчивается terminal core
- где начинается product shell
- где живут transport, persistence, search, browser/task adjacency
- какие идеи реально переносимы в нашу feature

## 1. `wterm`

### Snapshot

- Repo: [`vercel-labs/wterm`](https://github.com/vercel-labs/wterm)
- Stars: `1765`
- Pushed: `2026-04-18`
- Packages:
  - [`@wterm/core 0.1.8`](https://www.npmjs.com/package/@wterm/core)
  - [`@wterm/dom 0.1.8`](https://www.npmjs.com/package/@wterm/dom)
  - [`@wterm/react 0.1.8`](https://www.npmjs.com/package/@wterm/react)
  - [`@wterm/just-bash 0.1.8`](https://www.npmjs.com/package/@wterm/just-bash)
  - [`@wterm/markdown 0.1.8`](https://www.npmjs.com/package/@wterm/markdown)

### Архитектурная форма

`wterm` очень чётко разрезан на слои:

- `@wterm/core` - headless WASM bridge + WebSocket transport
- `@wterm/dom` - DOM renderer, input handler, orchestrator
- `@wterm/react` - thin React wrapper
- `@wterm/just-bash` - in-browser shell adapter
- `@wterm/markdown` - streaming Markdown to ANSI

Это хороший сигнал.  
Проект выглядит как **реально embeddable terminal toolkit**, а не как app, переодетый в package.

### Что у него реально сильное

- DOM rendering вместо canvas
- browser-native selection / find / accessibility / clipboard
- embedded WASM, без отдельного asset dance по умолчанию
- alternate screen, scrollback, resize, host responses
- грязные строки и `requestAnimationFrame`, то есть рендер не совсем наивный
- React-обвязка тонкая, не тащит свой state model

И ещё важная деталь из `@wterm/core` API:

- `WasmBridge` expose-ит cell-level access
- cursor state
- dirty-row checks
- pending title changes
- pending terminal responses
- bracketed paste / alt-screen state

Это значит, что `wterm` уже сейчас неплохо подходит не только для rendering, но и для higher-level app features вроде:

- search/index helpers
- title/cwd chrome
- shell integration side effects
- screen snapshotting

### Что у него важно понимать правильно

⚠️ `wterm` не даёт готовый IDE terminal product.

Он даёт:

- terminal core
- DOM renderer
- input handling
- WebSocket transport primitive

Он **не даёт**:

- PTY/session persistence
- tabs/splits
- command blocks
- shell integration layer
- remote/local runtime abstraction
- attention UX

`@wterm/just-bash` полезен как demo и adapter idea, но это не replacement for real PTY-backed terminal inside Electron.

### Что это значит для нас

Если выбирать `wterm`, то новая feature должна строиться так:

- `renderer` получает почти готовый terminal surface
- `main` и `preload` строят свой PTY/session bridge
- `core/application` строит tabs/splits/search/persistence/attention

То есть `wterm` хорош, когда приоритеты такие:

- React/Electron embedability
- native browser ergonomics
- минимальная зависимость от canvas/WebGL quirks

### Вывод

`wterm` выглядит самым чистым `embed-first` foundation.

`🎯 8   🛡️ 5   🧠 5`  
Примерно `2000-4500` строк до сильного MVP.

---

## 2. `restty`

### Snapshot

- Repo: [`wiedymi/restty`](https://github.com/wiedymi/restty)
- Stars: `339`
- Pushed: `2026-04-03`
- NPM: [`restty 0.1.35`](https://www.npmjs.com/package/restty)

### Архитектурная форма

По `docs/how-it-works.md` у `restty` очень явный pipeline:

- `src/pty/` - PTY WebSocket transport
- `src/wasm/` - WASM wrapper around `libghostty-vt`
- `src/renderer/` - `WebGPU` first, `WebGL2` fallback
- `src/surface/` - high-level `Restty` API
- `src/runtime/` - per-pane runtime

И это ключевой момент:

`restty` - это не просто renderer, а **browser terminal stack**.

### Что у него реально сильное

- `libghostty-vt` core
- `WebGPU` first rendering
- built-in panes
- themes + Ghostty theme parsing
- text shaping / ligatures
- plugin API
- shader stages
- xterm compatibility shim для migration path

Особенно важен plugin story:

- input interceptors
- output interceptors
- lifecycle hooks
- render hooks
- GPU shader stages

Это уже похоже на **terminal platform layer**, а не на просто widget.

При этом xterm compatibility у них честно ограничена:

- focused migration subset
- не full parity with xterm internals

Это хороший знак. Проект не обещает невозможного.

### Что у него важно понимать правильно

По собственным `goals.md` у `restty` есть явные non-goals:

- full terminal application UI
- native desktop runtime
- exact parity with native Ghostty renderers

То есть даже при наличии panes это всё ещё **не готовый terminal workspace product**.

Также риски:

- очень ранний lifecycle проекта
- dev/build story завязана на `bun`
- API authors прямо пишут, что high-level APIs usable now, but may still change

### Что это значит для нас

`restty` интересен, если хотим foundation с запасом под:

- terminal-specific plugins
- richer rendering pipeline
- custom shader / visual stages
- migration from xterm-like code

Но он менее "просто вставил и живёшь", чем `wterm`.

### Вывод

`restty` сильнее как terminal stack, но слабее как predictable embed foundation.

`🎯 8   🛡️ 6   🧠 6`  
Примерно `2500-5000` строк до сильного MVP.

---

## 3. `ghostty-web` + `libghostty` + `Ghostling`

### Snapshot

- [`coder/ghostty-web`](https://github.com/coder/ghostty-web) - `2341` stars, pushed `2026-04-13`, [`ghostty-web 0.4.0`](https://www.npmjs.com/package/ghostty-web)
- [`ghostty-org/ghostty`](https://github.com/ghostty-org/ghostty) - `51110` stars, pushed `2026-04-17`
- [`ghostty-org/ghostling`](https://github.com/ghostty-org/ghostling) - `952` stars, pushed `2026-04-06`

### Архитектурная форма

Здесь важно понимать три разных уровня:

1. `libghostty-vt`
   - parsing terminal sequences
   - terminal state
   - render state
   - no tabs/splits/session management

2. `ghostty-web`
   - web/WASM packaging around Ghostty parser
   - xterm-compatible API
   - migration target for browser apps

3. `Ghostling`
   - минимальный complete example на C
   - доказывает, что поверх `libghostty` реально можно быстро собрать рабочий terminal

### Что у этого стека реально сильное

- correctness и standards coverage
- strong Unicode / grapheme handling
- real Ghostty parser instead of JS reimplementation
- clear separation between emulation core and host app responsibilities

Самый важный момент из `Ghostling`:

🔥 `libghostty` **не обещает** tabs, splits, session management, config UI и search UI.

То есть direct `libghostty` path не снимает product work.  
Он только даёт сильнейший emulation core.

### Что у него важно понимать правильно

`ghostty-web` сейчас хорош как migration-friendly layer:

- `@xterm/xterm` -> `ghostty-web`
- zero runtime dependencies
- ~400KB WASM bundle

Но сам repo прямо пишет, что пока использует patch поверх Ghostty source и в будущем хочет перейти на official Ghostty WASM distribution.

А Ghostty README отдельно уточняет:

- `libghostty-vt` usable today
- functionality stable
- API signatures still in flux
- versioned release story ещё не оформлен нормально

Плюс Mitchell в `Libghostty Is Coming` отдельно пишет, что long-term план - не один monolith, а family of libs:

- `libghostty-vt`
- input handling
- GPU rendering
- GTK widgets / Swift frameworks

Это важный стратегический сигнал:

⚠️ direct `libghostty` path сегодня сложный, но со временем может стать намного проще и выше уровнем.

### Что это значит для нас

Если идти в direct `libghostty`, нужно честно принять:

- мы строим свою terminal platform layer
- нам самим нужны renderer/host bindings
- tabs/splits/persistence/attention/search всё равно наши

Если идти в `ghostty-web`, это уже не "всё сами", а "core correctness с xterm-like integration surface".

### Вывод

Это самый сильный long-term engine path, но не самый дешёвый integration path.

`🎯 7   🛡️ 5   🧠 9`  
Примерно `6000-12000` строк.

---

## 4. `cmux`

### Snapshot

- Repo: [`manaflow-ai/cmux`](https://github.com/manaflow-ai/cmux)
- Stars: `14729`
- Pushed: `2026-04-19`
- License: `GPL-3.0-or-later` or commercial

### Что в нём реально first-class

Не terminal tab.  
Не shell itself.  
А **attention + workspace metadata**.

Это видно по README:

- notification rings on panes
- unread state and notification panel
- vertical sidebar with branch, PR status, cwd, ports, latest notification
- browser pane рядом с terminal
- CLI + socket API

### Самый важный инсайт

`cmux` не делает ставку на "умный parser".  
Он делает ставку на то, что у пользователя одновременно много agent sessions, и главная проблема - **куда смотреть сейчас**.

Это очень важный product lesson.

### Важное ограничение

По README current behavior:

- layout and metadata restore are implemented
- live process state is **not** resumed after app restart

То есть `cmux` очень силён как attention shell, но пока не решает durable process persistence так, как `tmux`-based модели.

### Что брать как идею

- attention states inside layout
- unread navigation
- sidebar metadata richer than tab titles
- browser pane как adjacent surface
- OSC `9/99/777` as notification transport

### Вывод

`cmux` - лучший donor для attention UX, но не для persistence model.

`🎯 9   🛡️ 7   🧠 7`  
Примерно `3000-7000` строк, если переносить идеи.

---

## 5. `Factory Floor`

### Snapshot

- Repo: [`alltuner/factoryfloor`](https://github.com/alltuner/factoryfloor)
- Stars: `91`
- Pushed: `2026-04-16`
- License: `MIT`

### Что в нём реально first-class

Не session.  
Не terminal pane.  
А **workstream**.

Workstream в их модели это:

- git worktree
- Claude Code session
- terminal
- browser
- editor
- project hooks/env

### Самые сильные технические идеи

- `tmux` persistence on dedicated socket `factoryfloor`
- `.factoryfloor.json` with `setup`, `run`, `teardown`
- deterministic `FF_PORT`
- `FF_PROJECT`, `FF_WORKSTREAM`, `FF_WORKTREE_DIR`
- browser auto-navigation to detected dev server

Это уже не просто terminal UX.  
Это очень хорошая runtime model для parallel development.

### Что это значит для нас

Если мы хотим terminal feature useful for real coding workflows, надо думать не только о "new terminal session", а о:

- create workstream
- bind worktree
- run setup
- run dev server
- attach browser/editor
- persist agent session

### Вывод

`Factory Floor` - лучший donor для `workstream runtime object`.

`🎯 8   🛡️ 7   🧠 8`  
Примерно `3500-8000` строк, если переносить идеи.

---

## 6. `Hermes IDE`

### Snapshot

- Repo: [`hermes-hq/hermes-ide`](https://github.com/hermes-hq/hermes-ide)
- Stars: `208`
- Pushed: `2026-04-16`
- Latest release: `v0.6.15` от `2026-04-16`
- License: source-available `BSL 1.1`

### Что в нём реально first-class

`Hermes` делает terminal частью более широкой machine:

- session lifecycle
- execution timeline
- project scanning
- context injection
- git and diff workflow

### Что особенно полезно в архитектуре

Из `ARCHITECTURE.md` видно несколько очень хороших решений:

- module-level terminal pool, а не terminal instance inside every React component
- typed IPC wrappers
- backend PTY manager separate from frontend rendering
- provider adapter registry for different AI CLI tools
- explicit session phase state machine

Сильная деталь:

frontend terminal rendering живёт в `TerminalPool`, где держатся:

- xterm instance
- WebGL addon
- event listeners
- suggestion/ghost text intelligence state

Это хороший pattern, если terminal views могут remount-иться, но сам runtime не должен разваливаться.

### Что ещё важно

`Hermes` design principles жёстко anti-bloat:

- focused, not full-featured
- fast by default
- opinionated over configurable
- core vs extension

Это полезно как guardrail для нашей future feature, чтобы не превратить terminal в бесконечный kitchen sink.

### Вывод

`Hermes IDE` - сильный donor для `session state machine + terminal pool + execution timeline`.

`🎯 7   🛡️ 6   🧠 7`  
Примерно `3000-7000` строк, если переносить идеи.

---

## Общие выводы после deep dive

### 1. Лучшие foundation-проекты modular, а не magical

У сильных foundation paths есть чёткая граница:

- core/emulator
- renderer/input
- transport
- app shell

Это видно у `wterm`, `restty`, `ghostty-web/libghostty`.

### 2. `libghostty` не убирает product work

Очень важно не обманывать себя:

`libghostty` решает VT correctness и terminal state.  
Он не решает:

- persistence
- tabs/splits
- notifications
- workstreams
- browser/editor adjacency

### 3. Самые сильные donor products проектируют не terminal, а runtime object

- `cmux` - workspace attention object
- `Factory Floor` - workstream object
- `Hermes IDE` - project-aware session object

### 4. Для нашей feature это означает следующий architectural split

- foundation layer: `wterm` or `restty` or `ghostty-web`
- runtime layer: session service, persistence, attach/detach, worktree binding
- product layer: attention UX, timeline, browser/editor adjacency, search/control center

## Sources

- [wterm](https://github.com/vercel-labs/wterm)
- [restty](https://github.com/wiedymi/restty)
- [ghostty-web](https://github.com/coder/ghostty-web)
- [Ghostty](https://github.com/ghostty-org/ghostty)
- [Ghostling](https://github.com/ghostty-org/ghostling)
- [cmux](https://github.com/manaflow-ai/cmux)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [Hermes IDE Architecture](https://github.com/hermes-hq/hermes-ide/blob/main/ARCHITECTURE.md)
- [Hermes IDE Design Principles](https://github.com/hermes-hq/hermes-ide/blob/main/DESIGN_PRINCIPLES.md)
