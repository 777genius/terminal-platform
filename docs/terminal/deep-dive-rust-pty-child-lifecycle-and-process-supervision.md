# Deep Dive - Rust PTY Child Lifecycle And Process Supervision

**Проверено**: 2026-04-19

## Зачем этот слой важен

Для embeddable terminal runtime мирового уровня мало уметь:

- открыть PTY
- читать и писать байты
- отрисовать screen state

Нужно ещё корректно держать:

- spawn lifecycle
- kill / start_kill / wait / try_wait semantics
- process groups and sessions on Unix
- job objects on Windows
- shutdown and detach behavior
- signal compatibility and reaping

🔥 Именно здесь большинство terminal runtimes становятся platform spaghetti:

- PTY слой начинает владеть supervision semantics
- host app начинает знать слишком много про signals и job objects
- kill-on-drop делается как магия без явной policy
- detached session model конфликтует с UI lifecycle

Для reusable package этот слой должен быть чётко выделен.

## Primary Sources

### PTY and child abstractions

- [`portable-pty` lib.rs](https://github.com/wezterm/wezterm/blob/main/pty/src/lib.rs)
- [`pty-process` docs](https://docs.rs/pty-process/0.5.3/pty_process/)

### Process supervision and wrappers

- [`process-wrap` README](https://github.com/watchexec/process-wrap/blob/main/README.md)
- [`process-wrap` std/core.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/core.rs)
- [`process-wrap` process_group.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/process_group.rs)
- [`process-wrap` process_session.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/process_session.rs)
- [`process-wrap` job_object.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/job_object.rs)

### Signals and lower-level syscalls

- [`signal-hook` README](https://github.com/vorner/signal-hook/blob/master/README.md)
- [`rustix` README](https://github.com/bytecodealliance/rustix/blob/main/README.md)
- [`nix` README](https://github.com/nix-rust/nix/blob/master/README.md)
- [`wait-timeout` README](https://github.com/alexcrichton/wait-timeout/blob/master/README.md)

### Shared child helper

- [`shared_child.rs` repo](https://github.com/oconnor663/shared_child.rs)

## Freshness signals

- `portable-pty 0.9.0`
- `process-wrap 9.1.0` - repo `watchexec/process-wrap`, `43` stars, pushed `2026-04-18`
- `signal-hook 0.4.4` - repo `vorner/signal-hook`, `848` stars, pushed `2026-04-04`
- `nix 0.31.2`
- `rustix 1.1.4`
- `shared_child 1.1.1` - repo `oconnor663/shared_child.rs`, `51` stars, pushed `2026-01-22`
- `wait-timeout 0.2.1`
- `pty-process 0.5.3`
- `wezterm` repo `25.6k` stars, pushed `2026-04-01`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**PTY port и process supervision policy нельзя склеивать в один слой**

На сейчас healthiest shape выглядит так:

1. `portable-pty` owns PTY creation and basic child handle
2. runtime owns lifecycle policy
3. process/session/job semantics live in a dedicated supervision adapter layer
4. host UI only talks to typed session commands and state

То есть не:

- "the PTY crate somehow solves all process semantics"

а:

- `PtyPort`
- `ProcessSupervisionPort`
- `SessionRuntime`

## Top 3 directions for Rust PTY lifecycle and supervision

### 1. `portable-pty + dedicated supervision layer inspired by process-wrap + signal-hook`

`🎯 10   🛡️ 9   🧠 8`
Примерно `6000-12000` строк.

Это мой текущий **лучший default**.

Почему:

- `portable-pty` already gives the right PTY trait seam
- `process-wrap` shows an excellent composable-wrapper model
- `signal-hook` gives safe Unix signal coordination where needed

Но важная честная деталь:

⚠️ `process-wrap` сейчас выглядит **сильнее как donor model**, чем как готовый drop-in PTY wrapper.

То есть брать стоит не идею "один crate решит всё", а идею:

- PTY child handle
- separate supervision wrappers/policies
- explicit session/group/job semantics

### 2. `portable-pty + direct rustix/nix + windows-specific job adapter`

`🎯 8   🛡️ 8   🧠 9`
Примерно `8000-15000` строк.

Это сильный low-level path, если нужен максимальный контроль.

Почему:

- `rustix` is a very strong low-level Unix syscall layer
- `nix` stays useful for mature Unix process/signal abstractions
- custom Windows supervision can mirror job object semantics directly

Почему не default:

- more platform-specific code
- higher maintenance burden
- easier to leak low-level syscall semantics into public architecture

### 3. `pty-process + shared_child + wait-timeout` helper-first stack

`🎯 6   🛡️ 6   🧠 5`
Примерно `3000-7000` строк.

Это интересный smaller helper path для narrow products or prototypes.

Почему:

- `pty-process` gives a simpler PTY spawn story
- `shared_child` helps with multi-thread wait/kill coordination
- `wait-timeout` is a small cross-platform helper

Почему не world-class default:

- less architectural gravity than `portable-pty`
- supervision semantics stay comparatively thin
- not the strongest base for a reusable cross-host platform

## 1. `portable-pty` already has the right fundamental seam

`portable-pty` оказался ещё сильнее после более внимательного чтения `pty/src/lib.rs`.

Ключевые design points:

- `PtySystem`
- `MasterPty`
- `SlavePty`
- `Child`
- `ChildKiller`

Особенно важны:

- `Child::try_wait`
- `Child::wait`
- `Child::process_id`
- `ChildKiller::kill`
- `ChildKiller::clone_killer`

🔥 `clone_killer()` особенно важен.

Это очень сильный signal, что even at PTY level:

- waiting
- signalling
- cross-thread coordination

уже должны быть разведены.

Ещё одна полезная деталь:

- `PtyPair` явно фиксирует drop order: `slave` before `master`

Это маленькая, но очень здоровая подсказка, что resource/lifecycle semantics в terminal runtime должны быть deliberate, not accidental.

## 2. `process-wrap` shows the healthiest supervision model

`process-wrap` оказался важен не только как crate, а как архитектурный референс.

Главный принцип там очень правильный:

> composable wrappers which implement a single concern each

Это почти идеальный Port/Adapter mindset.

Полезные wrappers:

- `ProcessGroup`
- `ProcessSession`
- `JobObject`
- `KillOnDrop`
- `CreationFlags`

Их сильная сторона:

- one wrapper = one concern
- policies can be combined explicitly
- lifecycle semantics stay visible

🔥 Это очень похоже на то, как должен выглядеть наш `ProcessSupervisionPort`.

### Но есть важная граница

`process-wrap` today wraps `std::process::Command` / `Child` style flows.

То есть для нашего runtime strongest practical interpretation now is:

- do not assume `process-wrap` directly solves PTY spawning
- do borrow its supervision model and wrapper decomposition
- if needed, build a PTY-aware supervision adapter with similar shape

## 3. `process_group`, `process_session`, and `job_object` should stay product policies

Из `process_group.rs`, `process_session.rs` и `job_object.rs` видно очень полезное разделение:

### Unix

- `ProcessGroup::leader()` creates a fresh process group
- `ProcessGroup::attach_to(...)` binds to an existing group
- `ProcessSession` calls `setsid()` in `pre_exec`
- group wait/reap semantics are explicit

### Windows

- `JobObject` sets `CREATE_SUSPENDED`
- attaches the child to a Job Object
- resumes threads only when appropriate
- waits and terminates on the job, not only the top-level child

🔥 Practical rule:

**group/session/job behavior is product policy, not PTY trivia**

That means:

- different host apps may want different defaults
- these choices should be explicit in runtime config/capabilities
- they should not be hardcoded deep inside the PTY adapter

## 4. `signal-hook` is useful, but only in a narrow place

`signal-hook` is still one of the best sources for Unix signal sanity.

Important reminder from its README:

- signals are global
- handlers are constrained
- locking/allocation inside handlers is dangerous

So for this runtime:

- use it for narrow shutdown/termination coordination where needed
- do not spread signal behavior throughout the codebase
- do not let libraries compete over global signal ownership accidentally

This fits our architecture well:

- supervision adapters may use signal helpers internally
- public runtime semantics should still be command/state oriented

## 5. `rustix` and `nix` are excellent internal tools, not public architecture centers

This pass made the boundary clearer.

### `rustix`

Strong when you need:

- low-level safe syscall access
- PTY and process operations on Unix
- explicit control and performance

But it is not a cross-platform lifecycle solution by itself.

### `nix`

Strong when you need:

- mature Unix wrappers
- `setsid`, `killpg`, `waitpid`, signals and friends

But it also should stay internal.

🔥 Practical rule:

**`rustix`/`nix` can power adapters, but should not become the public mental model of the package**

## 6. `shared_child` and `wait-timeout` are helpers, not runtime truth

### `shared_child`

Interesting because it addresses:

- multiple threads wanting to `wait` or `kill`

That is useful if the runtime ever needs:

- temporary compatibility with multi-threaded ownership
- adapter seams around existing `Child` APIs

But it is weaker than an owner-task runtime as the primary architecture.

### `wait-timeout`

Useful little helper for:

- bounded shutdown
- polite termination escalation

But:

- it is just one primitive
- it should not define the lifecycle model

## 7. `pty-process` is interesting, but not the center

`pty-process` is useful as a smaller helper-oriented reference:

- PTY-attached process spawning
- optional async support

But for the kind of world-class reusable package you want, it currently looks more like:

- a useful side reference
- not the strongest foundation compared with `portable-pty`

## 8. Recommended runtime shape now

At this point, the healthiest architecture looks like:

1. `PtyPort`
   - backed by `portable-pty`
2. `ProcessSupervisionPort`
   - explicit policies: `group`, `session`, `job`, `kill_on_drop`, `timeout_escalation`
3. `SessionRuntime`
   - owns child lifecycle state
   - attach/detach
   - shutdown and reap ordering
4. `HostControlSurface`
   - typed commands like `start`, `terminate`, `detach`, `attach`, `force_kill`

This keeps:

- PTY details contained
- platform supervision explicit
- host bindings clean

## 9. What I would explicitly avoid

- ❌ assuming the PTY crate should own all process-group/session semantics
- ❌ hardcoding Unix group behavior as if Windows were just a compatibility afterthought
- ❌ hiding kill-on-drop inside random `Drop` impls without policy/config visibility
- ❌ letting UI bindings trigger raw process/signal logic directly
- ❌ exposing `nix` or `rustix`-shaped APIs to hosts
- ❌ using helper crates as substitutes for a real owner-task runtime

## Final recommendation

If building this runtime today, I would choose:

- PTY abstraction: `portable-pty`
- lifecycle/supervision model: `process-wrap`-inspired dedicated adapter layer
- Unix signal helper: `signal-hook`, but narrowly
- internal Unix syscalls: `nix` and/or `rustix` only where they materially simplify adapters
- helper crates like `shared_child` and `wait-timeout` only as secondary tools, not as the architecture center

🔥 The most important practical takeaway:

**do not search for one Rust crate that solves PTY plus lifecycle plus supervision plus detach semantics**

The healthier world-class design is:

- one crate for PTY capability
- one explicit supervision policy layer
- one runtime owner that turns both into stable host-facing semantics

## Sources

- [portable-pty](https://github.com/wezterm/wezterm/blob/main/pty/src/lib.rs)
- [process-wrap README](https://github.com/watchexec/process-wrap/blob/main/README.md)
- [process-wrap core.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/core.rs)
- [process-wrap process_group.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/process_group.rs)
- [process-wrap process_session.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/process_session.rs)
- [process-wrap job_object.rs](https://github.com/watchexec/process-wrap/blob/main/src/std/job_object.rs)
- [signal-hook](https://github.com/vorner/signal-hook)
- [rustix](https://github.com/bytecodealliance/rustix)
- [nix](https://github.com/nix-rust/nix)
- [wait-timeout](https://github.com/alexcrichton/wait-timeout)
- [shared_child.rs](https://github.com/oconnor663/shared_child.rs)
- [pty-process docs](https://docs.rs/pty-process/0.5.3/pty_process/)
