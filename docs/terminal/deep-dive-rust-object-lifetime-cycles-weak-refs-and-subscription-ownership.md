# Deep Dive - Rust Object Lifetimes, Cycles, Weak Refs And Subscription Ownership

**Проверено**: 2026-04-19

## Зачем этот deep dive

После предыдущих Rust deep dive уже стало ясно, что:

- public API должен быть protocol-first
- host SDKs не должны видеть внутренние storage keys
- callbacks и promises должны жить в adapter layer

Но этого всё ещё недостаточно для world-class embeddable terminal package.

Остаётся ещё один очень опасный слой:

🔥 **кто именно чем владеет между Rust runtime и host language, как избежать циклов и как не превратить subscriptions в вечные утечки**

Именно здесь многие красивые SDK ломаются:

- foreign object и Rust object начинают удерживать друг друга
- callback registration случайно становится strong ownership edge
- host wrapper переживает свой session и превращается в stale handle
- destroy/close/dispose semantics становятся неидемпотентными
- detached session умирает только потому, что умер JS/Swift/Python wrapper

Для terminal package это особенно опасно, потому что у нас:

- long-lived sessions
- attach/detach
- screen subscriptions
- transcript/search subscriptions
- возможно несколько host surfaces одновременно
- remote/local/runtime routes

## Primary Sources

