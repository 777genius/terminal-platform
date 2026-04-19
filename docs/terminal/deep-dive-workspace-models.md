# Deep Dive: Workspace And Workstream Models

**Проверено**: 2026-04-19

Этот файл нужен, чтобы понять не только terminal core, но и то, как сильные продукты собирают вокруг него **workspace shell**:

- project/workspace/session identity
- durable truth vs runtime observation
- local/worktree/remote runtime modes
- recovery semantics
- terminal adjacency with files, tasks and agents

Для terminal feature это критично, потому что "удобно как в IDE" почти всегда требует не один terminal widget, а нормальную workspace model.

## 1. `OpenCove` - terminal must live inside a truth model

### Какие документы особенно полезны

- `docs/ARCHITECTURE.md`
- `docs/PERSISTENCE.md`
- `docs/RECOVERY_MODEL.md`
- `docs/CONTROL_SURFACE.md`
- `docs/TERMINAL_ANSI_SCREEN_PERSISTENCE.md`
- `docs/SPACE_EXPLORER.md`

### Что стало понятнее

#### A. Durable truth, runtime observation and UI projection must be separated explicitly

`RECOVERY_MODEL.md` у OpenCove очень хорошо раскладывает состояние на 4 класса:

- user intent
- durable fact
- runtime observation
- UI projection

🔥 Это один из самых сильных найденных architecture patterns.

Для terminal feature это почти напрямую означает:

- pane layout != session truth
- PTY alive/exited != resumable durable state
- badges/highlights != business truth
- restoration must rely on durable facts, not on accidental watcher observations

#### B. Recovery model is richer than "restore tabs"

OpenCove явно фиксирует:

- who owns resume binding
- who owns task-to-window relation
- what restart should read from persistence
- what watcher events are allowed to update

Это очень полезно для нас, если terminal должен потом жить рядом с agents/tasks.

#### C. Control Surface as common business entry point

`CONTROL_SURFACE.md` описывает единый `command / query / event` facade для:

- desktop IPC
- CLI
- web UI
- remote worker

Это хорошая сильная идея:

⚠️ terminal-adjacent capabilities should not leak through random renderer hooks or ad-hoc IPC calls.

Если у terminal feature появятся:

- session attach/detach
- search
- notifications
- remote attach
- scrollback export

их лучше моделировать как stable use-case/control-surface calls.

#### D. Alternate-screen persistence requires a committed screen cache

`TERMINAL_ANSI_SCREEN_PERSISTENCE.md` особенно полезен.

Они фиксируют реальную проблему:

- raw PTY snapshot cap can trim away the semantics of entering alt-screen
- on restore, replaying raw delta can clobber the last full-screen frame

Их solution:

- keep serialized committed screen state
- treat alt-screen restore as a special case
- only replay raw delta in specific safe conditions
- suppress resize noise during alt-screen restore

🔥 Это очень сильный practical lesson.

Persisting a terminal is not the same as storing "last N chars".

#### E. Space Explorer shows terminal adjacency model

`SPACE_EXPLORER.md` показывает useful shape:

- workspace surface owns an explorer/panel tied to the current space
- opening files creates/focuses canvas nodes
- file system access goes through approved boundaries

Это полезно не потому, что нам нужен canvas.  
А потому, что terminal feature often needs **adjacent surfaces**, not full IDE replacement.

### Что утащить как идею

- truth classes: durable vs runtime vs UI projection
- recovery model with explicit owners
- control-surface unification
- committed-screen restore for alternate buffers
- terminal adjacency as bounded side surfaces

---

## 2. `Nezha` - multi-project workspace as the product shell

### Какие материалы особенно полезны

- `README.md`
- `src-tauri/src/pty.rs`
- `src-tauri/src/session.rs`
- `src-tauri/src/git.rs`

### Что стало понятнее

`Nezha` интересен не из-за terminal engine, а из-за product framing:

