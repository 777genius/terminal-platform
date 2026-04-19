# Deep Dive - Rust Ports, Adapters, Composition Roots, And Test Seams

**Проверено**: 2026-04-19

## Зачем этот слой важен

Для world-class reusable Rust terminal package мало выбрать:

- PTY layer
- emulator core
- daemon protocol
- host SDK boundary

Нужно ещё правильно выразить саму **application architecture**:

- где лежат ports
- как делаются adapters
- как выражать async boundaries
- как собирать runtime without service-locator smell
- как делать test doubles так, чтобы tests не диктовали форму доменной модели

Именно здесь очень легко уехать в одну из двух плохих крайностей:

- macro soup ради "удобного DI"
- или хаотичный manual wiring без ясного composition root

🔥 Для такого пакета это особенно критично, потому что потом один и тот же core должен жить:

- внутри Electron host
- внутри standalone terminal app
- внутри daemon/server mode
- в C/Node/other language adapters

## Primary Sources

### Async traits and public async boundaries

- [`async-trait` README](https://github.com/dtolnay/async-trait/blob/master/README.md)
- [`impl-trait-utils` README](https://github.com/rust-lang/impl-trait-utils/blob/main/README.md)
- [`tower-service` docs](https://docs.rs/tower-service)
- [`tower` docs](https://docs.rs/tower)

### Test seams and trait-object helpers

- [`mockall` README](https://github.com/asomers/mockall/blob/master/README.md)
- [`dyn-clone` README](https://github.com/dtolnay/dyn-clone/blob/master/README.md)
- [`downcast-rs` README](https://github.com/marcianx/downcast-rs/blob/master/README.md)
- [`enum_dispatch` docs](https://docs.rs/enum_dispatch)

### DI/container-style crates

- [`shaku` README](https://github.com/AzureMarker/shaku/blob/master/README.md)
- [`syrette` README](https://github.com/HampusMat/Syrette/blob/master/README.md)

## Freshness signals

- `async-trait 0.1.89` - repo `dtolnay/async-trait`, `2139` stars, pushed `2026-03-24`
- `trait-variant 0.1.2` - repo `rust-lang/impl-trait-utils`, `126` stars, pushed `2025-01-27`
- `mockall 0.14.0` - repo `asomers/mockall`, latest crate, stable-safe mock library
- `tower 0.5.3` - repo `tower-rs/tower`, `4164` stars, pushed `2026-02-24`
- `tower-service 0.3.3`
- `shaku 0.6.2` - repo `AzureMarker/shaku`, `585` stars, pushed `2025-01-24`
- `syrette 0.5.1` - repo `HampusMat/Syrette`, `23` stars, pushed `2024-09-15`
- `dyn-clone 1.0.20` - repo `dtolnay/dyn-clone`, `752` stars, pushed `2026-03-24`
- `downcast-rs 2.0.2` - repo `marcianx/downcast-rs`, `216` stars, pushed `2025-12-31`
- `enum_dispatch 0.3.13`

## Короткий вывод

🔥 Самый здоровый shape для такого проекта сейчас выглядит так:

1. explicit composition root
2. plain constructor injection
3. narrow trait ports only where the seam is real
4. application services as concrete structs
5. async boundaries expressed deliberately, not sprayed through every trait
6. test doubles at port boundaries, not DI-container-driven architecture

То есть healthiest Rust architecture here is **not**:

- giant IoC container
- service locator hidden behind macros
- `#[async_trait]` on every trait in the codebase
- downcasting and enum-dispatch as normal application flow

## Top 3 directions for Ports/Adapters in Rust

### 1. `Explicit composition root + narrow trait ports + concrete application services`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-12000` строк.

Это мой текущий **лучший default**.

Идея:

- domain and application services are ordinary structs
- ports are traits only where there is a real seam:
  - PTY
  - emulator
  - persistence
  - daemon transport
  - remote route
  - clock/uuid/env if needed
- one explicit composition root wires concrete adapters
- no central container crate required

Почему это strongest path:

- easiest to keep Clean Architecture visible
- easiest to test without turning tests into framework consumers
- easiest to embed into different hosts
- easiest to evolve from local-only to daemon/remote architecture later

### 2. `Explicit composition root + helper crates for async/test/adapter ergonomics`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `7000-14000` строк.

Идея:

- same explicit architecture as above
- but selectively use:
  - `trait-variant`
  - `async-trait`
  - `mockall`
  - `tower-service` / `tower`
  - `dyn-clone`
  - `enum_dispatch`

Почему это хороший second path:

- more ergonomic when host boundaries or adapter families get richer
- gives stronger tooling for public async traits and test doubles
- useful for outer transport/middleware leaves

Где риск:

- helper crates start shaping the architecture if used indiscriminately
- easy to slide into macro-first design

### 3. `Container-centric DI / IoC architecture`

`🎯 4   🛡️ 5   🧠 6`  
Примерно `8000-15000` строк.

Идея:

- make container framework the center of assembly
- register traits and implementations globally or module-wise
- resolve services from a DI container

Почему это weak fit here:

- hides composition root behind framework machinery
- adds friction for multi-host embedding
- makes architecture feel more like web-app DI than reusable runtime platform
- often couples tests and app startup too tightly to the container

Практический вывод:

⚠️ Useful for app shells or some web/server products.  
❌ Bad default for a universal embeddable terminal runtime.

## 1. `async-trait` is still useful, but it must stay a boundary helper

The `async-trait` README now explicitly reminds us of something important:

- async functions in traits stabilized in Rust `1.75`
- but traits containing async functions are still not `dyn Trait` compatible

That makes `async-trait 0.1.89` still very relevant.

What it does:

- erases async trait methods into boxed futures
- makes dyn-style async trait objects workable

Why that matters for us:

- some ports really do want trait objects
- especially in adapters, registries and late-bound runtime seams

But the architectural warning is just as important:

🔥 `async-trait` should be a **boundary helper**, not a worldview.

If every trait becomes boxed async by default, you start paying:

- hidden allocation/boxing
- less obvious async semantics
- more macro-driven API shape than domain-driven API shape

Practical takeaway:

- use `async-trait` where dyn async traits are truly needed
- do not spray it across the whole runtime core

## 2. `trait-variant` is the cleanest current donor for public async trait pairs

`trait-variant 0.1.2` is much more interesting than it first looks.

Its README shows a very healthy pattern:

- define a base local trait with `async fn` and/or `-> impl Trait`
- generate a specialized variant with `Send` requirements where needed

This is one of the best current architectural clues for a reusable package:

🔥 **public async ports may need deliberately different local and Send-capable variants**

Why this matters here:

- some application services can stay local to one runtime task
- some adapter-facing traits need `Send`
- forcing one universal trait shape on all of them is often unnecessary friction

Practical takeaway:

✅ Strong helper for public trait design around async/Send.  
⚠️ Still a helper. The domain boundary should stay understandable without knowing the macro expansion.

## 3. `mockall` is the strongest mainstream mock library, but should not dictate domain shape

`mockall 0.14.0` is currently the strongest mature mainstream mocking library I found for this purpose.

Useful facts:

- 100% safe and stable Rust
- ergonomic expectations and predicates
- explicit dev-dependency usage pattern

Why it matters:

- application services above external ports need fast unit tests
- terminal runtime has many real seams where mock doubles are healthy:
  - persistence
  - clock
  - id generation
  - remote transport
  - notification/status bridge

But the warning matters too:

⚠️ **do not distort public traits just to make `mockall` happy**

For world-class package architecture:

- ports should exist because the seam is real
- mocks should adapt to the seam
- not the other way around

## 4. `tower-service` and `tower` are strongest at outer adapter edges

`tower-service 0.3.3` and `tower 0.5.3` are very strong ecosystem tools.

What they are good at:

- request/response style adapter seams
- middleware around retries, timeouts, buffering, load-shedding
- external transports and service wrappers

That makes them interesting for:

- daemon API facade
- remote route adapters
- outer automation/control services

But not ideal as the center of all application ports.

Why:

- a lot of terminal runtime behavior is not a neat request/response service
- subscriptions, event streams and session ownership have richer lifecycles
- forcing all application services into `Service<Request>` form can make the domain less clear

Practical takeaway:

✅ Great for outer transport edges and facades.  
⚠️ Do not let `tower` become the domain language of the whole runtime.

## 5. `shaku` and `syrette` prove DI containers are possible, but not that they are healthy here

`shaku 0.6.2`:

- compile-time dependency injection
- components, providers, submodules
- web framework integrations

`syrette 0.5.1`:

- IoC/DI container inspired by InversifyJS
- async factories
- named bindings
- transient/singleton style registration

These crates are real and usable.

But for this project they reinforce a different conclusion:

🔥 **our package should have a composition root, not a framework-shaped runtime identity**

Why container-first is a bad default here:

- the runtime itself is the product, not a web app assembled once at startup
- host embedding across languages wants explicit seams, not hidden container resolution
- service lifetimes in a terminal runtime are already complex enough without extra container semantics

Practical takeaway:

⚠️ Fine for some app shells or internal tools.  
❌ Not my architectural center for a universal embeddable runtime.

## 6. `dyn-clone` is a sharp tool for closed adapter registries, not a blanket dependency

`dyn-clone 1.0.20` solves a very specific pain:

- cloning trait objects safely

This is useful when you really have:

- cloneable strategy registries
- policy objects
- small closed adapter sets

But it should stay narrow.

Why:

- many true ports should not be cloneable by default
- making trait objects cloneable can hide ownership semantics

Practical takeaway:

✅ Good for strategy-like leaves.  
⚠️ Avoid making `DynClone` a default trait requirement across core ports.

## 7. `downcast-rs` is an internal escape hatch, not a normal port pattern

`downcast-rs 2.0.2` is technically good:

- safe Rust only
- supports associated types and constraints

But its natural use-case is very specific:

- open-ended trait object container
- occasional recovery of concrete types

That is not what most clean application ports should need.

🔥 If routine application logic needs downcasting, the abstraction is usually wrong.

Practical takeaway:

✅ Useful for internal extension islands or tooling registries.  
❌ Bad as a normal control-flow tool in domain/application layers.

## 8. `enum_dispatch` is for closed-world optimization, not open-world architecture

`enum_dispatch 0.3.13` is attractive because it promises near drop-in replacement for dynamic dispatch.

That can be useful when:

- the family of implementations is closed
- performance on a hot path is measured and real
- the set of variants is under our control

But ports/adapters in a reusable package are often intentionally open-world.

That makes `enum_dispatch` a weak architectural center.

Practical takeaway:

✅ Good for some internal closed adapter families.  
⚠️ Not the right default model for public ports.

## What I would actually build

For this terminal package I would shape Rust Ports/Adapters like this:

1. **Domain/application services stay concrete**
2. **Ports are traits only for real external seams**
3. **One explicit composition root assembles runtime variants**
4. **`async-trait` is allowed only at genuine dyn async boundaries**
5. **`trait-variant` is preferred when public async trait pairs need local/Send forms**
6. **`tower` is allowed at outer transport and facade leaves**
7. **`mockall` is used in tests, but tests do not dictate the production design**
8. **DI containers are not required for runtime correctness**

That keeps the architecture:

- cleaner for embedding
- easier to reason about across languages
- less macro-dependent
- more honest about ownership and lifecycle

## Practical recommendations

- ✅ Prefer explicit composition roots over DI containers
- ✅ Keep constructor injection boring and visible
- ✅ Make async trait object boundaries deliberate, not ambient
- ✅ Use `trait-variant` when local and Send public trait variants are genuinely useful
- ✅ Use `tower` only at outer request/response style edges
- ✅ Use `mockall` at port boundaries, not as an excuse to over-abstract
- ⚠️ Keep `dyn-clone` narrow
- ⚠️ Keep `downcast-rs` as an internal escape hatch only
- ⚠️ Keep `enum_dispatch` limited to closed internal families
- ❌ Do not let IoC/DI frameworks define the product architecture

## Sources

- [async-trait README](https://github.com/dtolnay/async-trait/blob/master/README.md)
- [impl-trait-utils README](https://github.com/rust-lang/impl-trait-utils/blob/main/README.md)
- [mockall README](https://github.com/asomers/mockall/blob/master/README.md)
- [tower](https://docs.rs/tower)
- [tower-service](https://docs.rs/tower-service)
- [shaku README](https://github.com/AzureMarker/shaku/blob/master/README.md)
- [syrette README](https://github.com/HampusMat/Syrette/blob/master/README.md)
- [dyn-clone README](https://github.com/dtolnay/dyn-clone/blob/master/README.md)
- [downcast-rs README](https://github.com/marcianx/downcast-rs/blob/master/README.md)
- [enum_dispatch](https://docs.rs/enum_dispatch)
