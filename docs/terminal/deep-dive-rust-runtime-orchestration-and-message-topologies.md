# Deep Dive - Rust Runtime Orchestration, Supervision, And Message Topologies

**Проверено**: 2026-04-19

## Зачем этот deep dive

После выбора:

- PTY adapter
- emulator core
- protocol boundary
- persistence primitives

остаётся ещё один опасный слой:

**как именно runtime будет владеть состоянием и координировать работу**

Именно здесь terminal platforms чаще всего скатываются в:

- giant `Arc<DashMap<...>>`
- случайный event bus
- смешивание command, state, events и PTY bytes
- implicit supervision
- host-specific lifecycle hacks

Для reusable package мирового уровня это уже не детали реализации.
Это фундамент embed-story.

## Primary Sources

### Runtime and actor frameworks

- [`tokio` README](https://github.com/tokio-rs/tokio/blob/master/README.md)
- [`tokio::sync::mpsc` docs](https://docs.rs/tokio/1.52.1/tokio/sync/mpsc/)
- [`tokio::sync::oneshot` docs](https://docs.rs/tokio/1.52.1/tokio/sync/oneshot/)
- [`tokio::sync::watch` docs](https://docs.rs/tokio/1.52.1/tokio/sync/watch/)
- [`tokio::sync::broadcast` docs](https://docs.rs/tokio/1.52.1/tokio/sync/broadcast/)
- [`ractor` README](https://github.com/slawlor/ractor/blob/main/README.md)
- [`ractor` runtime semantics](https://github.com/slawlor/ractor/blob/main/docs/runtime-semantics.md)
- [`xtra` README](https://github.com/Restioson/xtra/blob/master/README.md)

### Channels and signaling

- [`async-broadcast` README](https://github.com/smol-rs/async-broadcast/blob/master/README.md)
- [`event-listener` README](https://github.com/smol-rs/event-listener/blob/master/README.md)
- [`bus` README](https://github.com/jonhoo/bus/blob/master/README.md)
- [`kanal` README](https://github.com/fereidani/kanal/blob/master/README.md)
- [`postage` README](https://github.com/austinjones/postage-rs/blob/main/README.md)
- [`crossbeam-channel` README](https://github.com/crossbeam-rs/crossbeam/blob/master/crossbeam-channel/README.md)

### Concurrent state containers

- [`dashmap` README](https://github.com/xacrimon/dashmap/blob/master/README.md)
- [`scc` README](https://github.com/wvwwvwwv/scalable-concurrent-containers/blob/main/README.md)

## Freshness signals

### Runtime / actor layer

- `tokio 1.52.1` - repo `31.7k` stars, pushed `2026-04-18`
- `tokio-util 0.7.18` - same Tokio repo, pushed `2026-04-18`
- `ractor 0.15.12` - repo `1992` stars, pushed `2026-03-29`
- `xtra 0.6.0` - repo `360` stars, pushed `2024-11-16`

### Channels / signaling

- `crossbeam-channel 0.5.15` - repo `8408` stars, pushed `2026-02-22`
- `async-channel 2.5.0` - repo `936` stars, pushed `2025-07-06`
- `async-broadcast 0.7.2` - repo `191` stars, pushed `2025-08-19`
- `event-listener 5.4.1` - repo `505` stars, pushed `2026-02-07`
- `kanal 0.1.1` - repo `1708` stars, pushed `2025-12-18`
- `postage 0.5.0` - repo `266` stars, pushed `2023-01-09`
- `bus 2.4.1` - repo `838` stars, pushed `2023-09-09`

### Concurrent containers

- `dashmap 7.0.0-rc2` - repo `4018` stars, pushed `2025-03-05`
- `scc 3.7.0` - repo `502` stars, pushed `2026-01-05`, MSRV `1.85.0`

## Короткий вывод

🔥 Для embeddable Rust terminal runtime лучший default сейчас выглядит так:

- **explicit owner-task orchestration on Tokio**
- **разные lanes для command, state, events и byte-stream**
- **actor frameworks только там, где реально нужна supervision tree semantics**
- **concurrent maps only as adjunct indexes, not as source of truth**

То есть правильный runtime shape ближе к:

- `SessionRuntime` owns session truth
- `PaneRuntime` owns pane-local projection state
- `WorkstreamRuntime` owns topology and lifecycle
- host talks through typed protocol
- observers subscribe through deliberately chosen lanes

а не к:

- one global concurrent map
- one event bus
- one actor framework everywhere

## Top 3 Orchestration Directions

### 1. Explicit Tokio owner-tasks with typed channel lanes

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк.

Это мой текущий **лучший default**.

Идея:

- каждый session/workstream/runtime object имеет своего owner task
- commands идут через bounded `mpsc`
- direct replies идут через `oneshot`
- latest-state projections идут через `watch`
- fanout events идут через `broadcast` или `async-broadcast`
- PTY byte/data plane живёт отдельно от control/event plane

Почему это strongest path:

- идеально ложится на DDD + Ports/Adapters
- ownership model остаётся явной
- легко строить deterministic lifecycle
- host-neutral protocol поверх этого очень естественный
- не тащит Erlang-style semantics туда, где они не нужны

Главный плюс:

🔥 это даёт actor-like safety without actor-framework lock-in

### 2. `ractor` for supervision-heavy runtime islands

`🎯 7   🛡️ 8   🧠 8`  
Примерно `7000-14000` строк.

Когда это реально интересно:

- нужен явный supervision tree
- нужен actor registry
- нужны process groups
- хочется message-priority semantics and explicit stop/kill behavior
- есть шанс, что later появится distributed/runtime-cluster story

Что в нём сильного:

- single-message processing
- supervisor-friendly design
- runtime semantics явно задокументированы
- нет requirement поднимать отдельный global system

Почему это не мой default:

- terminal runtime не обязан быть Erlang-like platform everywhere
- при неаккуратном использовании это принесёт больше semantic weight, чем продукта
- message priorities and supervision semantics становятся архитектурным центром, even where a simple owner task would be healthier

### 3. `xtra` or mixed lightweight actor islands

`🎯 6   🛡️ 6   🧠 6`  
Примерно `4000-9000` строк.

Когда это имеет смысл:

- нужен маленький actor abstraction для isolated subsystem
- хочется runtime-agnostic story
- нужно быстро закапсулировать несколько stateful workers

Что нравится:

- tiny and safe
- no dedicated runtime
- nice fit for leaf subsystems

Почему это не foundation choice:

- supervision and lifecycle model weaker than in `ractor`
- ecosystem gravity for serious service/runtime orchestration ниже, чем у explicit Tokio design
- легко получить hybrid architecture, где half-runtime живёт на messages, half-runtime на ad-hoc tasks

## 1. Best default - owner-task orchestration on Tokio

`tokio` сам себя позиционирует как runtime for reliable, asynchronous and scalable applications.
Для terminal package это важно не только из-за I/O, а из-за shape всей orchestration model.

### Почему именно owner-task model

Terminal runtime naturally has bounded state owners:

- terminal session
- pane/surface
- workstream
- runtime transport connection
- snapshot/durable mirror worker

Если у каждой такой сущности есть явный владелец, тогда:

- state mutation serializes naturally
- lifecycle becomes explicit
- stale handles become easier to reject
- supervision can be local and deliberate

Это гораздо здоровее, чем shared mutable registry with ambient locks.

## 2. Separate lanes by semantics, not by convenience

🔥 Один из самых важных выводов этого прохода:

**different traffic classes should not share one generic channel**

Правильный разрез выглядит так.

### Command lane

Что это:

- launch session
- resize
- send input
- request snapshot
- attach viewer

Чем возить:

- bounded `mpsc`
- optional `oneshot` for reply

Почему:

- commands are ordered
- backpressure is healthy
- caller usually wants success/failure, not fanout

### State lane

Что это:

- current phase
- attached/detached
- health
- current cwd/title/profile metadata

Чем возить:

- `watch`

Почему:

- receivers need the **latest** state
- they do not need every intermediate transition forever

### Event lane

Что это:

- session exited
- prompt marker detected
- unread attention event
- new timeline node committed

Чем возить:

- `broadcast` or `async-broadcast`

Почему:

- every active subscriber may need to see the event
- ephemeral fanout semantics differ from latest-state semantics

### Byte/data plane

Что это:

- PTY output bytes
- replay chunks
- snapshot chunks

Чем возить:

- dedicated framed stream
- not your generic app event bus

Почему:

- throughput, backpressure and recovery semantics are different here

## 3. `async-broadcast` vs Tokio `broadcast`

`async-broadcast` оказался интереснее, чем выглядит на первый взгляд.

Что у него полезно:

- MPMC broadcast with sender/receiver split
- every receiver gets every message via cloning
- explicit inactive receiver model
- overflow mode support

Это делает его реальным кандидатом для:

- viewer fanout
- attention event fanout
- bounded non-truth event channels

Но важный practical point:

⚠️ если runtime core уже целиком на Tokio, то `tokio::broadcast` and `tokio::watch` usually give a simpler ecosystem story.

Итог:

- use Tokio sync primitives by default
- reach for `async-broadcast` only where its inactive/overflow semantics are actually useful

## 4. Actor frameworks - when they help and when they hurt

## `ractor`

`ractor` прямо документирует:

- single-message processing
- supervision tree
- stop vs kill difference
- priority channels
- registry and process groups

Это очень сильный набор, если вы строите:

- daemon coordinator
- remote runtime controller
- restart-heavy worker topology

Но для default terminal session runtime есть risk:

⚠️ the framework semantics start shaping your product model

В terminal package мирового уровня supervision должна быть deliberate domain decision, not ambient framework gravity.

### Practical recommendation for `ractor`

Использовать не как universal base, а как **optional runtime island** for:

- daemon supervisor
- remote transport controller
- maybe plugin/sidecar manager later

## `xtra`

`xtra` хорош тем, что он:

- tiny
- safe
- runtime-agnostic
- ergonomic for small stateful workers

Это делает его симпатичным для leaf subsystems.
Но reusable terminal platform usually needs stronger lifecycle guarantees than "tiny actor wrapper".

Practical recommendation:

- okay for narrow internal subsystem
- weak as foundation of the whole runtime platform

## 5. Concurrent maps - powerful but dangerous as truth model

## `dashmap`

`dashmap` честно продаёт себя как direct replacement for `RwLock<HashMap<K, V>>`.
Для tactical indices это может быть удобно.

Но для нашей архитектуры есть 2 проблемы:

- current published line is `7.0.0-rc2`
- ergonomic shared mutability easily becomes architecture by accident

Практический вывод:

⚠️ `DashMap` is acceptable for adjunct indexes and host-facing lookup caches  
❌ bad as primary owner of terminal session truth

## `scc`

`scc` технически очень сильный:

- concurrent containers
- sync and async interfaces
- non-blocking resize
- write-heavy focus

Но practical caveats for our package:

- much heavier mental model
- MSRV `1.85.0`
- still the wrong answer if what you really need is owner-local truth

Итог:

- `scc` interesting for specialized indices
- not a substitute for an ownership model

## 6. Alternative channel stacks

## `kanal`

Интересен из-за:

- sync + async APIs in one crate
- performance-oriented design

Но для world-class reusable package я бы не делал его core default:

- smaller ecosystem gravity than Tokio
- more custom internals in a place where boring is good

## `postage`

Сильная идея:

- rich set of channels
- executor portability
- watch/broadcast/dispatch in one family

Проблема практическая:

- repo выглядит менее живым
- ecosystem center of gravity for daemon-like Rust runtime не на его стороне

## `bus`

`bus` сам в README предупреждает:

- single-producer only
- current implementation may busy-wait and increase CPU usage

Для terminal runtime это сразу делает его weak fit.

## 7. `event-listener` - useful, but one layer lower

`event-listener` is genuinely good, but it solves a lower-level problem:

- notify async tasks or threads
- turn non-blocking structures into async/blocking ones

Это отличный building block для:

- custom queue internals
- low-level synchronization
- hand-rolled concurrent primitives

Но:

⚠️ it is not your business-domain orchestration model

## Recommended orchestration blueprint

```text
Host / UI
  -> typed control protocol
  -> daemon/session router

WorkstreamRuntime owner task
  -> owns topology truth
  -> spawns SessionRuntime owners
  -> maintains attach/viewer bindings

SessionRuntime owner task
  -> owns session truth
  -> command inbox: mpsc
  -> state outlet: watch
  -> event outlet: broadcast
  -> byte plane: dedicated framed stream

Adjunct workers
  -> durable mirror worker
  -> analyzer worker
  -> snapshot exporter
  -> transport bridge
```

## What I would choose now

Если выбирать прямо сейчас для universal Rust terminal package:

1. **Tokio owner-task runtime as the default orchestration model**
2. **Tokio `mpsc/oneshot/watch/broadcast` as primary lane primitives**
3. **`crossbeam-channel` only where blocking thread boundaries are genuinely cleaner**
4. **`ractor` only for isolated supervision-heavy islands**
5. **`DashMap`/`scc` only for secondary indices, never as truth owners**

## Final architectural rule

🔥 **Do not let synchronization primitives become your domain model.**

Для такого terminal package domain model должен быть:

- sessions
- panes
- workstreams
- viewers/controllers
- snapshots
- timeline/events

А channels, actor frameworks и concurrent maps должны оставаться только means of orchestration.

