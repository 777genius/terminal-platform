# Deep Dive - Rust Async Host Loops, Cancellation And Thread Affinity

**Проверено**: 2026-04-19

## Зачем этот deep dive

После предыдущих deep dive уже стало видно, что reusable terminal package должен быть:

- runtime-first
- protocol-first
- handle-first

Но для реально универсального пакета этого мало.

Есть ещё один слой, на котором embeddable runtimes часто ломаются:

🔥 **как Rust async runtime должен пересекать host event loops, UI threads, cancellation semantics и long-lived subscriptions**

Это особенно важно для terminal platform, потому что у нас есть сразу несколько классов асинхронности:

- single-shot commands
- long-lived session streams
- hot screen deltas
- durable background sessions
- attach/detach and reconnect
- host UI main-thread delivery

Если здесь выбрать неправильную модель, то даже сильный core быстро превращается в:

- callback soup
- host-specific promise semantics
- implicit lifecycle via garbage collection
- невозможность нормально поддержать другой host/runtime

## Primary Sources

- [`async-ffi` docs.rs](https://docs.rs/async-ffi/latest/async_ffi/)
- [`ffi-support` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/)
- [`ffi-support::HandleMap` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/handle_map/struct.HandleMap.html)
- [`UniFFI` user guide - Async Overview](https://mozilla.github.io/uniffi-rs/latest/internals/async-overview.html)
- [`UniFFI` user guide - Async FFI details](https://mozilla.github.io/uniffi-rs/latest/internals/async-ffi.html)
- [`UniFFI` user guide - Callback interfaces](https://mozilla.github.io/uniffi-rs/latest/types/callback_interfaces.html)
- [`napi-rs` README](https://github.com/napi-rs/napi-rs/blob/main/crates/napi/README.md)
- [`Diplomat` README](https://github.com/rust-diplomat/diplomat/blob/main/README.md)
- [`diplomat-runtime` docs.rs](https://docs.rs/diplomat-runtime/latest/diplomat_runtime/)

## Короткий вывод

🔥 Для universal terminal package правильный async boundary выглядит так:

1. **core runtime owns async work and cancellation**
2. **hosts see operation handles, subscriptions and explicit state transitions**
3. **single-shot async and long-lived streams are different contracts**
4. **main-thread delivery is adapter work, not runtime truth**
5. **foreign callbacks are convenience edges, not the primary async model**

## Top 3 Async Boundary Strategies

### 1. `Operation handles + explicit poll/await/stream/cancel contract`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Что это значит:

- host starts an operation and receives an opaque `OperationId`
- host can:
  - poll state
  - await completion
  - subscribe to events
  - cancel explicitly
- long-lived subscriptions are separate from one-shot operations
- adapter may expose promises, callbacks or async iterators, but core semantics stay the same

Почему это strongest path:

- одинаково хорошо работает для Node, C, Swift, Python and daemon clients
- cancellation becomes explicit product semantics
- multi-client and reconnect story remain tractable
- backpressure and replay stay attached to the runtime, not the host language

Где риск:

- requires more deliberate design than just returning a future or taking a callback
- you must describe operation and subscription lifecycle clearly

Практический вывод:

✅ Это мой лучший recommendation для вашей цели.

### 2. `Adapter-level futures/promises over a stable runtime contract`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `5000-10000` строк.

Что это значит:

- core still uses handles and explicit runtime semantics
- host adapters map short-lived operations into:
  - JS `Promise`
  - Swift/Kotlin async call
  - Python awaitable
- streams remain streams, not fake promises

Почему это хорошо:

- ergonomic for common hosts
- practical for Electron integration
- aligns well with `napi-rs` and UniFFI async facilities

Где риск:

- adapter ergonomics may start leaking into the contract
- easy to accidentally blur operations and subscriptions together

Практический вывод:

✅ Good adapter layer.  
⚠️ Not enough as the only architectural rule.

### 3. `Callback-first async model`

`🎯 4   🛡️ 5   🧠 5`  
Примерно `3000-7000` строк to demo and then expensive cleanup.

Что это значит:

- start function takes callback(s)
- progress, output, completion and errors all come back through callbacks
- host-side object lifetime implicitly drives runtime lifecycle

Почему это плохо:

- hard to version
- hard to replay
- hard to test
- main-thread affinity becomes accidental host magic
- subscriptions, cancel and reconnect become fragile

Практический вывод:

❌ Не мой путь для v1 reusable platform.

## Tool-by-tool findings

## 1. `async-ffi` - useful tactical tool, not the architecture center

- Crate: [`async-ffi`](https://crates.io/crates/async-ffi)
- Latest: `0.5.0`
- Docs say it provides FFI-compatible `Future`s

Что особенно важно:

- crate explicitly says Rust has no stable ABI for `dyn Future` or `Waker`
- `FfiFuture` is an FFI-safe wrapper
- docs explicitly note extra allocation cost
- docs recommend wrapping async code **only once right at the FFI boundary**

Почему это полезно:

- good tactical seam for very narrow async FFI edges
- useful as a donor for understanding how host-driven future polling can work

Почему это не should be the center:

- terminal runtime is bigger than "export one async fn"
- long-lived sessions and subscriptions want handles, streams and cancel semantics
- FFI-safe futures still do not define product lifecycle

Итог:

✅ Good tactical helper.  
⚠️ Not the primary model for terminal runtime public APIs.

## 2. `ffi-support` - strong donor for FFI handle discipline

- Crate: [`ffi-support`](https://crates.io/crates/ffi-support)
- Latest: `0.4.4`

Что особенно важно:

- docs describe a clear split between:
  - Rust Component
  - FFI Component
  - FFI Consumer
- `HandleMap` gives stable handles convertible to and from 64-bit integers
- docs explicitly say FFI is subtle and error-prone

Почему это важно для нас:

- очень хорошо совпадает с нашей handle-first architecture
- useful donor for C ABI object identity and destruction patterns

Где ограничение:

- this is a support crate, not a whole runtime architecture
- async/session semantics still need to be designed above it

Итог:

✅ Strong donor for C ABI handle model.

## 3. `UniFFI` - useful proof that async over FFI needs explicit scaffolding

- Crate: [`uniffi`](https://crates.io/crates/uniffi)
- Latest: `0.31.1`

Что особенно важно из docs:

- async support is explicit, not "magic"
- generated model includes future handles and operations like:
  - create future handle
  - poll
  - complete
  - free
- callback interfaces are first-class and documented

Почему это важно:

- confirms the right architectural direction
- even a mature binding generator does not pretend plain callbacks are enough
- async lifecycle becomes explicit protocol machinery

Где ограничение:

- generator constraints still shape your object model
- Node story remains third-party and early

Итог:

✅ Strong proof for explicit async contracts.  
⚠️ Still adapter/generator territory, not our core truth.

## 4. `napi-rs` - excellent Node main-thread adapter, not runtime truth

- Crate: [`napi`](https://crates.io/crates/napi)
- Latest: `3.8.5`

Что особенно важно:

- README maps Rust `Async/Future` to JS `Promise`
- README maps `JsFunction` to `threadsafe function`

Что это practically means:

- Node/Electron adapter can safely marshal work back into JS land
- but this is a **Node-specific thread-affinity solution**
- it should not define core product semantics

Правильная роль:

- Rust runtime emits events and owns lifecycle
- `terminal-node` adapter decides when/how to turn that into:
  - promise resolution
  - async iterator item
  - event emitter callback

Итог:

✅ Best Node/Electron leaf.  
❌ Not a universal async contract.

## 5. `Diplomat` and `diplomat-runtime` - useful for callback-safe typed surfaces

- Crate: [`diplomat`](https://crates.io/crates/diplomat)
- Latest: `0.15.0`
- Runtime docs mention FFI-safe types and callback support
- `diplomat-runtime` has `jvm-callback-support`

Что особенно важно:

- useful proof that serious multi-language SDKs need explicit runtime carrier types
- callbacks and string/buffer ownership are not left implicit

Где ограничение:

- still a generator layer
- async lifecycle policy remains your responsibility

Итог:

✅ Good future SDK-layer option.  
⚠️ Not enough to replace runtime protocol design.

## The key architectural rules

## 1. Single-shot ops and subscriptions must be separate

Do not model these as the same thing:

- `create_session`
- `snapshot_now`
- `search_history`
- `subscribe_screen_deltas`
- `attach_viewer`

The first group is operation-like.
The second group is subscription-like.

Host SDKs may make both look ergonomic, but runtime truth must distinguish them.

## 2. Cancellation must be explicit, not garbage-collection driven

Bad shapes:

- "drop the callback and hope Rust notices"
- "if JS promise gets abandoned the operation is effectively cancelled"
- "if the host object disappears, background work probably stops"

Better shape:

- operation has explicit state
- host may call `cancel_operation(op_id)`
- runtime emits terminal state like:
  - pending
  - running
  - completed
  - failed
  - cancelled

## 3. UI/main-thread affinity belongs in adapters

The Rust runtime should not know:

- Node main thread rules
- Electron renderer event-loop quirks
- Swift `DispatchQueue.main`
- Python loop integration details

It should only know:

- event categories
- subscription semantics
- ordering
- cancellation
- backpressure

Then host adapters marshal onto their own safe UI thread or event loop.

## 4. Promises are good for one-shot work, bad for live terminal truth

Good promise candidates:

- create a session
- fetch a snapshot
- request transcript export
- run a bounded search

Bad promise candidates:

- live PTY output
- terminal delta stream
- session phase updates
- long-lived attach/view semantics

Those want stream/subscription semantics.

## 5. Async FFI helpers should stay at the edge

`async-ffi` docs explicitly recommend wrapping only at the boundary.

That aligns with the right package shape:

- inside runtime: normal Rust async/tasks/owners
- at the edge: adapter-specific future or polling bridge

This is another argument against letting FFI async tools leak into domain code.

## Recommended shape for this project

```text
terminal-core
  owns tasks, sessions, cancellation tree, subscriptions

terminal-protocol
  owns operation IDs, subscription IDs, event categories, state envelopes

terminal-daemon
  exposes stream-oriented contract

terminal-capi
  exposes explicit start/poll/cancel/drain surface

terminal-node
  maps the same model into promises, async iterators and TSFN-backed delivery
```

## What I would do in v1

1. Model explicit `OperationId` and `SubscriptionId`.
2. Make cancellation first-class.
3. Keep promises only in host adapters.
4. Keep subscriptions/streams separate from one-shot operations.
5. Use `napi-rs` only as Electron/Node leaf adapter.
6. Treat `async-ffi` as a tactical edge tool, not a public architecture center.
7. Keep main-thread affinity entirely outside the core runtime.

## What I would avoid in v1

- ❌ Direct callback-first public API
- ❌ Returning foreign callers a raw Rust future as the main contract
- ❌ Letting dropped host objects implicitly own cancellation semantics
- ❌ Mixing single-shot async ops and live subscriptions into one generic async surface
- ❌ Encoding host UI thread rules into runtime logic

## Final take

Если сжать всё в одно правило:

🔥 **a world-class embeddable terminal runtime should export explicit async semantics, not borrowed host async habits**

То есть:

- handles first
- operations and subscriptions separate
- cancellation explicit
- adapter promises/callbacks secondary
- host main-thread delivery out of core