- [UniFFI foreign traits guide](https://mozilla.github.io/uniffi-rs/latest/foreign_traits.html)
- [CXX built-in bindings reference](https://cxx.rs/bindings.html)
- [`ffi-support` HandleError docs](https://docs.rs/ffi-support/latest/ffi_support/handle_map/enum.HandleError.html)
- [`ffi-support` ByteBuffer docs](https://docs.rs/ffi-support/latest/ffi_support/struct.ByteBuffer.html)
- [`safer-ffi` docs](https://docs.rs/safer-ffi/0.2.0-rc1/safer_ffi/)
- [`diplomat-runtime` docs](https://docs.rs/diplomat-runtime/0.15.1/diplomat_runtime/)
- [`interoptopus::patterns` docs](https://docs.rs/interoptopus/latest/interoptopus/patterns/)

## Короткий вывод

🔥 Для reusable terminal package правильный lifetime model выглядит так:

1. **Rust runtime owns the real object graph**
2. **hosts only see opaque IDs, borrowed inputs and owned outputs**
3. **subscriptions are explicit owned tokens with explicit close semantics**
4. **observer edges should be weak or owner-task mediated, not strong object cycles**
5. **destroy/close must be idempotent and stale handles must fail deterministically**

Если сделать иначе, то universal SDK быстро превратится в смесь:

- language-specific wrapper semantics
- hidden reference cycles
- unpredictable dispose timing
- cross-language lifetime bugs, которые потом уже нельзя чинить без breaking changes

## Top 3 Lifetime Boundary Strategies

### 1. `Opaque handles + owner-task truth + weak observer edges + explicit subscription tokens`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк до сильного reusable слоя.

Что это значит:

- session, pane, transcript, search stream, screen stream имеют opaque public IDs
- реальная truth model живёт у Rust runtime owners
- host wrappers не владеют runtime graph напрямую
- subscription represented as explicit token:
  - `SubscriptionId`
  - `close_subscription`
  - `subscription_closed` / `subscription_stale`
- observer relationships внутри Rust делаются через owner-task routing или `Weak`-style semantics

Почему это strongest path:

- одинаково хорошо ложится на Node/Electron, C ABI и другие host SDK
- detached session может жить без любого конкретного UI wrapper
- stale host object детерминированно получает `stale_handle` / `wrong_owner` / `already_closed`
- внутреннюю storage модель можно менять, не ломая host contracts

Где сложность:

- нужно очень дисциплинированно разрезать identity, ownership и observation
- нужно явно моделировать close/drop semantics, а не надеяться на host GC

Практический вывод:

✅ Это мой лучший recommendation для terminal package мирового уровня.

### 2. `Generated object wrappers backed by handles and explicit owned carriers`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `6000-12000` строк.

Что это значит:

- public truth всё равно живёт на handles
- но сверху generators вроде `Diplomat` или `UniFFI` выдают language-native wrappers
- wrappers умеют:
  - `close()`
  - `subscribe()`
  - `drop/finalize`
  - typed results/errors

Почему это интересно:

- проще дать удобные SDK для нескольких языков
- ergonomics лучше, чем у "голого C ABI"
- можно держать thin but pleasant host APIs

Где риск:

- генератор начинает навязывать object model
- host wrapper легко начинают воспринимать как "реальные owners"
- callbacks/foreign traits могут снова внести cycle risk

Практический вывод:

✅ Хороший второй слой поверх правильной core lifetime model.  
⚠️ Плохой фундамент, если он начинает определять семантику ownership.

### 3. `Cross-boundary shared ownership object graphs`

`🎯 3   🛡️ 4   🧠 6`  
Примерно `3000-8000` строк на быстрый старт и потом дорогое исправление.

Что это значит:

- Rust object и foreign wrapper удерживают друг друга напрямую
- subscriptions живут пока "кто-то где-то ещё держит ссылку"
- callbacks регистрируются как strong object edges
- dispose timing размазывается между Rust RC и host GC/refcount

Почему это кажется удобным:

- на демо выглядит очень натурально
- объектная модель "сама складывается"

Почему это плохой path:

- циклы и утечки становятся архитектурной нормой
- detached runtime начинает зависеть от случайных host refs
- hard to reason about shutdown ordering
- multi-host attach/detach становится особенно хрупким

Практический вывод:

❌ Для v1 reusable terminal platform я бы этот путь не брал.

## Tool-by-tool findings

## 1. `ffi-support` - лучший donor для stale-handle semantics

- Crate: [`ffi-support`](https://crates.io/crates/ffi-support)
- Latest: `0.4.4`

Что особенно важно:

- `HandleError` уже разделяет `NullHandle`, `InvalidHandle`, `StaleVersion`, `IndexPastEnd`, `WrongMap`
- docs прямо описывают `StaleVersion` как эквивалент `use-after-free / double-free`
- `WrongMap` отдельно моделирует ошибку "handle used against the wrong owner/map"

Почему это важно для terminal package:

- `SessionId`, `PaneId`, `SubscriptionId`, `SearchHandleId` не должны падать одинаково
- нам нужны осмысленные failure categories, а не один generic "invalid handle"

Итог:

✅ Очень сильный donor для public stale-handle policy.  
⚠️ Но не надо тащить сам `HandleMap` как единственную internal truth model.

## 2. `UniFFI` - прямое предупреждение про cycles, не только generator

- Crate: [`uniffi`](https://crates.io/crates/uniffi)
- Latest: `0.31.1`
- Repo stars: `4511`
- Repo updated: `2026-04-19`

Что особенно важно:

- foreign traits принимаются в Rust как `Arc<dyn Trait>`
- в foreign trait methods ссылки не поддерживаются, всё передаётся by value
- guide прямо предупреждает: foreign trait implementations make it easy to create cycles between Rust and foreign objects causing memory leaks
- authors explicitly say UniFFI does not try to solve this for you

Почему это важно:

- если сделать terminal intelligence, auth, storage, shell integration hooks или search providers как foreign trait callbacks, цикл можно создать очень быстро
- значит callback/provider seams должны быть:
  - narrow
  - explicitly owned
  - easy to unregister

Итог:

✅ Очень полезный warning-source для architecture decisions.  
⚠️ Нельзя рассчитывать, что generator сам разрулит ownership graph.

## 3. `cxx` - сильный сигнал не строить truth вокруг shared object graph across ABI

- Crate: [`cxx`](https://crates.io/crates/cxx)
- Latest: `1.0.194`
- Repo stars: `6707`
- Repo updated: `2026-04-19`

Что особенно важно:

- `UniquePtr<T>` и `SharedPtr<T>` cannot hold opaque Rust type
- `Arc<T>` listed as pending binding, not the ready default path
- raw pointers require unsafe declaration

Почему это важно:

- даже очень зрелый Rust/C++ interop tool не говорит "строите спокойно shared RC graph across boundary"
- наоборот, ограничения подталкивают к более явной ownership model

Итог:

✅ Хороший reference, почему shared ownership across ABI не надо делать product truth.  
⚠️ Не воспринимать `SharedPtr` как архитектурную лицензию на cross-language lifetime soup.

## 4. `safer-ffi` - owned и borrowed carriers должны быть разными типами

- Crate: [`safer-ffi`](https://crates.io/crates/safer-ffi)
- Latest: `0.2.0-rc1`
- Repo stars: `1032`
- Repo updated: `2026-04-19`

Что особенно важно:

- crate deliberately separates:
  - boxed types
  - pointer wrappers
  - `char *`-compatible strings
  - repr(C) strings
  - repr(C) vectors
  - closure carriers

Почему это важно:

- ownership should be visible in types
- borrowed input and owned output must not look identical
- subscription callback carriers should not silently become object owners

Итог:

✅ Сильнейший donor для type-level ownership discipline на C ABI edges.

## 5. `diplomat-runtime` - хороший carrier/runtime donor, но не owner of truth

- Crate: [`diplomat-runtime`](https://crates.io/crates/diplomat-runtime)
- Latest: `0.15.1`
- Repo stars: `766`
- Repo updated: `2026-04-17`

Что особенно важно:

- runtime already has explicit owned and borrowed carriers:
  - `DiplomatOwnedSlice`
  - `DiplomatUtf8StrSlice`
  - `DiplomatSlice`
  - `DiplomatCallback`
- also ships explicit Rust-memory alloc/free helpers and destructors
- docs even call out `jvm-callback-support` as a distinct feature when callbacks exist

Почему это важно:

- callbacks and owned slices are not "incidental helper details"
- they are architecture seams that deserve first-class design

Итог:

✅ Очень полезный donor для host SDK carrier design.  
⚠️ Но runtime ownership truth should still live above these carriers.

## 6. `Interoptopus` - service/callback patterns are good adapter sugar

- Crate: [`interoptopus`](https://crates.io/crates/interoptopus)
- Latest observed: `0.16.0-alpha.20`

Что особенно важно:

- patterns docs explicitly model `service` as a grouped receiver/class abstraction
- callback patterns are treated as language/backend conveniences layered on top of C-compatible functions and types
- unsupported pattern can degrade to raw fallback bindings instead of redefining truth

Почему это важно:

- очень здоровое напоминание, что pretty host SDK abstractions должны быть layered patterns
- а не реальное место, где живёт product semantics

Итог:

✅ Хороший adapter-design donor.

## Recommended architecture rules

### 1. Runtime sessions must outlive host wrappers

- host object is just a view/control surface
- session lifetime must be independent
- `detach` should never mean "drop the real session"

### 2. Subscriptions are owned resources, not incidental listeners

- every subscription gets an opaque ID
- every subscription can be explicitly closed
- repeated close must be idempotent
- host GC may call close eventually, but correctness must not depend on that

### 3. Observer edges should be weak by default

- UI listeners
- telemetry sinks
- search observers
- transcript watchers

Эти связи не должны держать session graph живым.

### 4. Stale and wrong-owner must be first-class error categories

Нужно различать хотя бы:

- `not_found`
- `stale_handle`
- `wrong_owner`
- `already_closed`
- `session_poisoned`

### 5. No host-facing borrowed references into runtime internals

Нельзя отдавать наружу:

- emulator cell refs
- transcript rope refs
- direct pointers into session-owned caches
- borrow-based screen views that outlive a call

Наружу должны идти:

- owned blobs
- flat repr(C) carriers
- explicit snapshots/deltas
- subscription streams

### 6. Destroy ordering must be explicit and idempotent

Правильный shape:

- close subscription
- detach viewer
- terminate session or leave detached
- destroy wrapper

Неправильный shape:

- foreign finalizer implicitly kills runtime
- callback still references dropped session
- host wrapper drop silently changes product semantics

## Concrete shape for this terminal package

Для этого проекта healthiest ownership model now looks like:

- `SessionId`, `PaneId`, `ViewerId`, `SubscriptionId` are public opaque IDs
- internal owners keep truth in Rust registries and owner tasks
- host SDK wrappers are thin objects around IDs and operation helpers
- subscriptions are explicit handles with explicit close/cancel
- outbound payloads are owned snapshots/deltas/blobs
- callbacks exist only as adapter sugar above a stable event stream or event pump
- weak observer edges are used for UI and tooling listeners

## Bottom line

Если свести весь deep dive к одной фразе:

🔥 **universal terminal package should export stable identities and explicit owned resources, not cross-language object graphs pretending to share lifetime naturally**

Это один из тех слоёв, где "удобно сейчас" почти всегда превращается в architectural debt потом.
