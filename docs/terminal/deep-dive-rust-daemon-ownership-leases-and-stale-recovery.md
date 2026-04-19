# Deep Dive - Rust Daemon Ownership, Leases, and Stale Recovery

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для embeddable terminal runtime мало иметь:

- хороший local transport
- хороший state dir layout
- хорошие lock primitives

Нужен ещё ответ на более неприятный вопрос:

- кто сейчас canonical owner daemon-а
- как новый host понимает, что daemon живой
- когда можно reclaim-ить stale runtime
- как отличить "daemon ещё стартует" от "daemon уже мёртв"
- как не превратить singleton logic в набор race conditions

🔥 Именно здесь часто появляются deceptively simple решения:

- "просто pidfile"
- "просто single-instance crate"
- "просто lock file"

А потом реальность ломает это при:

- crash между bind и publish
- stale socket path after kill -9
- long startup after migrations or snapshot rebuild
- competing host processes after reboot/login restore
- Electron restart while Rust daemon still lives

Для world-class reusable package ownership model must be first-class.

## Primary Sources

### Lock and ownership primitives

- [`fs4` crate](https://crates.io/crates/fs4)
- [`fs4-rs` repo](https://github.com/al8n/fs4-rs)
- [`fd-lock` crate](https://crates.io/crates/fd-lock)
- [`fd-lock` repo](https://github.com/yoshuawuyts/fd-lock)

### Singleton and pidfile-oriented helpers

- [`single-instance` crate](https://crates.io/crates/single-instance)
- [`single-instance` repo](https://github.com/WLBF/single-instance)
- [`pidlock` crate](https://crates.io/crates/pidlock)
- [`pidlock` repo](https://github.com/rockstar/pidlock)
- [`pidfile-rs` crate](https://crates.io/crates/pidfile-rs)
- [`pidfile-rs` repo](https://github.com/andrewshadura/pidfile-rs)

### Liveness helpers

- [`process_alive` crate](https://crates.io/crates/process_alive)
- [`process_alive` repo](https://github.com/caido/process_alive)
- [`sysinfo` crate](https://crates.io/crates/sysinfo)
- [`sysinfo` repo](https://github.com/GuillaumeGomez/sysinfo)

## Freshness signals

- `fs4 0.13.1` - repo `al8n/fs4-rs`, `106` stars, pushed `2026-04-03`, latest crate published `2025-03-08`
- `fd-lock 4.0.4` - repo `yoshuawuyts/fd-lock`, `85` stars, pushed `2026-03-09`, latest crate published `2025-03-10`
- `single-instance 0.3.3` - repo `WLBF/single-instance`, `44` stars, pushed `2026-04-14`, latest crate published `2021-12-16`
- `pidlock 0.2.2` - repo `rockstar/pidlock`, `5` stars, pushed `2026-02-26`, latest crate published `2025-10-17`
- `pidfile-rs 0.3.1` - repo `andrewshadura/pidfile-rs`, `2` stars, pushed `2025-10-03`, latest crate published `2025-10-03`
- `process_alive 0.2.0` - repo `caido/process_alive`, `8` stars, pushed `2025-10-21`, latest crate published `2025-10-21`
- `sysinfo 0.38.4` - repo `GuillaumeGomez/sysinfo`, `2668` stars, pushed `2026-04-16`, latest crate published `2026-03-09`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**daemon ownership should be modeled as a lock plus validated runtime state, not as a PID file and a prayer**

Healthiest shape сейчас выглядит так:

1. explicit ownership lock is the primary coordination primitive
2. owner publishes runtime state only after successful startup milestones
3. contenders validate the published state before trusting it
4. pid and liveness checks are adjunct signals, not sole truth
5. stale recovery is an ordered reclaim protocol, not startup cleanup folklore

## Top 3 directions for daemon ownership

### 1. `Lock-first ownership + published runtime state + validated reclaim protocol`

`🎯 10   🛡️ 9   🧠 8`
Примерно `7000-15000` строк.

Это strongest default.

Идея:

- one explicit ownership lock for daemon/workspace scope
- daemon publishes small runtime record only after bind/listen/open steps succeed
- contenders attempt connect or validate published state
- only after failed validation may they reclaim and rebuild runtime artifacts

Почему это лучший путь:

- ownership and liveness are separated cleanly
- stale socket and stale pid stop being the same thing
- fits protocol-first and recovery-first architecture
- works better for Electron restart, CLI attach and external hosts

### 2. `Lock-first ownership + adjunct pid/liveness helpers`

`🎯 8   🛡️ 8   🧠 7`
Примерно `5000-11000` строк.

Это good adjunct path.

Здесь useful:

- `process_alive` for narrow PID checks
- `sysinfo` for broader operational inspection
- maybe pid metadata for diagnostics and human support tooling

Но важно:

- this should enrich reclaim decisions
- it should not replace the primary lock/validate protocol

### 3. `Singleton helper or pidfile-first daemon ownership`

`🎯 4   🛡️ 5   🧠 5`
Примерно `3000-8000` строк на старт и потом дорого чинить.

Это weak default.

Типичные проблемы:

- ownership semantics follow platform quirks of helper crate
- liveness and authority get collapsed into one file or one mutex
- stale recovery policy becomes under-modeled
- multi-host and per-workspace reuse story gets awkward fast

## 1. `fs4` and `fd-lock` are the right class of primitives

`fs4` and `fd-lock` turned out to be valuable precisely because they are narrow.

### `fs4`

What it gives:

- cross-platform file locks
- boring reusable locking primitive
- async-capable ecosystem story through features

That makes it healthy for:

- ownership locks
- migration gates
- workspace-scoped coordination locks

### `fd-lock`

The README says something very important:

- advisory locks are opt-in
- they can be ignored by other parties
- the crate should never be used for security purposes

🔥 This is the correct mental model.

It means:

- ownership locks are coordination tools
- not authorization tools
- not identity proofs

That fits the rest of the package architecture extremely well.

## 2. `single-instance` is instructive, but too app-shaped for core daemon ownership

`single-instance 0.3.3` is useful mainly as a warning signal.

Its README openly says:

- Windows uses a named mutex
- Linux binds an abstract Unix socket
- macOS uses a file plus `flock`

This is convenient for:

- GUI app "allow only one window/process" behavior
- app startup convenience

But it is a weak center for reusable terminal daemon ownership because:

- Linux abstract socket is not the default security/ownership model we want
- semantics differ a lot by platform
- there is no rich published runtime state model
- attach/reclaim/startup phases remain implicit

Healthy role:

- host-app convenience leaf
- maybe standalone desktop app helper

Unhealthy role:

- foundation of the package's daemon ownership model

## 3. Pidfiles are useful metadata, but weak primary truth

This pass made the pidfile family much clearer.

### `pidlock`

`pidlock` is honest and practical:

- PID-based resource lock
- stale lock detection
- path validation

Good for:

- lightweight app-level singleton scenarios
- support tooling
- maybe auxiliary operator-facing state

But pidfile-first ownership still has structural weakness:

- PID alone is not enough runtime truth
- PID reuse exists
- a dead process and a half-published runtime are different failure modes

### `pidfile-rs`

This one is interesting because it explicitly references BSD pidfile semantics and even notes that `pidlock` does not actually use filesystem locks.

That is useful context, but still:

- pidfile discipline is narrower than full daemon lease protocol
- it does not by itself solve attach validation and stale endpoint reclaim

🔥 Strong rule:

**pidfile can be adjunct metadata, but should not be the canonical ownership contract**

## 4. `process_alive` and `sysinfo` belong to validation and diagnostics, not primary ownership

### `process_alive`

This crate has a healthy narrow purpose:

- check whether a specific PID is alive
- return `Unknown` when permissions or platform limitations intervene

That is exactly how such a tool should be used.

Good role:

- adjunct stale-owner heuristic
- operator diagnostics
- final confirmation step in reclaim flow

Bad role:

- the main source of truth for daemon ownership

### `sysinfo`

`sysinfo` is far heavier and broader.

Good role:

- diagnostics UI
- debug tooling
- support commands
- crash reports and observability

Bad role:

- being pulled into the hot startup/ownership path just to decide if we can bind a daemon

## 5. The healthy reclaim protocol

For this package the reclaim path should look more like:

1. acquire or contend for ownership lock
2. read published runtime state if present
3. try to validate it:
   - connect to endpoint
   - request handshake/ping/version
   - check expected scope/workspace/session identity
4. if validation fails, optionally consult adjunct liveness metadata
5. only then clean stale artifacts and rebuild

That is much healthier than:

- read pidfile
- if process dead, delete everything

or:

- socket path exists, therefore daemon exists

## 6. What should be published as runtime state

Published runtime state should stay tiny and explicit.

Healthy fields:

- daemon instance id
- startup epoch or boot id if available
- PID as adjunct metadata
- scope/workspace id
- endpoint identity
- protocol version
- maybe startup phase marker

Not healthy:

- giant mutable daemon state blob
- relying on modification timestamps as semantic truth

🔥 Ownership record should tell you how to validate, not try to replace validation.

## 7. Final verdict

The right question is not:

- "how do we ensure only one process runs?"

The right question is:

- "how do we coordinate one canonical runtime owner, let others validate it, and reclaim safely when it is stale?"

For your terminal package the strongest answer right now is:

- `fs4` or `fd-lock` class primitives for ownership coordination
- small published runtime record
- explicit validate-before-reclaim protocol
- `process_alive` only as adjunct heuristic
- `single-instance` and pidfile crates treated as donor/app helpers, not architecture center