- multi-project workspace
- fast task switching
- real-time terminal
- session auto-discovery
- native git integration
- lightweight editor

Главный insight:

terminal convenience grows sharply when the app treats:

- project
- task
- session
- git state

as one visible working set.

Это не значит, что надо копировать их UI.  
Но это подтверждает, что terminal feature is stronger when it is **project-aware**, not isolated.

### Что утащить как идею

- multi-project awareness
- session auto-discovery as first-class product behavior
- git/task/session adjacency as one workspace shell

---

## 3. `Mux` - runtime choice is a product feature, not an implementation detail

### Какие документы особенно полезны

- `docs/workspaces/index.mdx`
- `docs/runtime/local.mdx`
- `docs/runtime/worktree.mdx`
- `docs/runtime/ssh.mdx`

### Что стало понятнее

#### A. Runtime mode is explicit user-facing truth

Mux makes runtime choice explicit:

- `Local`
- `Worktree`
- `SSH`

И каждый mode has different semantics:

- isolation level
- filesystem path behavior
- security profile
- credential exposure
- review workflow

🔥 Это очень полезный framing.

If we ever support more than one terminal runtime mode, it should not be hidden in infra.
It should be a first-class product concept.

#### B. Local runtime warns about shared working copy

`local.mdx` explicitly documents:

- no isolation
- conflicts if multiple local workspaces stream simultaneously
- same project directory reused directly

Это хороший pattern:

when isolation is weak, the product should say so explicitly instead of pretending all runtimes are equivalent.

#### C. SSH runtime has a real threat model

`ssh.mdx` explicitly states:

- remote host treated as potentially hostile
- by default local keys/credentials are not forwarded
- only git archive + configured secrets are synced

Это очень важный design rule if remote terminals ever appear in our roadmap.

Remote runtime should begin with a threat model, not with "can we make SSH work".

#### D. Worktree runtime gives deterministic filesystem layout

`worktree.mdx` documents:

- isolated directories under `~/.mux/src/<project>/<workspace>`
- shared `.git`
- flexible branch/detached HEAD behavior

Это makes workspaces understandable and debuggable.

### Что утащить как идею

- runtime mode as first-class workspace property
- explicit warnings for weak-isolation modes
- threat model for remote runtime
- deterministic workspace filesystem layout

---

## 4. `Ghostling` - minimal libghostty boundary is now very clear

### Какие материалы особенно полезны

- `README.md`
- `main.c`

### Что стало понятнее

`Ghostling` полезен не тем, что это a good product, а тем, что он very clearly shows the lower boundary:

- libghostty-vt gives terminal emulation and render state
- consumer provides renderer, windowing, input events
- GUI features like tabs, windows, splits, session management, search UI are out of scope

README even states that some higher-level internals like search internals exist, but search UI does not.

Это полезно для decision making:

⚠️ direct libghostty path is a great emulation base, but it is not a shortcut to workspace UX.

### Что утащить как идею

- libghostty is core state/render-state, not workspace shell
- higher-level product layer remains our responsibility

---

## Synthesis

После deep dive по workspace-oriented projects картинка стала такой:

### 1. Terminal feature needs a truth model

If the product has sessions, tasks, worktrees, agent states or restore semantics, we must explicitly distinguish:

- durable facts
- runtime observations
- UI projections

### 2. Runtime choice should be visible

Local/worktree/remote are not interchangeable implementation details.  
They are different product semantics.

### 3. Recovery must preserve what the user actually saw

Especially for alternate-screen TUIs, "store the latest bytes" is not enough.

### 4. Terminal becomes much more useful when it is project-aware

Project, session, task, git status and nearby file/browser surfaces reinforce each other.

### 5. Strong terminal products rely on a common control surface

Not on duplicated special cases in renderer hooks, CLI wrappers and remote adapters.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Nezha](https://github.com/hanshuaikang/nezha)
- [Mux](https://github.com/coder/mux)
- [Ghostling](https://github.com/ghostty-org/ghostling)
