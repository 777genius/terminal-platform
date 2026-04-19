# Deep Dive - Rust Zero-Copy IPC, Shared Memory, and `mmap` Data Paths

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Когда проект доходит до ambitions уровня:

- world-class terminal runtime
- отдельный Rust daemon/core
- UI hosts на JS и других языках
- большие screen snapshots, replay tails и export surfaces

почти всегда появляется соблазн сказать:

- "давайте сделаем shared memory"
- "давайте дадим UI zero-copy доступ"
- "давайте сразу строить transport вокруг `mmap`"

🔥 Именно здесь очень легко перепутать:

- internal performance primitive
- bulk artifact publication
- true cross-host IPC contract

Для вашего terminal package это особенно опасно, потому что:

- universal embedding важнее одной локальной microbenchmark победы
- JS/Electron host не должен получать raw emulator memory как public truth
- cleanup, locks, fd transfer, sealing, stale handles и recovery быстро становятся частью архитектуры, если shared memory выбрать слишком рано

## Primary Sources

### Public transport baseline

- [`interprocess` crate](https://crates.io/crates/interprocess)
- [`interprocess` repo](https://github.com/kotauskas/interprocess)

### Memory mapping and sealed-memory primitives

- [`memmap2` crate](https://crates.io/crates/memmap2)
- [`memmap2` repo](https://github.com/RazrFalcon/memmap2-rs)
- [`memfd` crate](https://crates.io/crates/memfd)
- [`memfd` repo](https://github.com/lucab/memfd-rs)
- [`region` crate](https://crates.io/crates/region)
- [`region` repo](https://github.com/darfink/region-rs)
- [`RUSTSEC-2024-0394: mmap unmaintained`](https://rustsec.org/advisories/RUSTSEC-2024-0394.html)

### Shared-memory and ring-based IPC candidates

- [`shared_memory` crate](https://crates.io/crates/shared_memory)
- [`shared_memory` repo](https://github.com/elast0ny/shared_memory)
- [`shmem-ipc` crate](https://crates.io/crates/shmem-ipc)
- [`shmem-ipc` repo](https://github.com/diwic/shmem-ipc)
- [`ringbuf` crate](https://crates.io/crates/ringbuf)
- [`ringbuf` repo](https://github.com/agerasev/ringbuf)
- [`arc-swap` crate](https://crates.io/crates/arc-swap)
- [`arc-swap` repo](https://github.com/vorner/arc-swap)

## Freshness signals

- `interprocess 2.4.1` - repo `kotauskas/interprocess`, `551` stars, pushed `2026-04-18`
- `memmap2 0.9.10` - repo `RazrFalcon/memmap2-rs`, `619` stars, pushed `2026-04-18`, latest crate published `2026-02-15`
- `shared_memory 0.12.4` - repo `elast0ny/shared_memory`, `436` stars, pushed `2026-04-09`, latest crate published `2022-03-01`
- `shmem-ipc 0.3.0` - repo `diwic/shmem-ipc`, `163` stars, pushed `2026-03-19`, latest crate published `2022-11-22`
- `memfd 0.6.5` - repo `lucab/memfd-rs`, `41` stars, pushed `2026-04-14`, latest crate published `2025-09-01`
- `region 3.0.2` - repo `darfink/region-rs`, `138` stars, pushed `2026-04-06`, latest crate published `2024-03-25`
- `ringbuf 0.4.8` - repo `agerasev/ringbuf`, `553` stars, pushed `2026-04-11`, latest crate published `2025-03-24`
- `arc-swap 1.9.1` - repo `vorner/arc-swap`, `1313` stars, pushed `2026-04-17`, latest crate published `2026-04-04`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**zero-copy should stay an internal runtime optimization or a narrow bulk-artifact seam, not the primary public transport story**

Healthiest shape сейчас выглядит так:

1. control plane stays framed and explicit
2. normal hot terminal deltas still travel as deliberate runtime envelopes/chunks
3. `mmap` and `memfd` are used for bulk snapshots, spill blobs or optional acceleration lanes
4. Linux-only shared-memory fast paths stay optional capability leaves
5. hosts, especially JS hosts, still consume stable projections instead of raw mapped memory truth

## Top 3 directions for this layer

### 1. `Framed socket/control plane + mmap-backed bulk artifacts`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-12000` строк.

Это strongest default.

Идея:

- ordinary daemon/session control stays on framed local IPC
- hot session bytes and deltas still use explicit chunk lanes
- large snapshots, spill files and export blobs may use `memmap2`
- Linux may internally use `memfd` where sealed anonymous files are attractive

Почему это лучший путь:

- cross-platform story остаётся нормальной
- public contract stays protocol-first
- zero-copy optimization does not leak into JS/UI semantics
- crash recovery and cleanup stay much simpler than shared-memory-first designs

### 2. `Socket control plane + optional Linux memfd/shared-ring fast lane`

`🎯 8   🛡️ 7   🧠 9`
Примерно `9000-17000` строк.

Это strongest advanced path, если later profile data покажут real pressure:

- very large local snapshot transfer
- ultra-low-latency local attach/viewer flows
- local heavy binary side channels on Linux

Хороший shape здесь такой:

- primary control stays normal and portable
- Linux leaf may publish sealed memfd blobs or bounded shared rings
- capability handshake decides whether the lane exists

Почему это интересно:

- `memfd` gives safe sealed Linux memory-backed files
- `shmem-ipc` gives a serious donor for bounded shared-ring patterns
- it keeps the fast lane optional instead of making it the package identity

### 3. `Shared-memory-first cross-platform transport`

`🎯 4   🛡️ 5   🧠 9`
Примерно `8000-18000` строк.

Это плохой default.

Обычно здесь происходит следующее:

- shared memory layout becomes accidental public protocol
- synchronization and stale-resource cleanup infect core architecture
- host SDKs inherit platform-specific quirks
- recovery and security get harder before real user value appears

Особенно плохо это выглядит для Electron/JS host story:

- UI still needs typed events and lifecycle
- mapped buffers do not replace ownership, cancellation and attach semantics
- debugging and compatibility become worse, not better

## 1. `memmap2` looks like the right default `mmap` primitive

`memmap2 0.9.10` сейчас выглядит strongest boring default для memory mapping.

Что он явно даёт:

- cross-platform memory mapped IO
- file-backed maps
- anonymous maps
- copy-on-write maps
- sync and async flushing
- Linux-only huge-page support

Почему это особенно хорошо ложится на terminal runtime:

- durable snapshot blobs
- spill files for heavy replay/history adjuncts
- exported transcript/screen artifacts
- maybe read-mostly published screen snapshots

Что важно не перепутать:

- `memmap2` solves mapping and flush semantics
- it does **not** define multi-client IPC lifecycle
- it should not become the public session protocol

Ещё один сильный сигнал:

⚠️ `RUSTSEC-2024-0394` прямо говорит, что old `mmap` crate unmaintained, and points to `memmap2` as the main alternative.

## 2. `shared_memory` is useful as a low-level cross-process building block, but weak as the center

`shared_memory 0.12.4` interesting, but the shape matters.

README and docs make two things very clear:

- it is a thin shared-memory wrapper
- it expects you to combine it with sister synchronization primitives like `raw_sync`

Это полезный signal, а не недостаток:

- crate is honest about being a building block
- it does not pretend to solve the full runtime architecture

Почему я не ставлю его в strongest-default position:

- latest published crate line is still `2022-03-01`
- if this becomes the center, sync/ownership/cleanup burden moves into our core
- for a universal package we would still need a normal control plane, capability negotiation and recovery semantics around it

То есть healthiest role here is:

- optional infra primitive
- maybe for internal large-buffer sharing
- not the main public transport identity

## 3. `shmem-ipc` is a strong Linux acceleration donor, not a universal public contract

`shmem-ipc 0.3.0` is actually one of the most instructive crates in this pass because the README is very explicit.

It basically says:

- performance or latency is crucial
- you run Linux
- it uses memfd sealing
- signaling uses eventfd
- a separate channel is still needed to transfer file descriptors

🔥 This is excellent donor material precisely because it is so honest.

What it gives:

- wait-free/lock-free bounded SPSC shared ring patterns
- untrusted-process safety through memfd sealing
- realistic view of Linux-only best-case shared-memory IPC

What it does **not** give:

- cross-platform host-neutral package story
- a replacement for the control plane
- magical elimination of attach/replay/session truth problems

So the healthy role is:

- Linux fast-lane donor
- benchmark reference
- maybe optional local acceleration leaf

Not:

- universal default SDK transport

## 4. `memfd` is most valuable as a publication primitive

`memfd 0.6.5` is smaller in scope, but architecturally very valuable.

It gives:

- Linux memfd creation
- sealing support
- a safe way to publish memory-backed file data

This is most interesting for:

- immutable snapshot publication
- publish-once large artifact handoff
- safe sealed blob exchange across trusted runtime components

This is much healthier than pretending the UI should directly own mapped live runtime truth.

🔥 Strong pattern:

**use `memfd` to publish bulk immutable artifacts, not to redefine session semantics**

## 5. `region`, `ringbuf`, and `arc-swap` are support primitives, not the transport story

### `region`

`region 3.0.2` is a low-level virtual-memory API:

- alloc/protect/query/lock
- cross-platform virtual memory helpers

Useful if we later need:

- low-level memory experiments
- protection tricks
- explicit page/lock control

Not useful as the architecture center for this package.

### `ringbuf`

`ringbuf 0.4.8` remains strong, but its healthiest role here is:

- in-process SPSC staging
- hot replay tail
- local serialization/output staging

It should not be confused with a complete cross-process transport contract.

### `arc-swap`

`arc-swap 1.9.1` is even more clearly not a data-plane tool.

It is good for:

- atomically swapping read-mostly immutable snapshots
- config/policy/runtime view replacement
- stable read-side access with rare writes

Great for:

- policy snapshots
- published read models
- maybe pointer swaps to current immutable screen projection metadata

Not for:

- main IPC transport
- giant binary session payload movement

## 6. What this means for the package architecture

If we compress the whole pass into one practical rule set, it looks like this:

### Public truth

- framed local daemon protocol
- typed control plane
- explicit hot delta/snapshot chunk lanes

### Internal performance tools

- `memmap2` for bulk artifact mapping
- `memfd` for sealed Linux publication
- `ringbuf` for hot in-process bounded tails
- `arc-swap` for immutable snapshot swapping

### Optional platform leaves

- `shmem-ipc` style Linux acceleration lane
- maybe `shared_memory`-based experiments only behind explicit capability flags

## Final verdict

🔥 The right question is not:

- "can we make IPC zero-copy?"

The right question is:

- "which copies are actually on the critical path, and can we remove them without turning memory layout into public architecture?"

Для вашего terminal package healthiest answer сейчас такой:

- keep protocol-first daemon boundary
- keep JS/UI on typed projections
- use `mmap`/`memfd` only where bulky immutable data really benefits
- treat shared-memory-first designs as later optimization leaves, not as v1 identity
