# Deep Dive - Rust Sync Facades, Runtime Embedding, and Executor Neutrality

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

После предыдущих deep dive уже видно, что core runtime у такого terminal package почти неизбежно будет async-first.

Но для reusable Rust package этого мало.  
Нужно ещё отдельно решить:

- нужен ли blocking facade для Rust embedders
- можно ли обещать executor-neutral API
- как не сделать Tokio публичной религией
- где `block_on` уместен, а где уже ломает lifecycle
- как не дублировать API ради sync/async illusion

🔥 Здесь очень легко сделать ложную "универсальность":

- выдать `block_on` поверх каждого async метода и назвать это sync API
- попытаться держать один и тот же public API одновременно sync и async через macro-magic
- сделать Tokio runtime скрытой глобальной магией
- пообещать executor neutrality, но на деле зацементировать Tokio internals

Для вашего terminal package это важно, потому что часть Rust embedders будут async-first, а часть захотят простой blocking client or facade.

## Primary Sources

### Blocking/executor helper crates

- [`pollster` crate](https://crates.io/crates/pollster)
- [`pollster` repo](https://github.com/zesterer/pollster)
- [`futures-executor` crate](https://crates.io/crates/futures-executor)
- [`futures-rs` repo](https://github.com/rust-lang/futures-rs)
- [`async-compat` crate](https://crates.io/crates/async-compat)
- [`async-compat` repo](https://github.com/smol-rs/async-compat)
- [`sync_wrapper` crate](https://crates.io/crates/sync_wrapper)
- [`sync_wrapper` repo](https://github.com/Actyx/sync_wrapper)
- [`maybe-async` crate](https://crates.io/crates/maybe-async)
- [`maybe-async-rs` repo](https://github.com/fMeow/maybe-async-rs)

### Runtime foundation references

- [`tokio` crate](https://crates.io/crates/tokio)
- [`tokio` repo](https://github.com/tokio-rs/tokio)
- [`futures-core` crate](https://crates.io/crates/futures-core)
- [`futures-util` crate](https://crates.io/crates/futures-util)

## Freshness signals

- `pollster 0.4.0` - repo `zesterer/pollster`, `671` stars, pushed `2025-11-19`
- `futures-executor 0.3.32` - repo `rust-lang/futures-rs`, `5842` stars, pushed `2026-04-12`
- `async-compat 0.2.5` - repo `smol-rs/async-compat`, `190` stars, pushed `2026-01-20`
- `sync_wrapper 1.0.2` - repo `Actyx/sync_wrapper`, `36` stars, pushed `2024-11-20`
- `maybe-async 0.2.10` - repo `fMeow/maybe-async-rs`, `175` stars, pushed `2024-02-22`
- `tokio 1.52.1` - repo `tokio-rs/tokio`, `31697` stars, pushed `2026-04-18`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**the healthiest shape is async-first core plus an optional blocking facade leaf**

Healthiest shape сейчас выглядит так:

1. runtime and public truth stay async-first
2. blocking Rust users get a separate facade/client layer
3. blocking facade owns its own runtime embedding strategy explicitly
4. helper crates like `pollster` or `async-compat` stay tactical
5. `maybe-async` should not define the package's public architecture

То есть не:

- "делаем одну magical API surface и sync, и async одновременно"

а:

- async truth
- optional sync leaf
- explicit runtime ownership

## Top 3 directions for Rust embed surface

### 1. `Async-first core + optional blocking facade crate`

`🎯 10   🛡️ 9   🧠 7`
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- core runtime and canonical Rust API stay async
- an optional `blocking` or `sync` facade crate wraps that API
- facade owns runtime embedding explicitly:
  - dedicated background runtime
  - dedicated client thread
  - or blocking calls into daemon client

Почему это лучший путь:

- keeps one canonical semantic surface
- avoids duplicating logic
- still supports blocking embedders
- preserves future async richness for streams/subscriptions

### 2. `Async-only Rust surface`

`🎯 8   🛡️ 8   🧠 5`
Примерно `5000-10000` строк.

Идея:

- Rust package is unapologetically async
- blocking users must bring their own executor or consume daemon/CLI mode

Почему это sometimes works:

- cleanest semantics
- smallest surface area
- least duplication

Почему это weaker for your stated ambition:

- less friendly to some embedders
- makes "easy embed" story worse for sync consumers
- pushes integration burden outward

### 3. `Dual sync/async public API via macro abstraction`

`🎯 4   🛡️ 5   🧠 8`
Примерно `8000-17000` строк.

Это плохой default.

Симптомы:

- duplicated docs and behavior
- subtle drift between sync and async modes
- macro constraints start shaping the entire API
- harder semver and test matrix

## 1. `pollster` is useful, but only as a tactical blocking helper

`pollster 0.4.0` is clean and focused.

What it is good at:

- tiny blocking bridges
- examples
- tests
- narrow one-shot helper methods

What it is **not** good at for this package:

- whole long-lived runtime embedding story
- complex subscription lifecycle
- sustained terminal session client semantics

🔥 Practical rule:

**`pollster` can help a sync facade, but it should not be the sync facade architecture**

## 2. `futures-executor` is not a substitute for a Tokio-backed runtime story

`futures-executor 0.3.32` is useful, but we need to read it correctly.

Good at:

- generic future execution
- smaller async contexts
- some tests/tooling scenarios

Not enough for this package because:

- the real runtime likely depends on Tokio I/O, timers, tasks, channels
- terminal platform semantics are not just "run this future to completion"

So the right lesson is:

- generic executor helpers are not enough to erase runtime reality

## 3. `async-compat` is a bridge, not a public contract

`async-compat 0.2.5` is helpful exactly as its name suggests:

- compatibility adapter

This can matter for:

- isolated integration seams
- edge compatibility with different async ecosystems
- narrow bridge layers

But it should not become:

- the package's public async philosophy

Healthy role:

- tactical bridge at edges

Bad role:

- excuse to pretend executor differences no longer matter

## 4. `sync_wrapper` is an internal helper, not sync API design

`sync_wrapper 1.0.2` is useful for implementation details around concurrency boundaries.

But for this package:

- it does not design lifecycle
- it does not design blocking semantics
- it does not design embed ergonomics

So it belongs to:

- internal helper bucket

not:

- public API strategy

## 5. `maybe-async` is the clearest example of false universality risk here

`maybe-async 0.2.10` is clever.

Why it is tempting:

- one source for sync and async flavors
- less visible duplication at first glance

Why it is a bad center for this package:

- terminal runtime semantics are natively async-rich
- subscriptions/streams/lifecycle do not collapse cleanly into sync shape
- docs and semver matrix become trickier
- macro abstraction starts defining product reality

For a world-class reusable terminal package, I would not make this the main public strategy.

## 6. Async-first truth matches the package we are actually building

This is the broader architectural synthesis.

The package already wants:

- long-lived sessions
- subscriptions
- attach/detach
- reconnect
- remote routes
- background daemon modes

Those are async-native concepts.

So the best architecture is:

- async truth first
- sync convenience second

not the other way around.

## 7. If we offer a sync facade, it should likely be a separate crate or clearly separate leaf

This is the cleanest product shape.

Possible shape:

- `terminal-runtime` - canonical async API
- `terminal-runtime-blocking` - optional blocking facade

Why this is healthy:

- docs stay clearer
- semver stays clearer
- async and blocking assumptions do not blur
- embedders choose explicitly

## 8. Blocking facade should own runtime embedding explicitly

This is the main architectural rule.

Good blocking facade designs make it explicit how async work is hosted:

- dedicated Tokio runtime in a background thread
- dedicated client runtime
- blocking client over daemon protocol

Bad blocking facade designs:

- hidden global runtime
- ad hoc `block_on` in arbitrary methods
- methods that accidentally block while holding locks or session ownership paths

## 9. Recommended stack for this layer

### Strong default

- async-first core API
- optional blocking facade leaf
- `pollster` only for very narrow tactical blocking bridges
- Tokio runtime ownership explicit inside blocking facade

### Useful but secondary

- `async-compat` for narrow ecosystem bridge seams
- `futures-executor` for limited helper/test roles
- `sync_wrapper` as internal helper

### Avoid as centers

- `maybe-async`
- ad hoc `block_on` sprinkled across public API

## 10. If I were designing this layer right now

- `terminal-runtime` exposes canonical async Rust API
- `terminal-runtime-blocking` is a separate leaf crate
- blocking leaf clearly documents runtime ownership
- one-shot convenience methods may use a tiny blocking helper
- streaming/subscription-heavy APIs remain async-first even if the blocking leaf offers polling or iterator-like adaptors

## Things to avoid

- ❌ Pretending one macro-generated API can serve sync and async equally well here
- ❌ Hiding a Tokio runtime globally without explicit ownership semantics
- ❌ Letting blocking convenience freeze async lifecycle design
- ❌ Promising executor neutrality that the runtime cannot really honor
- ❌ Using `block_on` as architecture instead of as a tactical helper

## Final verdict

🔥 For this terminal package, the healthiest embed story is:

- canonical async Rust API
- optional separate blocking facade leaf
- explicit runtime embedding strategy
- tactical bridging crates only at the edges

That gives blocking embedders a good experience without distorting the real async nature of the runtime.

## Sources

- [pollster](https://github.com/zesterer/pollster)
- [futures-rs](https://github.com/rust-lang/futures-rs)
- [async-compat](https://github.com/smol-rs/async-compat)
- [sync_wrapper](https://github.com/Actyx/sync_wrapper)
- [maybe-async-rs](https://github.com/fMeow/maybe-async-rs)
- [tokio](https://github.com/tokio-rs/tokio)
