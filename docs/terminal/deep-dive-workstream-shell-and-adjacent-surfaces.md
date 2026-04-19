# Deep Dive - Workstream Shell And Adjacent Surfaces

**Проверено**: 2026-04-19

## Зачем этот deep dive

Два проекта особенно полезны не для выбора terminal engine, а для понимания того, **как вокруг terminal строится product shell**:

- `Factory Floor` - как workstream становится основной runtime-единицей, а terminal/browsers/editors живут внутри него
- `cmux` - как status bridge, browser adjacency и remote daemon превращаются в отдельные product/runtime слои, а не в UI-хак вокруг pane

Для terminal feature это важно, потому что "удобно как в IDE" почти никогда не появляется только от renderer-а.  
Оно появляется, когда рядом с terminal уже есть:

- workstream/workspace identity
- attention/status system
- persistent adjacent surfaces
- remote/runtime topology с явными границами ответственности

## Primary Sources

### Factory Floor

- [`docs/terminal-spawning.md`](https://github.com/alltuner/factoryfloor/blob/main/docs/terminal-spawning.md)
- [`docs/terminal-resilience-design.md`](https://github.com/alltuner/factoryfloor/blob/main/docs/terminal-resilience-design.md)
- [`docs/remote-coordinator-design.md`](https://github.com/alltuner/factoryfloor/blob/main/docs/remote-coordinator-design.md)
- [`Sources/Models/TmuxSession.swift`](https://github.com/alltuner/factoryfloor/blob/main/Sources/Models/TmuxSession.swift)
- [`Sources/Models/WorkstreamEnvironment.swift`](https://github.com/alltuner/factoryfloor/blob/main/Sources/Models/WorkstreamEnvironment.swift)
- [`Sources/Views/TerminalContainerView.swift`](https://github.com/alltuner/factoryfloor/blob/main/Sources/Views/TerminalContainerView.swift)

### cmux

- [`docs/notifications.md`](https://github.com/manaflow-ai/cmux/blob/main/docs/notifications.md)
- [`docs/remote-daemon-spec.md`](https://github.com/manaflow-ai/cmux/blob/main/docs/remote-daemon-spec.md)
- [`Sources/Workspace.swift`](https://github.com/manaflow-ai/cmux/blob/main/Sources/Workspace.swift)
- [`Sources/RightSidebarPanelView.swift`](https://github.com/manaflow-ai/cmux/blob/main/Sources/RightSidebarPanelView.swift)
- [`Sources/Panels/BrowserPanel.swift`](https://github.com/manaflow-ai/cmux/blob/main/Sources/Panels/BrowserPanel.swift)

## 1. `Factory Floor` - terminal belongs to a workstream shell

### Workstream is the runtime object, not the tab

`Factory Floor` очень последовательно строит продукт не вокруг "открыть terminal tab", а вокруг `workstream`.

Из `terminal-spawning.md` и `TerminalContainerView.swift` видно, что workstream связывает:

- worktree
- coding agent session
- optional setup/run scripts
- optional browser/editor tabs
- environment contract
- restorable workspace tab state

Это сильный product pattern.

🔥 Если в нашей feature появятся tabs/splits/session restore, стоит думать не только "session vs pane", а ещё и "workstream shell above them".

### Terminal/browser/editor lifecycle should be owned by the workspace shell

`TerminalContainerView.swift` хранит отдельный `WorkspaceTabSnapshot`, где persistятся:

- tabs
- active tab
- terminal/browser/editor counters
- browser titles
- terminal titles
- editor file paths
- run state flags

Особенно полезно, что `reconciled(liveSurfaceIDs:)` удаляет умершие terminal tabs, но не сносит browser/editor tabs автоматически.

Это очень зрелый boundary:

- terminal surface life != workspace shell life
- adjacent browser/editor surfaces живут в своей модели
- restore/reconcile работает не только на одном terminal object

### Transparent tmux wrapper is a strong persistence shape

`TmuxSession.swift` прямо формулирует нужную идею: `tmux` должен быть **transparent session persistence wrapper**.

Полезные детали:

- dedicated socket `-L factoryfloor`
- deterministic session name `app/project/workstream/role`
- config intentionally strips tmux UI chrome
- `allow-passthrough on`
- `aggressive-resize on`
- `window-size latest`
- `remain-on-exit on`
- `alternate-screen off`
- `history-limit 50000`

Это очень хороший middle path:

- persistence есть
- shell/TUI не прячутся за ещё одним жирным UI слоем
- external attach/debug остаётся возможным

Для нашей feature это сильный аргумент, что durable sessions не обязаны начинаться со сложного session-daemon своего дизайна.  
Иногда достаточно прозрачного wrapper-а с правильной ownership model.

### Environment contract matters more than it looks

`WorkstreamEnvironment.swift` inject-ит не только `FF_*`, но и compatibility aliases для других ecosystem tools:

- `CONDUCTOR_*`
- `EMDASH_*`
- `SUPERSET_*`

🔥 Это неожиданно сильная идея.

Workstream shell может быть не только "мы придумали свои env vars", а ещё и слоем совместимости с рядом существующих workflows.

Если потом появятся terminal-adjacent scripts, agent templates или worktree automation, такая compatibility-модель может резко уменьшить friction.

### Launch chain failures must become explicit product state

`terminal-resilience-design.md` у `Factory Floor` очень полезен именно своей прагматикой.

Они отдельно фиксируют:

- blank pane is not acceptable failure mode
- surface creation failure needs explicit retry state
- short health check after spawn catches early wrapper failures
- `tmux` stderr надо логировать, а не выбрасывать
- `resume -> fresh` fallback не должен быть silent
- missing launcher binary should degrade visibly

Это сильный reminder:

⚠️ terminal spawning pipeline почти всегда многослойный, и если не проектировать error surface явно, пользователю достаётся "пустая вкладка и тишина".

### Remote coordinator should dispatch, not sit in hot path

`remote-coordinator-design.md` очень полезен даже если мы пока не делаем remote runtime.

У `Factory Floor` coordinator:

- не запускает terminals
- не управляет worktrees
- не находится в keystroke hot path
- только ставит jobs и принимает status updates

Это хороший architecture rule:

- remote dashboard/job board можно строить отдельно
- execution/runtime control лучше оставлять у локального worker/app

## 2. `cmux` - attention UX and browser adjacency need their own runtime model

### Notifications and status are different channels

`notifications.md` очень явно разделяет:

- `cmux notify`
- `cmux set-status`
- `cmux clear-status`

И это очень правильная мысль.

Notification сообщает про событие.  
Status bridge сообщает про **текущее состояние**, которое должно жить дольше единичного toast-а.

Например:

- `Running`
- `Waiting for approval`
- `Error`
- `Idle`

🔥 Для agent-heavy terminal feature status bridge почти так же важен, как notifications.

### Shell-visible control identity is a real integration surface

`cmux` пробрасывает в shell:

- `CMUX_SOCKET_PATH`
- `CMUX_TAB_ID`
- `CMUX_PANEL_ID`

Это полезно сразу в двух смыслах:

- shell/hooks/plugins получают способ говорить с host app
- session/panel identity становится явной, а не скрытой магией renderer-а

Это один из лучших найденных patterns для future shell integration beyond OSC markers.

### Browser adjacency should be workspace-scoped, not tab-scoped

Из `BrowserPanel.swift` видно очень зрелую модель:

- remote workspace получает свой `WKWebsiteDataStore`
- local workspace может использовать profile store
- proxy endpoint хранится на уровне workspace binding
- reconnect может перевешивать browser panel на новый workspace/proxy/store
- localhost для remote routing перекидывается через loopback alias host

Это сильный product/runtime вывод:

🔥 browser рядом с terminal - это не "открыли WebView".  
Нужны ещё:

- workspace identity
- storage/cookie isolation
- proxy identity
- reconnect semantics

Если когда-нибудь делать browser adjacency рядом с terminal, её лучше проектировать как workspace-owned surface.

### Remote browser networking wants one shared proxy path

`remote-daemon-spec.md` даёт очень сильный architectural выбор:

- не зеркалить каждый remote port отдельно
- держать один transport-scoped local proxy endpoint
- browser panels для remote workspaces автоматически вешать на него

Это сильно лучше ad-hoc port mirroring.

Плюсы такого подхода:

- меньше topology chaos
- меньше скрытой state explosion
- browser routing становится частью workspace runtime

### Remote daemon is transport/runtime infrastructure, not pane logic

Из `remote-daemon-spec.md` видно, что `cmux` уже держит отдельный remote/runtime layer:

- remote bootstrap
- daemon hello handshake
- proxy stream RPC
- session open/attach/resize/detach/status/close
- reconnect/re-bootstrap path
- resize semantics `smallest screen wins`

Главный вывод:

⚠️ если когда-нибудь появятся remote terminals, их нельзя прятать внутрь pane-level UI logic.  
Это отдельный runtime subsystem со своим transport, handshake, resize contract и recovery semantics.

### Session index as an adjacent surface is underrated

`RightSidebarPanelView.swift` показывает полезную простую вещь:

- sidebar mode `files`
- sidebar mode `sessions`

Это не выглядит "магической инновацией", но product-value тут высокий:

- session resume видно сразу
- history/adjacent runtime surfaces не приходится прятать в terminal chrome
- workspace shell становится полезнее без превращения в full IDE clone

### Workspace truth already includes more than terminal bytes

`Workspace.swift` у `cmux` хранит и снапшотит не только панели, но и:

- `statusEntries`
- `metadataBlocks`
- `progress`
- layout
- logs
- remote daemon metadata

Это ещё раз подтверждает важную вещь:

terminal-centered workspace truth почти никогда не равен просто `sessionId + scrollback`.

## 3. Что особенно стоит утащить в нашу feature

### A. Workstream shell above terminal panes

Стоит явно решить, нужен ли нашей feature верхний уровень `workstream/workspace shell`, который владеет:

- session group
- adjacent browser/editor/file surfaces
- launch/run state
- attention/status state

Даже если UI будет проще, чем у `Factory Floor`, ownership model от этого станет заметно чище.

### B. Transparent persistence before exotic intelligence

`Factory Floor` хорошо показывает, что early value может дать:

- deterministic session identity
- transparent tmux wrapper
- visible launch diagnostics

А не только "умные suggestions" и сложные overlays.

### C. Status bridge as a first-class integration surface

Из `cmux` стоит почти напрямую утащить идею маленького host bridge:

- `notify`
- `set-status`
- `clear-status`
- maybe `ping`

Это сильная основа для agent workflows, long-running tasks и approval waits.

### D. Browser adjacency only with explicit workspace ownership

Если browser surface вообще появится, она должна жить не как ad-hoc helper рядом с terminal, а как surface с:

- workspace-scoped identity
- isolated storage
- optional proxy binding
- reconnect/update semantics

### E. Remote runtime should begin with topology and threat boundaries

Оба проекта подтверждают полезный принцип:

- remote dispatcher != runtime hot path
- remote daemon != pane widget helper

Сначала нужны topology and responsibility boundaries, потом уже конкретный transport/UI.

## Short Takeaway

`Factory Floor` и `cmux` вместе очень хорошо показывают, что сильная terminal feature обычно состоит из трёх уровней:

1. honest terminal runtime
2. workspace/workstream shell
3. adjacent attention and control surfaces

И именно второй и третий уровни чаще всего решают, будет ли terminal "полезным как в IDE", а не просто ещё одним PTY view.
