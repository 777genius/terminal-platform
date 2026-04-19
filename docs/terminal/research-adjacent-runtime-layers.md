# Research: Adjacent Runtime Layers And Terminal-Native UX

**Проверено**: 2026-04-19

Этот файл про вещи, которые не являются лучшим terminal foundation сами по себе, но могут сильно усилить terminal feature:

- runtime wrappers
- worktree/agent orchestration shells
- terminal-native UI protocols
- parser/viewer sidecars

## Почему этот слой важен

После основного ресёрча стало видно:

⚠️ Искать ещё один "магический terminal renderer" уже почти бессмысленно.

Гораздо больше value дают adjacent-слои:

- orchestration shell
- worktree isolation model
- terminal-native structured UI
- autocomplete / prompt metadata
- headless parser / replay / viewer pieces

## 1. `Mux`

### Snapshot

- Repo: [`coder/mux`](https://github.com/coder/mux)
- Stars: `1657`
- Pushed: `2026-04-18`

### Что это такое

`Mux` - desktop + browser application для isolated parallel agentic development.

Что особенно важно:

- local / worktree / SSH runtimes
- central view on git divergence
- desktop и browser surfaces одновременно
- multi-model story
- responsive UI даже для mobile server mode

### Почему он интересен нам

`Mux` показывает очень сильный паттерн:

terminal feature может быть частью не "IDE terminal panel", а **runtime control plane** для workspaces.

Особенно полезные идеи:

- unified runtime abstraction: local / worktree / SSH
- git divergence как first-class surface
- agent status в sidebar
- central orchestration UX, а не просто терминальные окна

### Где применимо в нашей feature

- session/workspace abstraction
- remote/local/worktree runtime model
- terminal metadata tied to git state

### Оценка как donor

`🎯 8   🛡️ 7   🧠 7`  
Примерно `4000-9000` строк, если переносить идеи.

---

## 2. `Supacode`

### Snapshot

- Repo: [`supabitapp/supacode`](https://github.com/supabitapp/supacode)
- Stars: `854`
- Pushed: `2026-04-18`

### Что это такое

`Supacode` - native macOS command center для coding agents, построенный на `libghostty`.

Из того, что подтверждено публично:

- native macOS app
- `libghostty`
- worktree coding agents command center
- фокус на массовом parallel-agent workflow

### Почему он интересен

Даже минималистичный README уже показывает сильную product direction:

- terminal performance важна
- orchestration layer должна быть native-fast
- worktree isolation - это не bonus, а основа multi-agent работы

### Что нужно брать как идею

- command center mindset
- BYOA / bring-your-own-agents pattern
- worktree isolation as default

### Почему это не foundation

- не library
- больше orchestration app, чем reusable SDK

### Оценка как donor

`🎯 7   🛡️ 6   🧠 7`  
Примерно `3000-8000` строк, если переносить идеи.

---

## 3. `Ghostree`

### Snapshot

- Repo: [`sidequery/ghostree`](https://github.com/sidequery/ghostree)
- Stars: `138`
- Pushed: `2026-04-15`

### Что это такое

`Ghostree` - fork of Ghostty с native worktrees и vertical tabs для agent workflows.

Сильная мысль проекта:

- не обязательно строить отдельный orchestration shell
- можно глубоко адаптировать сам terminal app под worktree/agent usage

### Что здесь важно

`Ghostree` интересен как proof-of-demand для таких вещей:

- vertical tabs
- built-in worktree awareness
- agent-friendly terminal chrome

### Что брать как идею

- worktree awareness directly in terminal surface
- agent-focused sidebar/tab metaphors

### Где граница

Для нашего Electron app это не foundation, а референс того, какие terminal-native affordances пользователям реально нужны.

### Оценка как donor

`🎯 6   🛡️ 5   🧠 6`  
Примерно `2000-5000` строк.

---

## 4. `Trolley`

### Snapshot

- Repo: [`weedonandscott/trolley`](https://github.com/weedonandscott/trolley)
- Stars: `274`
- Pushed: `2026-04-04`

### Что это такое

`Trolley` - runtime для доставки TUI apps конечным пользователям, powered by `libghostty`.

Главная идея:

- bundle terminal app + runtime + assets
- дать non-technical user нормальное desktop-like experience у TUI приложения

### Почему это интересно нам

Это не наш основной use case, но проект очень полезен как архитектурный сигнал:

- `libghostty` уже пригоден не только для terminal emulators, но и для runtime packaging
- terminal runtime можно стандартизировать и параметризовать через manifest

### Что брать как идею

- manifest-driven terminal runtime settings
- packaging mentality вокруг terminal apps
- platform wrappers вокруг terminal core

### Важный technical insight

`Trolley` прямо описывает platform runtime split:

- macOS: Swift/AppKit + Metal
- Linux: Zig + GLFW + OpenGL
- Windows: Zig + Win32 + OpenGL

Это полезный signal о том, как люди уже operationalize `libghostty`.

### Оценка как adjacent runtime donor

`🎯 6   🛡️ 6   🧠 7`  
Примерно `2000-4000` строк.

---

## 5. `ghostty-opentui`

### Snapshot

- Repo: [`remorses/ghostty-opentui`](https://github.com/remorses/ghostty-opentui)
- Stars: `56`
- Pushed: `2026-04-11`

### Что это такое

`ghostty-opentui` - parser/viewer bridge:

- parses ANSI/VT via Ghostty
- exports JSON
- strips ANSI to text
- can render in OpenTUI
- has Node N-API addon story

### Почему это интересно

Это очень полезный "middle layer" проект.

Не renderer для end-user terminal feature, а инструмент для:

- structured parsing terminal logs
- converting terminal output into renderable/viewable data
- text extraction for LLMs or indexing

### Что брать как идею

- structured terminal snapshot representation
- ANSI-to-text not via regex, а через real emulator
- parser sidecar for search/index/AI features

### Где применимо у нас

- terminal output indexing
- search/control center
- AI summaries over terminal state
- test fixtures / snapshotting

### Оценка как sidecar donor

`🎯 7   🛡️ 6   🧠 5`  
Примерно `1500-3500` строк.

---

## 6. `termprompt`

### Snapshot

- Repo: [`seeden/termprompt`](https://github.com/seeden/termprompt)
- Stars: `2`
- Pushed: `2026-03-09`
- NPM: [`termprompt 0.2.3`](https://www.npmjs.com/package/termprompt), опубликован `2026-03-09`

### Что это такое

`termprompt` - prompt library, которая вместе с обычным TUI эмитит structured `OSC 7770` sequences.

Это очень важная идея:

- обычный terminal показывает fallback TUI
- smart terminal перехватывает структурированные события и рисует native UI

### Почему это интересно

Это один из лучших найденных мостов между:

- terminal compatibility
- richer native UI

Без необходимости ломать старый terminal world.

### Что брать как идею

🔥 Smart-terminal protocol layer.

Наша future terminal feature потенциально может:

- слушать структурированные OSC payloads
- рендерить richer native controls
- при этом оставаться совместимой с обычным terminal fallback

### Где риск

- проект очень ранний
- adoption почти нулевой

### Оценка как protocol idea

`🎯 7   🛡️ 4   🧠 4`  
Примерно `800-2000` строк.

---

## 7. `ghost-complete`

### Snapshot

- Repo: [`StanMarek/ghost-complete`](https://github.com/StanMarek/ghost-complete)
- Stars: `128`
- Pushed: `2026-04-18`
- Latest release: `v0.8.2` от `2026-04-18`

### Что это такое

`ghost-complete` - terminal-native autocomplete engine через PTY proxy.

Особенно важные моменты:

- no accessibility overlay hacks
- suggestions renderятся как ANSI popup
- uses terminal capability profiling
- uses OSC 133 prompt detection
- ships with `709` Fig-compatible specs

### Почему это интересно

Autocomplete такого уровня - это одна из немногих вещей, которая реально делает terminal "не обычным".

### Что брать как идею

- PTY-proxy augmentation layer
- prompt detection через OSC 133
- terminal-native overlay instead of separate UI overlay
- reuse of Fig completion specs / spec ecosystem

### Где риск

- macOS only
- pre-1.0
- не foundation, а augmentation layer

### Оценка как UX donor

`🎯 7   🛡️ 6   🧠 8`  
Примерно `3000-7000` строк.

---

## 8. `Muxy`

### Snapshot

- Repo: [`muxy-app/muxy`](https://github.com/muxy-app/muxy)
- Stars: `202`
- Pushed: `2026-04-19`

### Что это такое

`Muxy` - lightweight native macOS terminal на `SwiftUI + libghostty`.

Особенно интересные функции:

- project-based workflow
- vertical tabs
- split panes
- workspace persistence per project
- in-terminal search
- built-in basic git/VCS actions
- mobile iOS companion for testing

### Почему это интересно

`Muxy` - один из самых полезных референсов на тему:

"как сделать terminal feel modern и useful без превращения продукта в большой orchestration suite"

### Что брать как идею

- project-scoped terminal state
- search inside terminal as baseline, а не как optional addon
- lightweight native VCS helpers рядом с terminal
- mobile companion as long-term extension idea

### Оценка как donor

`🎯 7   🛡️ 6   🧠 5`  
Примерно `2000-5000` строк.

---

## 9. `webterm`

### Snapshot

- Repo: [`rcarmo/webterm`](https://github.com/rcarmo/webterm)
- Stars: `106`
- Pushed: `2026-03-01`

### Что это такое

`webterm` - browser terminal server с dashboard mode и live-updating tiles.

Что особенно ценно:

- reconnect support
- dashboard of multiple sessions
- live PNG/SVG screenshots of sessions
- SSE activity updates
- mobile/touch support
- theme/font controls
- uses Ghostty WebAssembly terminal engine

### Почему это интересно

Это один из лучших найденных референсов для:

- web/remote terminal dashboard
- multi-session monitoring
- preview tiles instead of plain tab labels

### Что брать как идею

- session thumbnails / previews
- dashboard mode for many long-running sessions
- touch/mobile terminal control affordances

### Оценка как donor

`🎯 7   🛡️ 6   🧠 6`  
Примерно `2500-6000` строк.

---

## 10. `Open Terminal`

### Snapshot

- Repo: [`open-webui/open-terminal`](https://github.com/open-webui/open-terminal)
- Stars: `2330`
- Pushed: `2026-04-17`

### Что это такое

`Open Terminal` - remote shell + file management backend через REST API.

Поддерживает:

- Docker sandbox mode
- bare metal mode
- file browsing/upload/download/edit
- multi-user options
- configurable environment/bootstrap

### Почему это интересно

Это не terminal renderer, а **terminal backend service**.  
Очень полезно как альтернативный architectural path, если terminal feature когда-то пойдёт в сторону:

- remote execution
- sandboxed agent runtime
- bring-your-own terminal backend

### Что брать как идею

- backend terminal as a service
- file management рядом с terminal API
- sandbox/bare-metal dual mode

### Оценка как backend donor

`🎯 7   🛡️ 7   🧠 6`  
Примерно `2000-5000` строк.

---

## 11. `Obsidian Ghostty Terminal`

### Snapshot

- Repo: [`lavs9/obsidian-ghostty-terminal`](https://github.com/lavs9/obsidian-ghostty-terminal)
- Stars: `5`
- Pushed: `2026-03-16`

### Почему это очень полезно

Это один из лучших proof-of-embedability для `ghostty-web` внутри Electron-like plugin environment.

Самое важное:

- использует `ghostty-web`
- вместо `node-pty` использует Python PTY proxy через stdlib `pty`
- multi-split support
- file-explorer context menu
- auto-reads user Ghostty config

### Что это нам даёт

🔥 Очень полезный integration pattern:

если `node-pty` окажется слишком хрупким в каком-то окружении, можно рассматривать sidecar PTY proxy path.

### Что брать как идею

- terminal here from file explorer
- fallback PTY proxy architecture
- config import from user's existing terminal setup

### Оценка как integration donor

`🎯 7   🛡️ 5   🧠 6`  
Примерно `1500-4000` строк.

---

## 12. `BooTTY`

### Snapshot

- Repo: [`0xBigBoss/vscode-bootty`](https://github.com/0xBigBoss/vscode-bootty)
- Stars: `14`
- Pushed: `2026-02-28`

### Что это такое

`BooTTY` - VS Code terminal extension powered by `libghostty-vt` via WebAssembly.

Поддерживает:

- panel/editor terminals
- multi-tab support
- theme integration
- file path detection
- notifications from terminal apps via `OSC 9`

### Почему это интересно

Это очень полезный proof, что `libghostty` stack уже можно встраивать в IDE-like host, а не только в standalone terminal app.

### Что брать как идею

- panel vs editor terminal modes
- file path click detection
- `OSC 9` notifications as integration surface

### Оценка как proof-of-embedability donor

`🎯 7   🛡️ 5   🧠 5`  
Примерно `1500-3500` строк.

---

## 13. `ht`

### Snapshot

- Repo: [`andyk/ht`](https://github.com/andyk/ht)
- Stars: `892`
- Pushed: `2025-07-25`

### Что это такое

`ht` - headless terminal wrapper around any binary, with JSON over stdio and WebSocket APIs.

### Почему это интересно

Это не renderer, а очень сильный pattern для:

- programmatic terminal access
- machine-readable terminal sessions
- automation and orchestration

### Что брать как идею

- terminal session API as structured protocol
- wrap-any-binary model
- terminal observability outside visual UI

### Оценка как automation donor

`🎯 7   🛡️ 7   🧠 5`  
Примерно `1500-3500` строк.

---

## 14. `terminalcp`

### Snapshot

- Repo: [`badlogic/terminalcp`](https://github.com/badlogic/terminalcp)
- Stars: `118`
- Pushed: `2025-08-17`
- NPM: [`@mariozechner/terminalcp 1.3.3`](https://www.npmjs.com/package/@mariozechner/terminalcp), опубликован `2025-08-17`

### Что это такое

`terminalcp` - persistent terminal server / MCP tool / CLI around `node-pty + @xterm/headless`.

Ключевые pieces:

- `TerminalManager` для PTY sessions и terminal emulation
- `TerminalServer` как background process на Unix socket
- `TerminalClient` для CLI и MCP clients
- attach/detach к живой session
- rendered screen mode и separate stream mode

### Почему это интересно

Это один из самых конкретных найденных примеров того, как terminal стоит проектировать не как view-local штуку, а как **persistent session service**.

Особенно полезны идеи:

- session переживает disconnect клиента
- one session can be used both by AI and by human attach
- screen view и log stream разделены как разные read models

### Что брать как идею

- background terminal server as infrastructure layer
- attach/detach semantics without killing the process
- two output APIs: rendered screen vs incremental stream
- same session reachable from UI, automation, and human terminal

### Где риск

- стек не свежий по emulator части: `@xterm/headless ^5.5.0`
- это не UI foundation
- сам проект полезнее как architecture donor, чем как direct dependency

### Оценка как runtime donor

`🎯 8   🛡️ 7   🧠 5`  
Примерно `2000-4500` строк.

---

## 15. `xterm-pty` и `run-pty`

### Snapshot

- [`mame/xterm-pty`](https://github.com/mame/xterm-pty) - `122` stars, pushed `2026-04-07`
- [`lydell/run-pty`](https://github.com/lydell/run-pty) - `124` stars, pushed `2026-04-07`

### Почему они интересны

`xterm-pty`:

- добавляет PTY layer для `xterm.js`
- особенно интересен для Emscripten / browser-contained TUIs

`run-pty`:

- concurrent command dashboard
- kill/restart all at once
- one-command-at-a-time visible output model

### Что брать как идею

- minimal multi-process dashboard semantics
- command orchestration UX
- alternative PTY bridging models

### Оценка как small donor set

`🎯 6   🛡️ 6   🧠 4`  
Примерно `800-2500` строк.

## Главные новые выводы после этого прохода

### 1. `libghostty` ecosystem уже стал больше, чем просто terminal core

Сейчас вокруг него уже есть:

- embeddable web terminals
- native worktree terminals
- command centers for coding agents
- terminal runtime packagers
- parser/viewer sidecars
- project-based native terminals
- browser terminal dashboards
- IDE/plugin embeddability proofs

### 2. Лучшие новые идеи лежат в двух направлениях

#### A. Runtime / orchestration layer

- `Mux`
- `Supacode`
- `Ghostree`
- `Muxy`
- `Open Terminal`
- `terminalcp`

#### B. Smart-terminal augmentation layer

- `termprompt`
- `ghost-complete`
- `ghostty-opentui`
- `ht`
- `webterm`
- `BooTTY`

### 3. Для нашего продукта это значит следующее

Новая terminal feature должна проектироваться не только как:

- renderer + PTY

А как платформа, куда позже можно добавить:

- structured OSC UI
- shell integration metadata
- autocomplete/completion layers
- parser sidecars for search/indexing
- worktree-aware runtime model
- optional remote/sandbox backend path
- session preview/dashboard surfaces

## Sources

- [Mux](https://github.com/coder/mux)
- [Supacode](https://github.com/supabitapp/supacode)
- [Ghostree](https://github.com/sidequery/ghostree)
- [Muxy](https://github.com/muxy-app/muxy)
- [webterm](https://github.com/rcarmo/webterm)
- [Open Terminal](https://github.com/open-webui/open-terminal)
- [Obsidian Ghostty Terminal](https://github.com/lavs9/obsidian-ghostty-terminal)
- [BooTTY](https://github.com/0xBigBoss/vscode-bootty)
- [ht](https://github.com/andyk/ht)
- [terminalcp](https://github.com/badlogic/terminalcp)
- [terminalcp npm](https://www.npmjs.com/package/@mariozechner/terminalcp)
- [xterm-pty](https://github.com/mame/xterm-pty)
- [run-pty](https://github.com/lydell/run-pty)
- [Trolley](https://github.com/weedonandscott/trolley)
- [ghostty-opentui](https://github.com/remorses/ghostty-opentui)
- [termprompt](https://github.com/seeden/termprompt)
- [termprompt npm](https://www.npmjs.com/package/termprompt)
- [ghost-complete](https://github.com/StanMarek/ghost-complete)
- [Ghostling](https://github.com/ghostty-org/ghostling)
