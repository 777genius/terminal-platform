# Deep Dive - Rust State Directories, Atomic Writes, Locks, and Crash Consistency

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для embeddable terminal package мирового уровня мало выбрать:

- PTY layer
- emulator core
- durable store
- protocol/data plane

Нужно ещё правильно переживать very boring, but deadly things:

- partially written config or snapshot files
- stale daemon sockets and pid/lock files
- two hosts trying to bind the same session or workspace state
- crash between temp write and final publish
- restart after interrupted migration, replay flush or snapshot update

🔥 Именно тут ломаются многие "почти хорошие" local runtimes:

- state directories размазаны строками по коду
- write path неотличим от publish path
- locks появляются ad hoc
- socket cleanup полагается на удачный shutdown
- corruption recovery живёт в комментариях, а не в architecture

Для reusable multi-host package этот слой обязан быть first-class.

## Primary Sources

### Paths and state directories

- [`directories` crate](https://crates.io/crates/directories)
- [`directories-rs` repo](https://github.com/dirs-dev/directories-rs)
- [`camino` crate](https://crates.io/crates/camino)
- [`camino` repo](https://github.com/camino-rs/camino)

### Atomic writes and temp staging

- [`tempfile` crate](https://crates.io/crates/tempfile)
- [`tempfile` repo](https://github.com/Stebalien/tempfile)
- [`atomic-write-file` crate](https://crates.io/crates/atomic-write-file)
- [`rust-atomic-write-file` repo](https://github.com/andreacorbellini/rust-atomic-write-file)
- [`fs-err` crate](https://crates.io/crates/fs-err)
- [`fs-err` repo](https://github.com/andrewhickman/fs-err)

### File locks and socket/IPC boundaries

- [`fs4` crate](https://crates.io/crates/fs4)
- [`fs4-rs` repo](https://github.com/al8n/fs4-rs)
- [`fd-lock` crate](https://crates.io/crates/fd-lock)
- [`fd-lock` repo](https://github.com/yoshuawuyts/fd-lock)
- [`interprocess` crate](https://crates.io/crates/interprocess)
- [`interprocess` repo](https://github.com/kotauskas/interprocess)

### Durable stores touched by crash policy

- [`rusqlite` crate](https://crates.io/crates/rusqlite)
- [`rusqlite` repo](https://github.com/rusqlite/rusqlite)
- [`redb` crate](https://crates.io/crates/redb)
- [`redb` repo](https://github.com/cberner/redb)

## Freshness signals

- `tempfile 3.27.0` - repo `Stebalien/tempfile`, `1425` stars, pushed `2026-03-14`
- `fs4 0.13.1` - repo `al8n/fs4-rs`, `106` stars, pushed `2026-01-24`
- `directories 6.0.0` - repo `dirs-dev/directories-rs`, `834` stars, pushed `2025-01-12`
- `camino 1.2.2` - repo `camino-rs/camino`, `554` stars, pushed `2026-03-31`
- `fs-err 3.3.0` - repo `andrewhickman/fs-err`, `176` stars, pushed `2026-02-07`
- `atomic-write-file 0.3.0` - repo `andreacorbellini/rust-atomic-write-file`, `33` stars, pushed `2025-09-11`
- `fd-lock 4.0.4` - repo `yoshuawuyts/fd-lock`, `85` stars, pushed `2025-04-23`
- `interprocess 2.4.1` - repo `kotauskas/interprocess`, `551` stars, pushed `2026-04-18`
- `rusqlite 0.39.0` - repo `rusqlite/rusqlite`, `4148` stars, pushed `2026-04-19`
- `redb 4.0.0` - repo `cberner/redb`, `4420` stars, pushed `2026-04-19`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**filesystem hygiene must be a separate infrastructure layer, not an accidental side effect of whichever store or daemon transport you picked**

Healthiest shape сейчас выглядит так:

1. one authority for state-directory layout
2. one atomic publish path for file-like artifacts
3. one explicit lock strategy for singleton/session ownership
4. one clear separation between transient socket/runtime artifacts and durable truth
5. crash recovery rules designed as protocols, not lucky restarts

То есть не:

- "SQLite уже durable, значит всё остальное неважно"

и не:

- "сделаем temp file и как-нибудь rename"

а:

- `StatePathPolicy`
- `AtomicPublishPort`
- `LockPort`
- `RuntimeArtifactCleaner`
- store-specific durability below that

## Top 3 directions for crash-safe filesystem hygiene

### 1. `Directories + Camino + Tempfile/atomic publish + fs4 locks + interprocess runtime artifacts`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- `directories` decides platform-appropriate roots
- `camino` keeps path handling UTF-8-clean inside the package
- `tempfile` or `atomic-write-file` handle staged writes
- `fs4` handles explicit lock files for ownership
- `interprocess` owns socket/IPC artifact semantics
- `rusqlite` or `redb` still keep their own internal durability rules

Почему это лучший путь:

- clear separation of concerns
- host-neutral path policy
- atomic publication becomes reusable and testable
- lock semantics stay visible
- daemon/socket cleanup stops leaking into random code paths

### 2. `Store-centric durability with minimal outer hygiene`

`🎯 6   🛡️ 6   🧠 5`
Примерно `4000-9000` строк.

Это путь, где команда почти всё доверяет store и пишет немного glue вокруг:

- SQLite WAL
- a couple of socket files
- ad hoc temp writes
- maybe a pid file

Почему это иногда работает:

- fewer moving parts
- fast to start
- simpler for a very small product

Почему это слабый default:

- non-store artifacts still need correctness
- stale sockets and partial snapshots stay under-modeled
- path layout and lock ownership drift across modules

### 3. `One giant runtime directory with ad hoc files and cleanup`

`🎯 2   🛡️ 3   🧠 4`
Примерно `3000-7000` строк.

Это плохой путь.

Симптомы:

- stringly-typed paths
- random lock files
- rename and overwrite semantics vary by code path
- startup cleanup tries to "guess" what is stale
- different hosts accidentally share too much state

## 1. `directories` should own where things live

`directories 6.0.0` всё ещё выглядит самым здоровым default для platform-specific roots.

Для такого пакета особенно важно, что runtime likely needs multiple classes of local artifacts:

- config
- durable metadata
- cache
- spill blobs
- daemon sockets
- temp publish staging
- logs and diagnostics

🔥 Strong rule:

**never let individual subsystems invent their own root directories**

Instead:

- one `StatePathPolicy`
- explicit subdirectories by artifact class
- route/session/workspace naming policy kept centralized

Это особенно важно для later multi-host embedding, where:

- Electron host
- standalone CLI app
- external SDK user

must all reason about the same path conventions.

## 2. `camino` is a very healthy internal path discipline tool

`camino 1.2.2` не решает durability, but it strongly improves correctness surface.

Почему это важно:

- terminal package will surface lots of paths
- path normalization bugs become host-facing bugs
- config/state layout code gets noisy very fast with raw `PathBuf`

Здоровый вывод:

- use UTF-8 paths internally where product policy expects text paths
- keep raw OS path edges at adapters

⚠️ Но это именно internal discipline tool, not a public protocol answer.

## 3. `tempfile` and `atomic-write-file` belong to publish semantics, not just tests

После более внимательного просмотра стало ясно:

### `tempfile`

Лучший general-purpose brick, когда нужен:

- temp staging
- same-filesystem finalization
- safe cleanup semantics

### `atomic-write-file`

Очень интересен как narrow-purpose helper, когда нужно именно:

- write temp content
- publish atomically to final path

🔥 Practical rule:

**write path and publish path must be separate concepts**

То есть:

- build bytes
- write to staged location
- fsync/flush as needed by artifact type
- publish atomically
- only then update durable pointer/metadata if needed

Это касается:

- snapshots
- exported transcript chunks
- manifest-like files
- durable projections

## 4. `fs4` looks like the strongest default for lock files

`fs4 0.13.1` сейчас выглядит наиболее здоровым default для explicit lock semantics.

Почему он хорош:

- cross-platform story
- not tied to giant framework worldview
- fits the layer we actually need

Где он нужен:

- singleton daemon ownership
- per-workspace runtime ownership
- migration gate around one state directory
- coordination when host and sidecar may race

### `fd-lock`

`fd-lock 4.0.4` выглядит как valid alternative/watchlist, but weaker default here.

Почему:

- narrower ecosystem gravity
- less compelling as the central lock story for this package

Здоровый practical rule:

- choose one lock abstraction and make lock scope explicit
- do not let every subsystem create its own locking idiom

## 5. `interprocess` should own socket artifacts, not your random utility module

`interprocess 2.4.1` already emerged earlier as the strongest local IPC default.

This pass makes its filesystem role clearer too.

Daemon-based runtime likely needs:

- local socket files or names
- cleanup policy
- stale endpoint detection
- separation between durable truth and ephemeral runtime endpoints

🔥 Very important distinction:

- sockets are runtime artifacts
- databases and snapshots are durable truth artifacts

These should not share the same semantics.

Practical implication:

- ephemeral socket dir should be separated from durable state dir
- startup should validate endpoint ownership, not blindly delete
- recovery should be able to rebuild runtime artifacts from durable truth

## 6. Store durability and outer artifact hygiene are different concerns

This is where many designs get muddy.

`rusqlite` and `redb` can both be strong for durable truth.

But even a perfect store does **not** solve:

- stale socket path
- partial non-store snapshot write
- bad lock cleanup
- inconsistent artifact naming
- racing hosts attaching to the same session directory

🔥 Strong rule:

**store durability is not filesystem hygiene**

That means:

- durable store rules stay inside store adapter
- outer artifact publish/lock/cleanup rules stay in a separate infra layer

## 7. Crash consistency should be modeled as restart protocol

For this package, crash consistency should likely mean:

1. discover roots
2. acquire required ownership locks
3. open durable truth
4. validate or recreate ephemeral runtime artifacts
5. rebuild projections if generation says so
6. only then expose routes/attach points

This is much healthier than:

- "start everything and clean up whatever seems broken"

⚠️ Especially for an embeddable package, startup and restart must be deterministic.

## Practical verdict

If I were designing this layer right now:

### V1

- `directories` for root resolution
- `camino` for internal UTF-8 path discipline
- `tempfile` as general staging primitive
- optional `atomic-write-file` for narrow file-publish helper cases
- `fs4` for explicit lock ownership
- `interprocess` for daemon/runtime artifacts
- `rusqlite` as structured truth and `redb` only where separately justified

### V2

- stronger stale-artifact validation rules
- explicit artifact-class subdirectories
- crash-recovery integration tests over lock/socket/store combinations
- maybe dual-store adjuncts where replay/spill really need them

## Чего я бы избегал

- ❌ Letting each subsystem invent paths ad hoc
- ❌ Treating socket files like durable truth
- ❌ Publishing directly to final files without staging
- ❌ Multiple unrelated lock mechanisms in one package
- ❌ Assuming durable DB choice automatically solves outer artifact consistency

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- path layout needs a dedicated policy object
- atomic publish should be a reusable port
- locks should be explicit and centralized
- runtime artifacts and durable artifacts must live in different categories
- restart should be modeled as recovery protocol, not best-effort cleanup

## Sources

- [directories crate](https://crates.io/crates/directories)
- [directories-rs repo](https://github.com/dirs-dev/directories-rs)
- [camino crate](https://crates.io/crates/camino)
- [camino repo](https://github.com/camino-rs/camino)
- [tempfile crate](https://crates.io/crates/tempfile)
- [tempfile repo](https://github.com/Stebalien/tempfile)
- [atomic-write-file crate](https://crates.io/crates/atomic-write-file)
- [rust-atomic-write-file repo](https://github.com/andreacorbellini/rust-atomic-write-file)
- [fs-err crate](https://crates.io/crates/fs-err)
- [fs-err repo](https://github.com/andrewhickman/fs-err)
- [fs4 crate](https://crates.io/crates/fs4)
- [fs4-rs repo](https://github.com/al8n/fs4-rs)
- [fd-lock crate](https://crates.io/crates/fd-lock)
- [fd-lock repo](https://github.com/yoshuawuyts/fd-lock)
- [interprocess crate](https://crates.io/crates/interprocess)
- [interprocess repo](https://github.com/kotauskas/interprocess)
- [rusqlite crate](https://crates.io/crates/rusqlite)
- [rusqlite repo](https://github.com/rusqlite/rusqlite)
- [redb crate](https://crates.io/crates/redb)
- [redb repo](https://github.com/cberner/redb)
