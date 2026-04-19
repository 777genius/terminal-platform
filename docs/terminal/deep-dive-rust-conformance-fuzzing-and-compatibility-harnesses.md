# Deep Dive - Rust Conformance, Fuzzing And Compatibility Harnesses

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для terminal package такого уровня недостаточно:

- хорошего PTY layer
- красивого protocol boundary
- чистого runtime ownership

🔥 Если у вас нет сильной compatibility discipline, то package всё равно со временем начнёт деградировать:

- один VT sequence quietly ломает reflow
- snapshot restore расходится с live session
- alt-screen passes locally, but breaks after resize churn
- copy/search/link ranges начинают drift-ить на wide chars and soft wraps
- один host adapter accidentally changes data-plane ordering

Для terminal runtime мирового уровня testing надо мыслить не как "юнитки плюс пара ручных прогонов", а как **multi-layer verification system**.

## Primary Sources

- [`expectrl` docs](https://docs.rs/expectrl)
- [`termwright` docs](https://docs.rs/termwright/0.2.0/termwright/)
- [`insta` docs](https://docs.rs/insta/1.47.2/insta/)
- [`proptest` docs](https://docs.rs/proptest/latest/proptest/)
- [`cargo-fuzz` docs](https://docs.rs/cargo-fuzz/0.13.1/cargo_fuzz/)
- [`cargo-nextest` docs](https://nexte.st)
- [`libtest-mimic` docs](https://docs.rs/libtest-mimic)
- [`rexpect` docs](https://docs.rs/rexpect/0.7.0/rexpect/)
- [`strip-ansi-escapes` docs](https://docs.rs/strip-ansi-escapes)
- [SwiftTerm testing notes mentioning `vttest` and `esctest`](https://github.com/migueldeicaza/SwiftTerm)

## Freshness signals

- `expectrl 0.8.0` - repo `zhiburt/expectrl`, `211` stars, updated `2026-04-14`
- `termwright 0.2.0` - repo `fcoury/termwright`, `13` stars, updated `2026-04-12`
- `insta 1.47.2` - repo `mitsuhiko/insta`, `2827` stars, updated `2026-04-19`
- `proptest 1.11.0` - repo `proptest-rs/proptest`, `2102` stars, updated `2026-04-16`
- `cargo-fuzz 0.13.1` - repo `rust-fuzz/cargo-fuzz`, `1786` stars, updated `2026-04-18`
- `cargo-nextest 0.9.133` - repo `nextest-rs/nextest`, `2913` stars, updated `2026-04-18`
- `libtest-mimic 0.8.2` - repo `LukasKalbertodt/libtest-mimic`, `135` stars, updated `2026-04-12`
- `rexpect 0.7.0`
- `strip-ansi-escapes 0.2.1`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**terminal runtime should verify behavior through multiple orthogonal harnesses, not one giant test style**

Healthy shape now looks like:

1. property tests for invariants
2. snapshot/golden tests for projections and restore surfaces
3. interactive PTY tests for end-to-end session behavior
4. fuzzing for parser/protocol/hot replay inputs
5. external compatibility corpora for VT semantics regression

Если этого нет, то package быстро становится "works on my shell" системой.

## Top 3 testing and compatibility strategies

### 1. `Layered verification stack: property + snapshot + interactive PTY + fuzz + external conformance corpus`

`🎯 10   🛡️ 10   🧠 8`  
Примерно `6000-13000` строк.

Что это значит:

- `proptest` for invariants
- `insta` for projection snapshots
- `expectrl` / `termwright` for interactive PTY scenarios
- `cargo-fuzz` for parser/protocol/replay targets
- `libtest-mimic` or similar to drive external corpora
- `vttest` / `esctest` style suites as non-Rust reference inputs

Почему это strongest path:

- every test tool checks a different failure class
- avoids overfitting everything into one framework
- aligns with how serious terminal emulators actually stay correct over time

Где сложность:

- needs discipline in test ownership
- corpora and snapshots need curation

Практический вывод:

✅ Это мой лучший default для package такого уровня.

### 2. `Mostly Rust-native tests with limited external corpus`

`🎯 7   🛡️ 7   🧠 6`  
Примерно `4000-9000` строк.

Что это значит:

- strong property/snapshot/integration testing
- maybe a small hand-picked external corpus
- less reliance on classic VT suites

Почему это интересно:

- easier to keep in one repo
- less infra overhead

Почему weaker:

- easier to miss real-world escape-sequence edge cases
- harder to prove compatibility against established expectations

Практический вывод:

✅ Reasonable v1 compromise if time is tight.  
⚠️ For "world-class universal package" I would still push toward external conformance inputs.

### 3. `Mostly manual regression and host-app smoke tests`

`🎯 2   🛡️ 3   🧠 3`  
Примерно `1500-4000` строк на старт и потом дорого чинить.

Что это значит:

- run examples manually
- maybe smoke-test in Electron or CLI host
- rely on bugs to discover protocol/parser/emulator regressions

Почему это плохой path:

- terminal bugs are famously stateful and sequence-sensitive
- many regressions are not visually obvious until much later
- host-specific testing hides host-neutral runtime bugs

Практический вывод:

❌ Для вашего уровня амбиций этот путь не годится.

## Tool-by-tool findings

## 1. `proptest` - strongest invariant engine

- Crate: [`proptest`](https://crates.io/crates/proptest)
- Latest: `1.11.0`
- Repo stars: `2102`
- Repo updated: `2026-04-16`

Что особенно важно:

- property-based testing with shrinking
- good fit for:
  - protocol framing invariants
  - parser stability invariants
  - snapshot/replay merge rules
  - stale handle / ownership invariants
  - resize + scrollback contracts

Итог:

✅ Strongest default for invariant-heavy runtime logic.

## 2. `insta` - strongest snapshot tool for projections

- Crate: [`insta`](https://crates.io/crates/insta)
- Latest: `1.47.2`
- Repo stars: `2827`
- Repo updated: `2026-04-19`

Что особенно важно:

- snapshot testing is exactly right for:
  - `ScreenSnapshot`
  - `ScreenDelta`
  - restore projections
  - semantic timeline projections
  - protocol envelopes
- redactions and multiple formats help keep snapshots stable and reviewable

Итог:

✅ Strongest default for projection/golden surfaces.

## 3. `expectrl` - strongest practical PTY interaction donor

- Crate: [`expectrl`](https://crates.io/crates/expectrl)
- Latest: `0.8.0`
- Repo stars: `211`
- Repo updated: `2026-04-14`

Что особенно важно:

- positioned exactly as expect-like terminal automation
- good fit for:
  - shell integration tests
  - prompt marker tests
  - resize + output interaction tests
  - control key / paste scenarios

Ограничение:

- Unix-like focus and lower ecosystem gravity than some generic test libs

Итог:

✅ Strong interactive donor for PTY-level tests.

## 4. `termwright` - strongest "Playwright-like" TUI testing idea

- Crate: [`termwright`](https://crates.io/crates/termwright)
- Latest: `0.2.0`
- Repo stars: `13`
- Repo updated: `2026-04-12`

Что особенно важно:

- conceptually very useful
- terminal/TUI automation with a higher-level surface

Почему важно даже при низкой зрелости:

- reinforces the idea that terminal package should expose stable inspectable projections
- useful donor for your own higher-level test DSL

Итог:

⚠️ Great donor.  
✅ Worth watching, but not the only testing pillar.

## 5. `cargo-fuzz` - required for parser and protocol hot paths

- Tool: [`cargo-fuzz`](https://crates.io/crates/cargo-fuzz)
- Latest: `0.13.1`
- Repo stars: `1786`
- Repo updated: `2026-04-18`

Что особенно важно:

- parser/protocol code is exactly where fuzzing pays off
- best targets include:
  - VT parser inputs
  - protocol decoder
  - replay merge logic
  - snapshot import/rehydrate
  - OSC payload parsing

Итог:

✅ Must-have quality tool for this class of package.

## 6. `cargo-nextest` - strong execution layer for a serious workspace

- Tool: [`cargo-nextest`](https://crates.io/crates/cargo-nextest)
- Latest: `0.9.133`
- Repo stars: `2913`
- Repo updated: `2026-04-18`

Что особенно важно:

- faster and more operationally useful than plain cargo test for a big workspace
- helps with flaky-test handling, partitioning and large test matrices

Итог:

✅ Strong default test runner for the workspace.

## 7. `libtest-mimic` - strongest donor for corpus-driven compatibility suites

- Crate: [`libtest-mimic`](https://crates.io/crates/libtest-mimic)
- Latest: `0.8.2`
- Repo stars: `135`
- Repo updated: `2026-04-12`

Что особенно важно:

- lets you build custom test harnesses that still behave like Rust test suites
- very useful when you want:
  - one test per fixture/corpus entry
  - generated compatibility suites
  - imported external cases represented as regular test output

Итог:

✅ Strong donor for `vttest` / corpus-driven harness layers.

## 8. `rexpect` - useful Unix donor, but not the main future shape

- Crate: [`rexpect`](https://crates.io/crates/rexpect)
- Latest: `0.7.0`

Что особенно важно:

- similar family to expect-style PTY interaction
- useful donor and fallback

Почему не мой main default:

- narrower scope
- less aligned with the broader layered stack than `expectrl + other tools`

Итог:

⚠️ Good donor, not my first default.

## 9. `strip-ansi-escapes` - useful derived-surface helper, not parser truth

- Crate: [`strip-ansi-escapes`](https://crates.io/crates/strip-ansi-escapes)
- Latest: `0.2.1`

Почему это полезно:

- quick normalization for some transcript/export tests
- handy for comparing text-only derived outputs

Почему it should stay narrow:

- stripping ANSI is not the same as understanding terminal semantics
- do not let "strip then compare" become the main emulator correctness test

Итог:

✅ Useful helper for derived views only.

## External compatibility suites matter

Even though they are not Rust crates, they are important enough to affect architecture:

- `vttest` remains a valuable reference suite
- `esctest` is still cited by serious terminal emulator projects as a richer compliance suite
- SwiftTerm’s docs explicitly mention both `vttest` and `esctest` as useful terminal compliance resources
- mintty release notes still reference fixes driven by `vttest` and `esctest`

Практический вывод:

🔥 A world-class Rust terminal package should not rely only on self-invented tests.  
It should ingest or mirror external compatibility corpora as regression inputs.

## Recommended architecture rules

### 1. Keep test layers aligned with architecture layers

- property tests for domain/runtime invariants
- snapshot tests for projections
- interactive PTY tests for session behavior
- fuzz targets for parsers and data planes
- corpus suites for standards and compatibility

### 2. Treat resize + scrollback + restore as explicit non-regression contracts

These are not "we’ll notice if broken" features.  
They deserve named compatibility tests.

### 3. Keep snapshots at projection boundaries, not raw internal state dumps

Snapshot what the contract promises:

- `ScreenSnapshot`
- `ScreenDelta`
- `TranscriptProjection`
- `TimelineProjection`

not arbitrary internal structs that churn constantly.

### 4. Keep corpus-driven tests host-neutral

Compatibility suites should target:

- runtime core
- parser/emulator
- projection contracts

not only one Electron UI shell.

### 5. Fuzz hot parsing and merge seams continuously

Especially:

- VT parsing
- OSC parsing
- framed protocol decoding
- replay merge
- snapshot rehydrate

### 6. External compatibility suites should be curated, not blindly trusted

- pin versions
- snapshot expected outcomes
- record known deviations explicitly

## Bottom line

Если свести весь deep dive к одной фразе:

🔥 **a universal Rust terminal package should treat compatibility as a product subsystem, with layered harnesses and external conformance corpora, not as a side effect of having some tests**

Именно это потом отличает "works today" от "remains trustworthy across years and hosts".
