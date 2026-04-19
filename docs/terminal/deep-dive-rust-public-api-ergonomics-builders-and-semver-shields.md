# Deep Dive - Rust Public API Ergonomics, Builders, and Semver Shields

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для reusable terminal package мирового уровня мало спроектировать:

- сильный runtime core
- хорошие protocol boundaries
- нормальные host adapters

Нужно ещё очень аккуратно спроектировать именно внешний Rust API:

- как embedders будут создавать runtime/session objects
- как API будет эволюционировать без semver pain
- какие surface types должны быть extensible
- где нужны builders
- где нужны opaque handles or newtypes
- где trait boundaries надо seal-ить

🔥 Именно здесь library-level архитектура часто ломается:

- `new(...)` constructors быстро набирают 10+ аргументов
- public structs с открытыми полями цементируют representation
- public traits нельзя безопасно расширять
- builder crate случайно начинает диктовать shape всего API
- generic bounds и convenience derives превращаются в semver traps

Для вашего terminal package это критично, потому что им должны удобно пользоваться и Rust embedders, и внутренние host leaves.

## Primary Sources

### Builder ergonomics crates

- [`typed-builder` crate](https://crates.io/crates/typed-builder)
- [`typed-builder` repo](https://github.com/idanarye/rust-typed-builder)
- [`bon` crate](https://crates.io/crates/bon)
- [`bon` repo](https://github.com/elastio/bon)
- [`derive_builder` crate](https://crates.io/crates/derive_builder)
- [`derive_builder` repo](https://github.com/colin-kiegel/rust-derive-builder)
- [`buildstructor` crate](https://crates.io/crates/buildstructor)
- [`buildstructor` repo](https://github.com/BrynCooke/buildstructor)

### Public API and semver guidance

- [`cargo-public-api` crate](https://crates.io/crates/cargo-public-api)
- [`cargo-public-api` repo](https://github.com/cargo-public-api/cargo-public-api)
- [`cargo-semver-checks` crate](https://crates.io/crates/cargo-semver-checks)
- [`cargo-semver-checks` repo](https://github.com/obi1kenobi/cargo-semver-checks)
- [`Rust API Guidelines`](https://github.com/rust-lang/api-guidelines)
- [`Future proofing` section](https://raw.githubusercontent.com/rust-lang/api-guidelines/master/src/future-proofing.md)
- [`RFC 1105 - API evolution`](https://rust-lang.github.io/rfcs/1105-api-evolution.html)

## Freshness signals

- `typed-builder 0.23.2` - repo `idanarye/rust-typed-builder`, `1162` stars, pushed `2026-04-01`
- `bon 3.9.1` - repo `elastio/bon`, `2013` stars, pushed `2026-04-16`
- `derive_builder 0.20.2` - repo `colin-kiegel/rust-derive-builder`, `1535` stars, pushed `2026-01-02`
- `buildstructor 0.6.0` - repo `BrynCooke/buildstructor`, `71` stars, pushed `2025-11-20`
- `cargo-public-api 0.51.0` - repo `cargo-public-api/cargo-public-api`, `543` stars, pushed `2026-03-30`
- `cargo-semver-checks latest 0.47.0` - installed line `0.46.0`, repo `obi1kenobi/cargo-semver-checks`, `1611` stars, pushed `2026-04-17`
- `rust-lang/api-guidelines` - `1323` stars, pushed `2025-07-08`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**public Rust API should be future-proof by shape, not only by tooling**

Healthiest shape сейчас выглядит так:

1. public API is built around small stable entry points
2. configs and launch specs use builders or spec objects instead of wide constructors
3. structs hide fields unless they are truly passive data
4. extensibility is guarded through sealed traits, newtypes, and `non_exhaustive`-style thinking
5. semver tooling checks the API, but does not rescue sloppy shape decisions

То есть не:

- "потом `cargo-semver-checks` нас прикроет"

а:

- ergonomic shape first
- semver shields second
- verification third

## Top 3 directions for public Rust API shape

### 1. `Spec objects + selective typed builders + sealed traits`

`🎯 10   🛡️ 9   🧠 7`
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- public APIs accept deliberate spec types like `SessionSpec`, `ShellLaunchSpec`, `RuntimeOptions`
- builders only where argument count and optionality justify them
- traits intended only for crate-owned impls are sealed
- public structs keep fields private by default

Почему это лучший путь:

- strong future-proofing
- better rustdoc ergonomics
- fewer semver traps
- clearer domain language

### 2. `Builder-heavy public API`

`🎯 7   🛡️ 7   🧠 6`
Примерно `6000-12000` строк.

Идея:

- most public construction goes through derive-generated builders
- fewer explicit spec objects
- ergonomics optimized for caller convenience

Почему это иногда привлекательно:

- nice callsites
- compile-time required/optional checks possible
- good for config-like public surfaces

Почему это weaker than option 1:

- easier to let builder tooling dictate shape
- rustdoc can become macro-shaped
- less clear separation between stable domain concepts and convenience API

### 3. `Constructor-first API with public structs`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-8000` строк на старт и потом дорого чинить.

Это плохой path.

Симптомы:

- too many args
- public fields pin representation
- every future option becomes breaking or awkward
- trait evolution becomes painful

## 1. Builders are useful, but they should not become the architecture

This is the central lesson.

Builder crates solve one class of problem:

- constructing rich option objects ergonomically

They do **not** solve:

- semver strategy
- ownership model
- trait evolution
- protocol design
- host embedding boundaries

🔥 Strong rule:

**builders should wrap deliberate domain/spec types, not replace them**

Good:

- `ShellLaunchSpec::builder()`
- `DaemonConfig::builder()`
- `SessionOpenOptions::builder()`

Bad:

- "every public API is just a macro-derived builder because constructors are annoying"

## 2. `typed-builder` looks like the safest default builder brick

`typed-builder 0.23.2` currently looks like the strongest conservative default.

Why:

- compile-time type-checked builder
- focused scope
- mature enough and active
- less "new worldview" than some newer macro ecosystems

Best role here:

- public option/spec objects
- internal config structs
- host-leaf config carriers

I would currently rank it as the best default if we want a builder crate at all.

## 3. `bon` is very strong and more ambitious, but also more worldview-shaped

`bon 3.9.1` is impressive.

Why it is attractive:

- rich compile-time checked ergonomics
- function argument ergonomics
- strong builder-centered UX
- clearly active project

Why I would still treat it carefully as an architecture center:

- more opinionated
- more macro/worldview heavy
- easier to let it drive public API design instead of just supporting it

Healthy role:

- watchlist
- maybe use selectively if a specific ergonomics gap really matters

Unhealthy role:

- making the whole public runtime API bon-shaped

## 4. `derive_builder` remains credible, but it looks more legacy-default than strongest-future default

`derive_builder 0.20.2` is still healthy and active enough.

It remains:

- proven
- familiar
- broadly understood

But compared with `typed-builder`, it looks less compelling for a world-class fresh platform API where:

- compile-time guarantees matter
- we want more intentional future-proofing

So I read it as:

- good conservative fallback
- not my strongest default for this package

## 5. `buildstructor` is interesting for constructor-derived builders, but too small to be the center

`buildstructor 0.6.0` is a neat idea:

- derive builders from constructor functions

That can be elegant for some APIs, but for this package:

- it is smaller
- less battle-tested
- not where I would center a public package of this ambition

Good role:

- watchlist
- maybe small internal ergonomic helper

## 6. Public structs should hide fields unless they are truly passive data

The API Guidelines still reinforce the right lesson:

- public fields pin representation
- private fields preserve invariants and future flexibility

For this package, most important public structs are **not** passive C-style bags.

They usually encode:

- launch policy
- runtime options
- capabilities
- session identities
- transport routes

So the healthy default is:

- private fields
- getters if needed
- builder/spec construction

Not:

- giant public mutable bags

## 7. Sealed traits are still one of the strongest semver shields

The API Guidelines' future-proofing section is still one of the most practical references here.

Why sealed traits matter:

- you may add methods later without downstream impl breakage
- you can keep trait implementation authority in the crate
- you avoid accidentally turning every trait into extension surface

For this package, sealing is especially useful for traits like:

- projection formatters
- runtime capability markers
- host bridge traits that are crate-owned
- internal-facing extension traits exposed for method organization

🔥 Strong rule:

**if external implementation is not a product goal, seal the trait**

## 8. `non_exhaustive` thinking should influence enums and option objects even when the exact attribute is not the whole answer

Even if some public APIs won't literally use `#[non_exhaustive]` everywhere, the design mindset matters:

- enums may gain variants
- config/spec structs may gain fields
- error categories may grow

Healthy shape:

- avoid exposing exhaustive assumptions casually
- prefer explicit getters/builders/spec objects over fully open representations

This is especially important for:

- capability enums
- route kinds
- status/event categories
- host-facing error envelopes

## 9. Newtypes and opaque wrappers are still underrated semver tools

The API Guidelines are right here too.

Newtypes and opaque wrappers help:

- hide representation
- reserve future freedom
- attach semantics to IDs and handles
- prevent type confusion

For this package they are especially appropriate for:

- `SessionId`
- `PaneId`
- `SubscriptionId`
- `RouteId`
- maybe path-like or capability-like tagged values

This aligns strongly with earlier handle-model research.

## 10. Avoid gratuitous trait bounds on public data structures

This point from the API Guidelines is directly relevant.

Do not overspecify public structs with trait bounds that:

- can be derived
- are merely convenient today
- will block future evolution tomorrow

For a reusable terminal package, unnecessary bounds on public data structures are a real semver footgun.

Healthy default:

- keep public structs minimally constrained
- let impl blocks carry most behavioral requirements

## 11. `cargo-public-api` and `cargo-semver-checks` are guardrails, not a substitute for design

Both tools are absolutely worth using.

### `cargo-public-api`

Best role:

- inspect and diff actual public surface
- make API drift visible

### `cargo-semver-checks`

Best role:

- CI gate for accidental breaking changes
- semver discipline enforcement

But the strong rule remains:

- these tools catch mistakes
- they do not design a coherent API for you

## 12. Recommended public Rust API shape for this package

### Good candidates for builders/spec objects

- `RuntimeOptions`
- `SessionSpec`
- `ShellLaunchSpec`
- `DaemonConfig`
- `RemoteRouteSpec`
- maybe `ProjectionRequest`

### Good candidates for opaque/newtype wrappers

- IDs
- handles
- route markers
- capability tokens

### Good candidates for sealed traits

- crate-owned formatting/extensibility traits
- capability/provider traits not meant for third-party implementation

### Good candidates for plain passive structs

- small projection DTOs
- data-only read models
- schema/export-only carriers

## Recommended stack for this layer

### Strong default

- API shape guided by Rust API Guidelines
- `typed-builder` for selected public spec/config types
- `cargo-public-api`
- `cargo-semver-checks`

### Useful but secondary

- `bon` as an advanced/watchlist builder option
- `derive_builder` as conservative fallback
- `buildstructor` as niche/internal helper

## Things to avoid

- ❌ Public structs with open fields just for convenience
- ❌ Public traits that are not meant to be implemented, but are left unsealed
- ❌ Letting a builder macro dictate the domain model
- ❌ Constructor signatures that grow forever
- ❌ Relying on semver tooling instead of shaping a future-proof surface first
- ❌ Adding unnecessary public trait bounds to data structures

## Final verdict

🔥 For this terminal package, the healthiest Rust-facing API is:

- spec objects and small stable entry points
- selective builder use
- sealed traits where extension is not intended
- opaque/newtype wrappers for identities and handles
- semver tooling as CI guardrails, not architecture

That gives you a Rust API embedders can love without locking the package into brittle surface choices too early.

## Sources

- [typed-builder](https://github.com/idanarye/rust-typed-builder)
- [bon](https://github.com/elastio/bon)
- [derive_builder](https://github.com/colin-kiegel/rust-derive-builder)
- [buildstructor](https://github.com/BrynCooke/buildstructor)
- [cargo-public-api](https://github.com/cargo-public-api/cargo-public-api)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
- [Rust API Guidelines](https://github.com/rust-lang/api-guidelines)
- [Future proofing](https://raw.githubusercontent.com/rust-lang/api-guidelines/master/src/future-proofing.md)
- [RFC 1105](https://rust-lang.github.io/rfcs/1105-api-evolution.html)
