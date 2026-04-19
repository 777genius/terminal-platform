# Research: Product Patterns And Donor Projects

**Проверено**: 2026-04-19

Этот файл про проекты, которые полезны не как terminal foundation, а как источник сильных UX и operational patterns.

## Почему donor-проекты важнее, чем кажется

Один и тот же terminal core может давать очень разный user value.

Например:

- обычный terminal tab
- IDE-like terminal workspace
- agent-first command center
- spatial workspace с несколькими running contexts

Разница тут не в ANSI parser, а в том, как устроены:

- session persistence
- notification/attention model
- worktree model
- browser/editor/task integration
- search/control center

## 1. `cmux`

### Snapshot

- Repo: [`manaflow-ai/cmux`](https://github.com/manaflow-ai/cmux)
- Stars: `14730`
- Pushed: `2026-04-19`

### Почему это один из лучших donor-проектов

`cmux` очень силён не как "ещё один terminal app", а как **attention system for parallel agent work**.

Что у него особенно ценно:

- vertical tabs
- notification rings прямо на pane
- unread notification navigation
- sidebar metadata: branch, PR status, cwd, ports, last notification
- built-in browser рядом с terminal
- scriptable CLI + socket API
- `OSC 9/99/777`-driven notification model

### Что нужно украсть как идею

- pane attention states
- terminal notifications не только системными toast, а в layout itself
- sidebar с context summary вместо просто списка вкладок
- явный CLI/socket automation surface

### Почему это не foundation для нас

- native macOS app
- не embeddable library
- слишком продукт, а не SDK

### Вывод

`cmux` - топовый donor для attention UX.

---

## 2. `Factory Floor`

### Snapshot

- Repo: [`alltuner/factoryfloor`](https://github.com/alltuner/factoryfloor)
- Stars: `91`
- Pushed: `2026-04-16`

### Что в нём особенно сильное

`Factory Floor` строит workflow не вокруг "terminal tab", а вокруг **workstream**:

- git worktree
- dedicated Claude Code session
- terminal
- browser
- editor

Важные идеи:

- `tmux` persistence через dedicated socket
- deterministic port assignment
- auto browser navigation to detected dev server
- setup/run/teardown scripts per project
- keyboard-first workflow

### Что нужно украсть как идею

- workstream как first-class entity
- worktree + session + dev server как единый runtime object
- deterministic per-workstream env vars
- terminal рядом с browser/editor, а не изолированно

### Почему это не foundation

- native macOS app
- это уже целый product shell

### Вывод

`Factory Floor` - топовый donor для `worktree + persistence + browser` workflow model.

---

## 3. `OpenCove`

### Snapshot

- Repo: [`DeadWaveWave/opencove`](https://github.com/DeadWaveWave/opencove)
- Stars: `1143`
- Pushed: `2026-04-18`

### Почему он важен

`OpenCove` особенно полезен тем, что показывает:

можно взять вполне обычный foundation (`xterm.js + node-pty`) и всё равно сделать намного более ценный продукт за счёт workspace UX.

Ключевые паттерны:

- infinite spatial canvas
- persistent workspaces
- terminals + notes + tasks + agents on one plane
- global search/control center
- workspace isolation with directories and worktrees

### Что нужно украсть как идею

- terminal не обязан жить только в tab strip
- layout persistence реально важна
- search/control center должен смотреть не только в files, но и в terminal state
- рядом с terminal должны жить notes/tasks/context, а не только raw output

### Почему это не foundation

- сам использует `xterm.js + node-pty`
- ценность в UX shell, а не в terminal engine

### Вывод

`OpenCove` - лучший donor для `spatial workspace` и `context never disappears`.

---

## 4. `Nezha`

### Snapshot

- Repo: [`hanshuaikang/nezha`](https://github.com/hanshuaikang/nezha)
- Stars: `397`
- Pushed: `2026-04-19`
- Stack: `Tauri + React + TypeScript`

### Что в нём особенно сильное

`Nezha` хорошо показывает lightweight agent-first desktop pattern:

- multi-project workspace
- session auto-discovery
- task lifecycle visualization
- native Git integration
- code/markdown editors
- usage analytics

### Что важно для нас

Это полезное подтверждение, что terminal feature в agent product должна быть связана с:

- sessions
- tasks
- git
- project switching

А не только с raw shell.

### Что нужно украсть как идею

- session auto-discovery
- project-level status indicators
- waiting-for-input highlighting
- task lifecycle рядом с terminal runtime

### Почему это не foundation

- underlying terminal там всё равно `xterm.js`
- ценность снова в orchestration/UI

### Вывод

`Nezha` - хороший donor для `agent-first terminal workspace`, особенно если нужно не раздувать продукт до монструозной IDE.

---

## 5. `zmx`

### Snapshot

- Repo: [`neurosnap/zmx`](https://github.com/neurosnap/zmx)
- Stars: `1340`
- Pushed: `2026-04-17`
- Current binaries/docs: `0.5.0`

### Почему это очень важная находка

`zmx` не про renderer. Он про то, что реально делает terminal полезным в ежедневной работе:

- attach/detach
- restore previous terminal state/output
- native scrollback
- multiple clients on same session
- send commands without attach

### Что нужно украсть как идею

🔥 Session persistence важнее ещё одного renderer.

Нам стоит думать о terminal session как о durable object:

- живёт дольше конкретного view
- может быть reopened
- имеет replay/history
- допускает background run + foreground attach

### Почему это не foundation

- это persistence/session layer
- tabs, splits, search, renderer не решает

### Вывод

`zmx` - один из самых ценных infra donors для новой terminal feature.

---

## 6. `termscope`

### Snapshot

- Repo: [`mwunsch/termscope`](https://github.com/mwunsch/termscope)
- Stars: `8`
- Pushed: `2026-04-08`

### Что в нём особенно интересно

`termscope` - headless terminal automation CLI powered by `libghostty-vt`.

Что он даёт:

- snapshot terminal в `text/json/html/svg`
- programmatic `type`, `press`, `resize`, `wait`
- JSON-lines session protocol
- CI / testing / agent-friendly interaction story

### Что нужно украсть как идею

Если новая terminal feature будет важной частью продукта, её нужно уметь:

- автоматизированно тестировать
- snapshot-ить
- инспектировать вне UI
- использовать как automation surface

### Почему это не foundation

- это headless automation layer
- не заменяет renderer/UI

### Вывод

`termscope` - очень сильная идея для internal automation, QA и agent tooling.

---

## 7. `Hermes IDE`

### Snapshot

- Repo: [`hermes-hq/hermes-ide`](https://github.com/hermes-hq/hermes-ide)
- Stars: `208`
- Pushed: `2026-04-16`
- Latest release: `v0.6.15` от `2026-04-16`
- Stack: `Tauri + React + Rust`

### Почему это полезный donor

`Hermes IDE` важен не тем, что это "ещё один terminal app", а тем, что это очень свежий пример AI-native desktop продукта, который строит terminal ценность поверх вполне приземлённого foundation.

Подтверждённые публично вещи:

- multi-session management
- split panes
- execution timeline с exit codes и durations
- built-in git panel и inline diff viewer
- project scanning и context injection
- system notifications

### Что в нём особенно важно для нас

🔥 Это полезное доказательство важного рыночного тезиса:

даже очень свежий AI-native IDE продукт всё ещё может сидеть на `@xterm/xterm 6.0.0`, `@xterm/addon-webgl 0.19.0` и `@xterm/headless ^6.0.0`, а выигрывать в UX не exotic engine-ом, а product layer.

### Что нужно украсть как идею

- execution timeline как adjacent surface к terminal, а не только raw scrollback
- terminal + git sidebar как единый workflow
- project awareness tied to session runtime
- command palette и memory/context pins рядом с terminal work

### Почему это не foundation

- лицензия `BSL 1.1` делает проект плохим кандидатом для reuse/fork как базы competing terminal/IDE продукта
- foundation у них всё равно `xterm`-based

### Вывод

`Hermes IDE` - сильный donor для `execution timeline + project-aware terminal workspace`, но не для прямого reuse как foundation.

## Какие product patterns повторяются у лучших проектов

### 1. Session-first, not tab-first

Хорошие продукты держат focus не на "терминальной вкладке", а на сущности session/workstream/workspace.

### 2. Attention routing

Пользователю надо быстро понять:

- где агент ждёт input
- где что-то сломалось
- где есть непрочитанное изменение состояния

### 3. Terminal + Browser + Editor + Task context

У сильных workflow products terminal редко живёт в одиночестве.

### 4. Persistence as default

Сессии не должны исчезать только потому, что view закрылся или app перезапустился.

### 5. Search/control center

Пользователь должен искать не только по файлам, но и по sessions, output, tasks, notes.

## Что это значит для нашей feature

Новая terminal feature не должна проектироваться как:

- "просто modal с PTY"
- "просто tab strip с xterm"

Она должна проектироваться как:

- session workspace
- with persistence
- with attention system
- with shell integration
- with adjacent surfaces

## Sources

- [cmux](https://github.com/manaflow-ai/cmux)
- [Factory Floor](https://github.com/alltuner/factoryfloor)
- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Nezha](https://github.com/hanshuaikang/nezha)
- [Hermes IDE](https://github.com/hermes-hq/hermes-ide)
- [zmx](https://github.com/neurosnap/zmx)
- [termscope](https://github.com/mwunsch/termscope)
