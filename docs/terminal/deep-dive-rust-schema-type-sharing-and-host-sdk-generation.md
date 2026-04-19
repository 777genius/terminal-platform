# Deep Dive - Rust Schema, Type Sharing, and Host SDK Generation

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для reusable terminal package мирового уровня мало иметь:

- хороший protocol design
- хорошие FFI boundaries
- хороший Node leaf

Нужно ещё решить:

- где живёт source of truth для shared types
- как держать JS/UI types в sync с Rust
- как не утонуть в ручных `.d.ts`, json examples и drift
- как не превратить codegen tool в архитектурный центр
- как поддерживать не только TS, но и future Swift/Kotlin/Python/other host SDK leaves

🔥 Именно здесь очень легко сделать хрупкую систему:

- Rust structs становятся случайным public contract без product discipline
- JSON Schema начинают использовать "для всего", включая живой runtime protocol
- TS codegen tool начинает диктовать shape Rust моделей
- different host SDKs начинают дрейфовать по enum/tag/optional semantics

Для вашего terminal package это уже не мелочь, а часть universal-embed story.

## Primary Sources

### Rust-to-TypeScript and multi-language export

- [`ts-rs` crate](https://crates.io/crates/ts-rs)
- [`ts-rs` repo](https://github.com/Aleph-Alpha/ts-rs)
- [`specta` crate](https://crates.io/crates/specta)
- [`specta` repo](https://github.com/specta-rs/specta)
- [`typeshare` crate](https://crates.io/crates/typeshare)
- [`typeshare` repo](https://github.com/1Password/typeshare)

### Schema-based and reflection-based helpers

- [`schemars` crate](https://crates.io/crates/schemars)
- [`schemars` repo](https://github.com/GREsau/schemars)
- [`typify` crate](https://crates.io/crates/typify)
- [`typify` repo](https://github.com/oxidecomputer/typify)
- [`serde-reflection` crate](https://crates.io/crates/serde-reflection)
- [`serde-reflection` repo](https://github.com/zefchain/serde-reflection)

## Freshness signals

- `ts-rs 12.0.1` - repo `Aleph-Alpha/ts-rs`, `1764` stars, pushed `2026-04-09`
- `specta 2.0.0-rc.24` - repo `specta-rs/specta`, `566` stars, pushed `2026-04-16`
- `typeshare latest 1.0.5` - installed line `1.0.2`, repo `1Password/typeshare`, `2930` stars, pushed `2026-04-19`
- `schemars 1.2.1` - repo `GREsau/schemars`
- `typify 0.6.1` - repo `oxidecomputer/typify`, `825` stars, pushed `2026-04-07`
- `serde-reflection 0.5.2` - repo `zefchain/serde-reflection`, `186` stars, pushed `2026-02-23`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**type sharing is an adapter concern around a stable contract, not the contract itself**

Healthiest shape сейчас выглядит так:

1. Rust protocol DTOs stay the intentional truth
2. TS and other host types are generated from that truth or from explicit schema layers
3. config/manifests can use JSON Schema
4. live runtime control/data planes should not become "JSON Schema everywhere"
5. codegen tooling should remain replaceable

То есть не:

- "выберем генератор типов и он станет архитектурой"

а:

- stable contracts first
- codegen second
- host leaves third

## Top 3 directions for schema and host-type strategy

### 1. `Rust DTO truth + targeted generators per host`

`🎯 10   🛡️ 9   🧠 7`
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- core protocol types live in Rust
- TS leaf uses `ts-rs` or `typeshare`
- config/manifests use `schemars`
- external schema ingestion uses `typify` only where needed

Почему это лучший путь:

- clear source of truth
- tools stay replaceable
- host-specific generation stays host-specific
- avoids overfitting everything to one schema worldview

### 2. `Rust DTO truth + typeshare as multi-language export center`

`🎯 8   🛡️ 8   🧠 7`
Примерно `7000-15000` строк.

Идея:

- Rust remains truth
- `typeshare` becomes main export path for TS plus future languages

Почему это интересно:

- much closer to your "не только JS" goal
- deliberately multi-language
- strong repo activity and adoption signal

Почему это не мой default center:

- still should not define runtime semantics
- operations/subscriptions/errors still need separate host-SDK design
- not every protocol shape maps cleanly to every generated language model

### 3. `Schema-first everywhere`

`🎯 5   🛡️ 6   🧠 8`
Примерно `8000-17000` строк.

Идея:

- JSON Schema or schema-like intermediates become central
- Rust and host SDKs are generated around that

Почему это sometimes looks attractive:

- explicit machine-readable contract
- can help with docs and validation

Почему это weaker here:

- live terminal runtime protocol is richer than config/manifests
- schema tooling often loses intent around streams, subscriptions and runtime semantics
- easy to make the package feel schema-shaped instead of runtime-shaped

## 1. `ts-rs` is still the strongest TS-specific leaf

`ts-rs 12.0.1` remains the strongest focused TS export brick.

Why it matters:

- very clear purpose
- mature enough and active
- ergonomic Rust derive path
- good fit for JS/Electron leafs

Best role:

- generate TypeScript types for:
  - control-plane envelopes
  - config DTOs
  - projection shapes
  - host-facing event payloads

Bad role:

- becoming the reason Rust DTOs look a certain way

🔥 Practical rule:

**TS generation should follow protocol DTOs, not drive them**

## 2. `typeshare` is the strongest multi-language type-export donor I found

`typeshare` is especially relevant for your stated goal.

Why:

- it explicitly targets shared type definitions across languages
- much stronger signal for "future non-JS hosts" than pure TS tools
- active and popular enough to take seriously

What it is strong at:

- shared data models
- cross-language carrier types
- reducing drift across leaf SDKs

What it should not be asked to solve:

- runtime ownership model
- async operation/subscription semantics
- daemon protocol transport design
- fault model and cancellation semantics

So the healthy interpretation is:

- `typeshare` may become a very good generator for host DTOs
- but it still sits below real SDK design decisions

## 3. `specta` is interesting, but the RC line matters

`specta 2.0.0-rc.24` is clearly interesting.

Why:

- broader "export Rust types to other languages" ambition
- attractive ergonomics
- growing ecosystem signal

Why I would still treat it carefully:

- currently on RC line
- more tool-ecosystem-shaped than minimal-contract-shaped
- for a package of this ambition, I would rather keep the truth model more conservative

Healthy role:

- watch closely
- maybe use in selected host/tooling leaves
- do not make it the only contract story on day one

## 4. `schemars` is strong for config/manifests, but not the whole runtime contract

`schemars 1.2.1` remains extremely useful.

Best role:

- self-describing config
- manifest schemas
- external validation surfaces
- docs and tooling around structured non-live data

But the strong rule stays:

- JSON Schema is great for static documents
- it is not the whole answer for live runtime control/data planes

For this package that means:

- use `schemars` heavily for config and persisted document-like surfaces
- do not force streaming/session protocol semantics into the same mold

## 5. `typify` is the inverse seam and should stay that way

`typify 0.6.1` is useful because it solves the opposite problem:

- importing external schemas into Rust types

That is valuable for:

- consuming third-party schemas
- compatibility with external config documents
- bridges to schema-owned systems

It is not the right architectural center for your own terminal runtime.

Healthy role:

- boundary ingestion tool

Unhealthy role:

- main authoring path for your own runtime contracts

## 6. `serde-reflection` is interesting for analysis/testing/evolution, not public SDK center

`serde-reflection 0.5.2` is conceptually useful.

Why:

- helps reason about serialization formats
- useful for compatibility and testing tools
- useful as an introspection/evolution donor

Why it should stay secondary:

- reflection-derived shapes are not a substitute for intentional public contract design
- runtime and host semantics need more than reflection

Good role:

- compatibility harnesses
- regression checks
- schema-evolution experiments

## 7. One tool should not try to solve TS, multi-language, JSON Schema, and runtime protocol at once

This is the central anti-slop lesson.

For this package there are really different categories:

### Category A - live runtime contracts

- operations
- events
- subscriptions
- projections
- error envelopes

These should be intentional protocol DTOs.

### Category B - document-like contracts

- config
- manifests
- persisted metadata

These can lean much more on JSON Schema.

### Category C - host SDK carrier types

- TS definitions
- future Swift/Kotlin/etc data carriers

These can be generated from A and B, but should not redefine them.

## 8. Recommended stack for this package right now

### Strong default

- Rust DTO truth in protocol/runtime crates
- `ts-rs` for TS-specific host leaves
- `typeshare` as the most interesting future multi-language export seam
- `schemars` for config/manifests
- `typify` only for external schema ingestion

### Useful but secondary

- `specta` as an experimental/watchlist export layer
- `serde-reflection` for compatibility/testing/evolution tooling

## 9. Practical design rule for your architecture

If I were designing this layer right now:

- `terminal-protocol` owns the canonical Rust DTOs
- `terminal-config-schema` or equivalent owns JSON Schema exports if needed
- `terminal-node` consumes generated TS carriers
- future `terminal-swift-sdk` or `terminal-kotlin-sdk` can consume `typeshare`-style generated carriers
- no generator crate is allowed to dictate runtime semantics

## Things to avoid

- ❌ Making TS generation the hidden reason enums/options/errors look the way they do
- ❌ Treating JSON Schema as the one universal contract for runtime streaming semantics
- ❌ Letting codegen macros become the public architecture
- ❌ Assuming generated carrier types are enough to define a real SDK
- ❌ Collapsing config schemas and live protocol DTOs into one undifferentiated bucket

## Final verdict

🔥 For this terminal package, the healthiest path is:

- Rust-first canonical DTOs
- targeted host type generators as leaves
- `schemars` for document-like surfaces
- `typeshare` as the strongest multi-language watchlist/adjacent path
- `ts-rs` as the strongest TS-specific default today

That gives you a JS-friendly and future multi-language story without turning schema/codegen tooling into the true architecture.

## Sources

- [ts-rs](https://github.com/Aleph-Alpha/ts-rs)
- [specta](https://github.com/specta-rs/specta)
- [typeshare](https://github.com/1Password/typeshare)
- [schemars](https://github.com/GREsau/schemars)
- [typify](https://github.com/oxidecomputer/typify)
- [serde-reflection](https://github.com/zefchain/serde-reflection)
