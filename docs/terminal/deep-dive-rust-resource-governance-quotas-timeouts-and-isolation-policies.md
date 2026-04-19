# Deep Dive - Rust Resource Governance, Quotas, Timeouts, and Isolation Policies

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для terminal package мирового уровня мало иметь:

- PTY abstraction
- emulator core
- replay/snapshot
- process supervision
- backpressure and spill policy

Нужно ещё явно решить:

- кто ставит session-level time budget
- кто ограничивает attach/search/control storms
- кто владеет memory and output quotas
- где проходят outer OS isolation boundaries
- что является core runtime truth, а что deployment-specific enforcement

🔥 Если этого слоя нет отдельно, архитектура быстро деградирует в смесь:

- случайных timeout-ов
- ad hoc rate limiters
- platform-specific `rlimit` calls
- systemd/cgroup assumptions в core runtime
- host-specific "safety wrappers", которые становятся настоящей policy truth

Для reusable embeddable package это особенно опасно, потому что:

- один host хочет local desktop embedding
- другой хочет standalone terminal app
- третий хочет managed daemon or remote worker
- четвёртый вообще запускает runtime внутри larger product shell

У всех этих host-ов разные outer authority and isolation constraints.

## Primary Sources

### Async runtime and cancellation

- [`tokio` crate](https://crates.io/crates/tokio)
- [`tokio-util` crate](https://crates.io/crates/tokio-util)
- [`tokio-util` sync docs](https://docs.rs/tokio-util/latest/tokio_util/sync/)
- [`tokio-util` task docs](https://docs.rs/tokio-util/latest/tokio_util/task/)

### Rate limiting and local throttles

- [`governor` crate](https://crates.io/crates/governor)
- [`governor` repo](https://github.com/boinkor-net/governor)

### Resource limits and OS-specific enforcement

- [`rlimit` crate](https://crates.io/crates/rlimit)
- [`rlimit` repo](https://github.com/Nugine/rlimit)
- [`cgroups-rs` crate](https://crates.io/crates/cgroups-rs)
- [`cgroups-rs` repo](https://github.com/kata-containers/cgroups-rs)
- [`systemd-run` crate](https://crates.io/crates/systemd-run)
- [`rust-systemd-run` repo](https://github.com/xdu-icpc/rust-systemd-run)

### Process policy donor

- [`process-wrap` crate](https://crates.io/crates/process-wrap)
- [`process-wrap` repo](https://github.com/watchexec/process-wrap)
- [`process-wrap` README](https://github.com/watchexec/process-wrap/blob/main/README.md)

## Freshness signals

- `tokio 1.52.1` - repo `tokio-rs/tokio`, `31.7k` stars, pushed `2026-04-18`
- `tokio-util 0.7.18`
- `process-wrap 9.1.0` - repo `watchexec/process-wrap`, `43` stars, pushed `2026-04-18`
- `governor 0.10.4` - repo `boinkor-net/governor`, `900` stars, pushed `2026-02-09`
- `rlimit 0.11.0` - repo `Nugine/rlimit`, `58` stars, pushed `2026-04-01`
- `cgroups-rs 0.5.0` - repo `kata-containers/cgroups-rs`, `43` stars, pushed `2026-04-18`
- `systemd-run 0.9.0` - repo `xdu-icpc/rust-systemd-run`, `2` stars, pushed `2025-11-21`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**resource governance - это отдельный bounded context, не синоним backpressure и не синоним process supervision**

Healthiest shape сейчас выглядит так:

1. runtime owns explicit budgets, deadlines and policy decisions
2. process supervision owns lifecycle semantics
3. backpressure owns output-flow and retention pressure
4. OS limiters like `rlimit`, `cgroups`, `systemd-run` stay optional infrastructure leaves
5. host/deployer decides which outer isolation leaves are even available

То есть не:

- "давайте просто повесим пару timeout-ов"

и не:

- "cgroups всё решат"

а:

- `ResourceGovernancePolicy`
- `ProcessSupervisionPolicy`
- `BackpressurePolicy`
- optional `OsIsolationPort`

## Top 3 directions for Rust resource governance

### 1. `Explicit runtime governance policy + Tokio deadlines/cancellation + process-wrap-inspired supervision + optional OS leaves`

`🎯 10   🛡️ 9   🧠 8`
Примерно `7000-15000` строк.

Это strongest default.

Идея:

- runtime keeps explicit per-session/workstream budgets
- owner tasks enforce operation deadlines and cancellation
- process supervision stays visible and composable
- OS-level enforcement is optional and host-selected
- governance policy becomes typed domain configuration, not scattered helpers

Почему это лучший путь:

- переносим policy truth в runtime core
- не тащим Linux/systemd assumptions в universal package
- time and quota semantics становятся testable and portable
- host adapters can stay thin

Что сюда особенно хорошо ложится:

- `tokio::time::timeout`
- `tokio::sync::Semaphore`
- `tokio_util::sync::CancellationToken`
- `tokio_util::task::TaskTracker`
- `process-wrap` как donor для explicit supervision wrappers
- optional `rlimit` / `cgroups-rs` / `systemd-run` only at outer leaves

### 2. `Unix/Linux-heavy enforcement through rlimit + cgroups + systemd-run`

`🎯 7   🛡️ 7   🧠 8`
Примерно `8000-16000` строк.

Это сильный path для:

- standalone desktop terminal
- controlled Linux appliance/runtime
- managed daemon deployments

Почему он интересен:

- `rlimit` даёт direct Unix resource limits
- `cgroups-rs` даёт real Linux control groups
- `systemd-run` даёт transient unit boundary for supervised execution

Почему это не strongest universal default:

- too platform-shaped
- embeddable story for arbitrary hosts gets worse
- host apps cannot assume Linux, cgroup v2, or systemd
- easy to turn infrastructure capabilities into accidental core truth

### 3. `Helper-first governance via rate limiting and ad hoc counters`

`🎯 4   🛡️ 5   🧠 5`
Примерно `4000-9000` строк.

Это путь, где команда делает ставку примерно на:

- `governor`
- a few timeouts
- hand-rolled counters
- host-level guesses about overload

Почему это выглядит привлекательно в начале:

- cheap to start
- easy to explain
- no big policy model upfront

Почему это плохой default:

- rate limiting is not session governance
- CPU/memory/time budgets stay implicit
- behavior becomes inconsistent across routes and hosts
- hard to reason about detach/attach/replay under pressure

## 1. `Tokio` and `tokio-util` are the real core bricks for governance

После более внимательного просмотра current Tokio stack стало особенно ясно:

- time budgets should be explicit
- cancellation should be explicit
- grouped shutdown should be explicit
- long-lived subscriptions and short-lived operations should not share one lifecycle shape

### Что особенно полезно

- `tokio::time::timeout` and deadline-driven waiting
- `Semaphore` for bounded concurrent expensive operations
- `CancellationToken` for session or subtree cancellation
- `TaskTracker` for graceful shutdown and ownership-aware cleanup

🔥 Очень важный practical rule:

**resource governance needs owner-task semantics**

Не просто:

- "где-то есть timeout"

А:

- session owner decides if operation may start
- session owner decides when operation expires
- session owner decides what gets cancelled on detach/shutdown

Это почти идеально совпадает с уже установленной нами моделью:

- owner-task runtime
- explicit lanes
- explicit budgets

## 2. `process-wrap` is a strong donor, but not the governance center

`process-wrap 9.1.0` ещё раз оказался сильным, но важно правильно понимать его роль.

Он отлично показывает:

- one concern per wrapper
- visible process/session/job policy
- explicit kill-on-drop semantics
- separation of launch policy from business logic

Но governance в более широком смысле он **не закрывает**.

Он не отвечает сам по себе на:

- attach flood control
- semantic-analyzer quotas
- replay catch-up deadline policy
- search/index CPU budgeting
- durable mirror/spill budget decisions

То есть healthiest interpretation now:

- `process-wrap` helps model launch/supervision boundaries
- runtime still needs its own `ResourceGovernancePolicy`

## 3. `governor` is useful, but narrow

`governor 0.10.4` стоит держать, но не переоценивать.

Он реально полезен для:

- attach/control API burst smoothing
- maybe external search/export endpoints
- host-triggered side operations
- multi-tenant request fairness on outer surfaces

Но это **не** главный ответ для terminal resource governance.

Почему:

- runaway PTY output is not solved by token buckets
- stuck child process is not solved by rate limiting
- replay and scrollback memory policy are not solved by request throttling
- session CPU budget and operation deadline are different concerns

🔥 Practical rule:

**rate limiting should sit on outer API lanes, not replace runtime governance**

## 4. `rlimit` is a useful Unix leaf, not the public contract

`rlimit 0.11.0` выглядит достаточно свежо и честно полезно, но only in a narrow place.

Где он хорош:

- standalone Unix host
- explicit child launch boundaries
- limiting known risky subprocess capabilities

Почему это не core truth:

- Unix-only shape
- resource meanings differ by platform and host
- embeddable product cannot assume host is allowed to set such limits

Здоровый вывод:

- `rlimit` belongs in Unix-specific infrastructure leaves
- public runtime policy should express intent, not raw `RLIMIT_*` constants

То есть:

- `SessionBudget { cpu_time, file_descriptors, memory, wall_clock }`

лучше, чем:

- `setrlimit(RLIMIT_NOFILE, ...)` как часть core application service

## 5. `cgroups-rs` and `systemd-run` belong to deployment-aware outer layers

После просмотра свежих crate/repo signals вывод стал жёстче:

### `cgroups-rs`

Интересен, когда:

- runtime is deployed on Linux
- host explicitly wants cgroup-based isolation
- resource governance must survive beyond one child handle

Но он не должен становиться universal center because:

- Linux-specific
- cgroup availability and policy differ by environment
- container/desktop/embedded hosts all behave differently

### `systemd-run`

Интересен, когда:

- product is a standalone app or daemon on systemd-based Linux
- host wants transient service isolation
- operations should be delegated to system supervisor

Но risks here are obvious:

- tiny ecosystem signal compared to Tokio/portable-pty/process-wrap gravity
- extremely deployment-shaped
- bad assumption for arbitrary embedded hosts

🔥 Самый важный product rule:

**outer deployment governance and inner runtime governance must stay separate**

Иначе пакет быстро начинает диктовать host-ам:

- which init system they have
- which isolation primitives they must expose
- which OS contracts are "normal"

## 6. Governance should split into explicit policy domains

Healthiest domain split now looks like this:

### Session governance

- per-session wall-clock budget
- attach/detach limits
- replay catch-up limits
- output retention budget linkage

### Operation governance

- timeout for start/attach/search/export
- concurrency control for expensive jobs
- cancellation semantics

### Route governance

- local vs remote vs SSH route budgets
- maybe lower privileges for remote adjunct operations
- transport-aware retry and reconnect budgets

### Deployment governance

- `rlimit`
- `cgroups`
- `systemd-run`
- sandboxed future sidecars

⚠️ Только последний слой действительно platform-specific.

## 7. Windows and Unix should not share fake resource semantics

Уже из cross-platform PTY work понятно:

- Unix PTY lifecycle and Windows ConPTY lifecycle differ
- group/session semantics differ
- cleanup semantics differ

То же самое applies to resource governance.

Поэтому правильный path:

- one host-neutral policy model
- separate Unix and Windows leaves for enforcement
- maybe some intents unsupported on certain routes or hosts

Это нормально.

Ненормально:

- pretending all limits map 1:1 across platforms

## Practical verdict

Если бы проектировать этот слой прямо сейчас, я бы делал так:

### V1

- typed `ResourceGovernancePolicy`
- per-session budgets and operation deadlines in runtime core
- `CancellationToken` and `TaskTracker` as lifecycle primitives
- bounded concurrency via `Semaphore`
- `process-wrap` ideas in supervision adapter
- `governor` only for outer control/search/export API smoothing
- no hard dependency on `rlimit`, `cgroups-rs`, or `systemd-run` in core crates

### V2

- Unix leaf with optional `rlimit`
- Linux deployment leaf with optional `cgroups-rs`
- standalone Linux host leaf with optional `systemd-run`
- route-specific governance profiles

## Чего я бы избегал

- ❌ Treating rate limiting as the main overload answer
- ❌ Baking `rlimit` or cgroups into core application services
- ❌ Assuming embedders can or want to use systemd
- ❌ Smearing time budgets across random helper functions
- ❌ Making process supervision and resource governance one giant abstraction

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- resource governance should be its own bounded context
- public policy types should express intent, not raw OS constants
- OS-specific enforcement belongs in optional adapters
- route-aware limits should be first-class
- time budgets and cancel semantics should be explicit in the protocol
- host apps should negotiate capabilities, not inherit hidden platform assumptions

## Sources

- [tokio crate](https://crates.io/crates/tokio)
- [tokio repo](https://github.com/tokio-rs/tokio)
- [tokio-util crate](https://crates.io/crates/tokio-util)
- [tokio-util docs](https://docs.rs/tokio-util/latest/tokio_util/)
- [governor crate](https://crates.io/crates/governor)
- [governor repo](https://github.com/boinkor-net/governor)
- [rlimit crate](https://crates.io/crates/rlimit)
- [rlimit repo](https://github.com/Nugine/rlimit)
- [cgroups-rs crate](https://crates.io/crates/cgroups-rs)
- [cgroups-rs repo](https://github.com/kata-containers/cgroups-rs)
- [systemd-run crate](https://crates.io/crates/systemd-run)
- [rust-systemd-run repo](https://github.com/xdu-icpc/rust-systemd-run)
- [process-wrap crate](https://crates.io/crates/process-wrap)
- [process-wrap repo](https://github.com/watchexec/process-wrap)
