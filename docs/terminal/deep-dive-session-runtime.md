# Deep Dive: Session Runtime And Persistence

**Проверено**: 2026-04-19

Этот файл про то, как самые интересные проекты решают не renderer, а более сложную часть terminal feature:

- persistent sessions
- attach/detach
- rendered view vs raw stream
- event delivery
- task automation поверх terminal session

## Почему это важно

Во многих terminal products основная сложность живёт не в VT parser, а в ответах на вопросы:

- session живёт внутри UI или отдельно?
- как reconnect/attach работают на практике?
- что именно считается persistent:
  - layout
  - scrollback
  - process state
- UI читает rendered screen или raw output stream?

Ниже - два самых полезных проекта по этой теме.

## 1. `zmx`

### Snapshot

- Repo: [`neurosnap/zmx`](https://github.com/neurosnap/zmx)
- Stars: `1340`
- Pushed: `2026-04-17`
- Current binaries/docs: `0.5.0`
- Stack: Zig + `ghostty-vt`

### Что это такое на самом деле

`zmx` не пытается быть terminal app с окнами, вкладками и split panes.

Он очень честно ограничивает scope:

- persist terminal shell sessions
- attach/detach
- native terminal scrollback
- multiple clients
- run commands without attaching
- output history as plain text / VT / HTML

README прямо пишет, что он **не** даёт windows, tabs, or splits.

### Самый важный architectural insight

🔥 `zmx` показывает, что полезная persistence model может жить в очень маленьком runtime surface.

По repo shape видно:

- `src/main.zig`
- `socket.zig`
- `ipc.zig`
- `util.zig`
- `cross.zig`
- `log.zig`
- `completions.zig`

То есть это не гигантская IDE.  
Это компактный session runtime вокруг `ghostty-vt`.

### Что у него реально first-class

Не panes.  
Не renderer surface.  
А **session socket + task lifecycle**.

Особенно сильные команды:

- `attach`
- `run`
- `wait`
- `tail`
- `history`
- `write`

Это уже почти готовый vocabulary для terminal application/use-case layer.

### Самые интересные идеи

#### A. `run / wait / tail / history` как отдельные read/write models

Это очень сильная идея:

- `run` - отправить команду в existing session
- `wait` - дождаться completion detached task
- `tail` - follow output
- `history` - получить scrollback в нужном формате

То есть session API не должен быть только "attach and look at terminal".  
Он может иметь более task-oriented surface.

#### B. `write <file>` through the session

Это редкая, но очень полезная идея:

- запись файла через уже существующую session
- работает даже over SSH
- useful when runtime is remote and terminal session already has the right cwd/env/auth context

Для нашей feature это не P0, но как future runtime primitive это очень интересно.

#### C. Session identity is shell-visible

`ZMX_SESSION` и prompt integration examples показывают важный pattern:

- session identity should be visible inside shell
- not only in host UI chrome

Это помогает и человеку, и automation.

### Ограничения, которые важно не путать

`zmx` не equal IDE terminal.

Он не решает:

- tabs/splits/layout
- browser/editor adjacency
- search UX inside app
- product-level attention system

Но это не минус.  
Это делает его очень чистым donor для session runtime layer.

### Что брать как идею

- session runtime separate from UI
- task-oriented session API
- history export in multiple formats
- shell-visible session identity
- narrow scope is a feature

### Вывод

`zmx` - лучший найденный donor для **minimal durable session runtime**.

`🎯 8   🛡️ 7   🧠 6`  
Примерно `2000-5000` строк, если переносить идеи.

---

## 2. `terminalcp`

### Snapshot

- Repo: [`badlogic/terminalcp`](https://github.com/badlogic/terminalcp)
- Stars: `118`
- Pushed: `2025-08-17`
- NPM: [`@mariozechner/terminalcp 1.3.3`](https://www.npmjs.com/package/@mariozechner/terminalcp)
- Stack: Node.js + `node-pty` + `@xterm/headless`

### Что это такое на самом деле

`terminalcp` - centralized terminal session manager with:

- CLI
- MCP server
- background daemon
- attach client

Это уже не просто tool wrapper, а настоящий **session server**.

### Архитектурная форма

Кодовая форма у него очень показательная:

- `terminal-manager.ts` - PTY + headless terminal state
- `terminal-server.ts` - Unix socket daemon
- `terminal-client.ts` - client transport
- `attach-client.ts` - interactive attach
- `mcp-server.ts` - automation surface
- `index.ts` - CLI entry

Эта decomposition очень полезна для нашей feature.

### Самые важные технические идеи

#### A. Separate server process on Unix socket

Server lives at `~/.terminalcp/server.sock`.

Это даёт:

- session survives client disconnect
- CLI and MCP reuse the same session runtime
- one session can be observed by multiple consumers

Это очень сильный pattern для `main/infrastructure`.

#### B. Two output models: rendered screen vs raw stream

В `TerminalManager` есть:

- `getOutput` - rendered terminal screen through `@xterm/headless`
- `getStream` - raw output stream, optionally incremental and ANSI-stripped

🔥 Это, возможно, самый полезный найденный паттерн.

UI и automation почти никогда не хотят одно и то же представление terminal output.

Нужно явно иметь:

- screen view
- raw/log view

### C. Write queues for correctness

В `terminal-manager.ts` есть два serial queues:

- `terminalWriteQueue`
- `ptyWriteQueue`

Это не cosmetic detail.  
Это защита от race conditions:

- output must be serialized before writing into virtual terminal
- input to PTY should not interleave unpredictably

Это очень хороший concrete reminder, что terminal runtime почти всегда требует explicit ordering.

#### D. Subscription model per session

`terminal-server.ts` держит:

- clients map
- `sessionSubscribers`

То есть runtime не пушит события всем подряд, а знает, кто подписан на какую session.

Для desktop app это тоже важно:

- active pane
- preview/dashboard
- background watcher
- automation client

могут хотеть разные delivery modes.

### Где у него границы

Важно не переоценить `terminalcp`:

- persistence у него в первую очередь process/runtime-level, не rich product-level
- output state в памяти, не сложная durable database model
- emulator часть не самая свежая: `@xterm/headless ^5.5.0`

То есть он не лучший direct dependency, но очень сильный architecture donor.

### Что брать как идею

- background daemon/session server
- Unix socket or equivalent local IPC
- rendered screen and raw stream as separate APIs
- explicit write queues
- attach subscriptions
- one runtime serving CLI, app UI, and automation

### Вывод

`terminalcp` - лучший найденный donor для **terminal session service architecture**.

`🎯 8   🛡️ 7   🧠 5`  
Примерно `2000-4500` строк, если переносить идеи.

---

## Сравнение `zmx` vs `terminalcp`

### `zmx`

- `🎯 8   🛡️ 7   🧠 6`
- Лучше как inspiration для minimal durable session runtime
- Сильнее в detach/reattach philosophy and shell-native workflow
- Интереснее как "what if we keep scope very narrow"

### `terminalcp`

- `🎯 8   🛡️ 7   🧠 5`
- Лучше как inspiration для background session service inside app architecture
- Сильнее в explicit client/server decomposition
- Намного полезнее в read-model split: `screen` vs `stream`

## Что это значит для нашей feature

Если собирать сильную terminal feature, стоит разделить два слоя:

### 1. Session runtime layer

Отсюда брать:

- daemon/service semantics
- attach/detach
- subscriptions
- task-oriented commands
- screen/log dual read models

### 2. Product/UI layer

Сюда класть:

- tabs/splits
- search
- previews
- attention UX
- browser/editor adjacency

## Новые идеи для нашей архитектуры

### 1. Ввести два read models в contracts

- `TerminalScreenSnapshotDto`
- `TerminalStreamChunkDto`

Не пытаться одним DTO покрыть и visual screen, и log stream.

### 2. Session runtime не должен жить в renderer lifecycle

Если terminal pane remount-ится, session service не должен пересоздаваться.

### 3. Надо отдельно определить "persistence level"

- level 1 - layout restore
- level 2 - scrollback restore
- level 3 - live process persistence

И быть очень точными, что именно мы обещаем пользователю.

### 4. Task-oriented API полезнее, чем только attach API

Например:

- `runInSession`
- `waitForSessionTask`
- `readSessionHistory`
- `tailSession`

Это может очень пригодиться и UI, и automation.

## Sources

- [zmx](https://github.com/neurosnap/zmx)
- [zmx docs](https://zmx.sh)
- [terminalcp](https://github.com/badlogic/terminalcp)
- [terminalcp npm](https://www.npmjs.com/package/@mariozechner/terminalcp)
