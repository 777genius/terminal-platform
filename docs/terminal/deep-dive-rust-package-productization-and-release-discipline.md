# Deep Dive - Rust Package Productization, Release Discipline, And Host-Ready Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

Если terminal runtime должен стать не просто внутренним модулем, а **world-class reusable package**, то одних runtime choices мало.

Нужно ещё заранее решить:

- как выглядит public error surface
- как выглядит config and manifest story
- как держать semver discipline
- как shipping story работает для binaries, `cdylib`, Node adapters и C ABI
- как процесс supervision переживает разные платформы
- как package CI доказывает, что runtime не деградирует

Это уже не "какие crates нравятся", а **какая engineering discipline делает пакет реально встраиваемым и долго живущим**.

## Primary Sources

- [`thiserror` README](https://github.com/dtolnay/thiserror/blob/master/README.md)
- [`miette` README](https://github.com/zkat/miette/blob/main/README.md)
- [`schemars` README](https://github.com/GREsau/schemars/blob/master/README.md)
- [`cargo-semver-checks` README](https://github.com/obi1kenobi/cargo-semver-checks/blob/main/README.md)
- [`cargo-public-api` README](https://github.com/Enselic/cargo-public-api/blob/main/README.md)
- [`cargo-deny` README](https://github.com/EmbarkStudios/cargo-deny/blob/main/README.md)
- [`cargo-audit` README](https://github.com/RustSec/rustsec/blob/main/cargo-audit/README.md)
- [`cargo-vet` README](https://github.com/mozilla/cargo-vet/blob/main/README.md)
- [`cargo-dist` README](https://github.com/axodotdev/cargo-dist/blob/main/README.md)
- [`cross` README](https://github.com/cross-rs/cross/blob/main/README.md)
- [`cargo-zigbuild` README](https://github.com/rust-cross/cargo-zigbuild/blob/main/README.md)
- [`Criterion.rs` README](https://github.com/criterion-rs/criterion.rs/blob/master/README.md)
- [`cargo-fuzz` README](https://github.com/rust-fuzz/cargo-fuzz/blob/main/README.md)
- [`camino` README](https://github.com/camino-rs/camino/blob/main/README.md)
- [`signal-hook` README](https://github.com/vorner/signal-hook/blob/master/README.md)
- [`process-wrap` README](https://github.com/watchexec/process-wrap/blob/main/README.md)

## Freshness Signals

На `2026-04-19` актуальные версии и сигналы такие:

- `thiserror 2.0.18`, repo `5402` stars, push `2026-03-24`
- `miette 7.6.0`, repo `2542` stars, push `2025-09-29`
- `serde 1.0.228`
- `schemars 1.2.1`, repo `1330` stars, push `2026-02-03`
- `cargo-semver-checks 0.47.0` latest, repo `1611` stars, push `2026-04-17`
- `cargo-public-api 0.51.0`, push `2025-12-22`
- `cargo-deny 0.19.4`, repo `2265` stars, push `2026-04-15`
- `cargo-audit 0.22.1`, repo `1868` stars, push `2026-04-14`
- `cargo-vet 0.10.2`, repo `815` stars, push `2026-02-26`
- `cargo-dist 0.31.0`, repo `2006` stars, push `2026-04-17`
- `cross 0.2.5`, repo `8119` stars, push `2026-03-25`
- `cargo-zigbuild 0.22.2`, repo `2430` stars, push `2026-04-17`
- `criterion 0.8.2`, repo `5453` stars, push `2026-04-14`
- `cargo-fuzz 0.13.1`, repo `1786` stars, push `2026-02-10`
- `camino 1.2.2`, repo `554` stars, push `2026-03-31`
- `signal-hook 0.4.4`, repo `848` stars, push `2026-04-04`
- `process-wrap 9.1.0`, repo `43` stars, push `2026-04-18`

## Короткий вывод

🔥 Для reusable terminal platform нужен не только runtime stack, но и **package discipline stack**.

Сейчас strongest practical shape такой:

- `thiserror` for public library errors
- `miette` only at app/CLI leafs
- `serde` + `schemars` for config/manifests/tooling
- `process-wrap` for process-tree lifecycle instead of deprecated `command-group`
- `signal-hook` for deliberate Unix shutdown integration
- `cargo-semver-checks` + `cargo-public-api` for API discipline
- `cargo-deny` + `cargo-audit` + optionally `cargo-vet` for supply-chain gates
- `cargo-dist` for releases
- `cross` and/or `cargo-zigbuild` for build matrix
- `criterion` + `cargo-fuzz` for hot-path verification

## Top 3 Productization Strategies

### 1. `Library-first productization stack`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк инфраструктуры и CI glue.

Что входит:

- `thiserror`
- `serde`
- `schemars`
- `process-wrap`
- `camino`
- `cargo-semver-checks`
- `cargo-public-api`
- `cargo-deny`
- `cargo-audit`
- `cargo-dist`
- `criterion`

Почему это лучший default:

- сразу разделяет library surface и app surface
- даёт сильную semver discipline
- хорошо подходит и для standalone app, и для embeddable package
- не тащит лишнюю runtime-магии в public API

### 2. `Strict release-governed stack with supply-chain and multi-target gates`

`🎯 8   🛡️ 10   🧠 8`  
Примерно `7000-14000` строк CI/release plumbing.

Что добавляется:

- `cargo-vet`
- `cross`
- `cargo-zigbuild`
- `cargo-fuzz`
- more target-matrix semver checks

Почему это сильно:

- очень здоровая дисциплина для world-class artifact shipping
- особенно полезно, если package будут реально встраивать внешние команды

Где цена:

- CI и release pipeline сильно тяжелее
- организационная сложность выше технической

### 3. `App-first convenience stack`

`🎯 4   🛡️ 6   🧠 4`  
Примерно `3000-7000` строк.

Что это обычно значит:

- `anyhow`/`miette` everywhere
- ad-hoc JSON configs without generated schema
- manual release scripts
- no semver/public-api gates

Почему это плохо:

- отлично подходит для внутреннего desktop tool
- плохо подходит для reusable terminal platform
- public compatibility быстро размывается

Практический вывод:

❌ Для вашей цели это wrong default.

## Library-by-library findings

## 1. `thiserror` - лучший default для public library errors

- `thiserror 2.0.18`
- explicitly does **not** appear in your public API
- switching between handwritten impls and `thiserror` is not a breaking change
- allows opaque public error wrappers with private evolving internals

Самый важный вывод:

🔥 Для reusable runtime library public errors должны быть deliberate и typed.  
`thiserror` идеально подходит для этого.

## 2. `miette` - очень полезен, но только в app/CLI leaf layer

- `miette 7.6.0`
- diagnostic protocol and fancy report handlers
- README прямо советует fancy reporting only in toplevel crate
- libraries should still return concrete types

Самый важный вывод:

✅ Use `miette` for:

- standalone CLI app
- debug shell
- diagnostics UX in leaf host tools

❌ Do not make `miette::Result` the public API of the core runtime.

## 3. `serde` + `schemars` - лучший baseline для config/manifests/tooling

### `serde`

- `serde 1.0.228`
- default ecosystem baseline for structured data

### `schemars`

- `schemars 1.2.1`
- generate JSON Schema from Rust code
- explicitly aims for `serde` compatibility
- respects `#[serde(...)]` attributes

Самый важный вывод:

🔥 Если terminal runtime должен жить в разных host apps, то config, manifests and protocol-adjacent metadata должны быть не просто `serde`-able, а **schema-exportable**.

Это полезно для:

- host configuration UIs
- validation in Electron or any other host
- generated docs for plugin/adapter configs
- compatibility tooling

## 4. `cargo-semver-checks` - must-have для public crate discipline

- `cargo-semver-checks 0.47.0` latest
- lints crate API changes for semver violations
- checks can run in GitHub Actions
- supports target-specific scanning

Самый важный вывод:

🔥 Если runtime crates публикуются для внешнего мира, semver должен проверяться автоматически, не по памяти maintainers.

Особенно важно:

- run per relevant target if API is target-dependent
- run with deliberate feature set policy

## 5. `cargo-public-api` - сильный companion tool к semver gates

- `cargo-public-api 0.51.0`
- lists and diffs the public API
- supports CI snapshot tests

Почему полезен:

- semver checks ищут violations
- public-api snapshots помогают review intentional API changes

Самый важный вывод:

✅ `cargo-semver-checks` and `cargo-public-api` together give a much healthier release discipline than either one alone.

## 6. `cargo-deny`, `cargo-audit`, `cargo-vet` - не одно и то же

### `cargo-deny`

- dependency graph linting
- licenses
- advisories
- banned crates / duplicate versions
- trusted sources

### `cargo-audit`

- RustSec vulnerability scanning
- can audit binaries
- can preview fixes

### `cargo-vet`

- trusted-entity auditing model for third-party dependencies

Самый важный вывод:

🔥 Для serious reusable package лучше мыслить так:

- `cargo-deny` checks policy
- `cargo-audit` checks known vulnerabilities
- `cargo-vet` checks trust workflow

Это не замены друг другу.

## 7. `cargo-dist` - лучший найденный release orchestrator

- `cargo-dist 0.31.0`
- plan/build/host/publish/announce
- generates machine-readable manifests
- generates CI scripts
- workspace-aware

Почему это очень полезно:

- если пакет станет одновременно library + standalone app + adapters, release story быстро станет тяжёлой
- `cargo-dist` помогает сделать artifact publishing repeatable instead of tribal knowledge

Ограничение:

- это release orchestration, не замена для C ABI packaging или Node adapter packaging

## 8. `cross` vs `cargo-zigbuild`

### `cross`

- `cross 0.2.5`
- "zero setup" cross compilation and testing
- container-based
- great for target matrix validation

### `cargo-zigbuild`

- `cargo-zigbuild 0.22.2`
- uses zig as linker
- very useful for glibc targeting and build portability
- caveats around bindgen/clang and some target specifics are explicit

Самый важный вывод:

🔥 Эти инструменты решают разные задачи:

- `cross` is great for matrix building/testing
- `cargo-zigbuild` is great for artifact portability tuning

Их не надо воспринимать как strict alternatives.

## 9. `criterion` and `cargo-fuzz` - package credibility tools, not optional nice-to-haves

### `criterion`

- `criterion 0.8.2`
- statistically rigorous microbenchmarking
- good for detecting regressions, not just measuring speed once

### `cargo-fuzz`

- `cargo-fuzz 0.13.1`
- easy libFuzzer integration
- nightly and Unix-like only

Самый важный вывод:

🔥 У terminal runtime есть несколько очень "fuzzable" seams:

- parser and framing logic
- replay/snapshot restore logic
- escape sequence handling
- config and manifest parsing

И несколько clearly benchmark-worthy seams:

- PTY write path
- output batching
- snapshot generation
- search/read-model updates

## 10. `camino` - underrated, but very useful for cross-host package ergonomics

- `camino 1.2.2`
- `Utf8Path` and `Utf8PathBuf`
- clear UTF-8 path contract
- conservative MSRV policy

Почему это важно:

- embeddable package inevitably crosses JSON configs, manifests, CLI args, Electron bridges, path serialization
- repeated `to_str().unwrap()` everywhere is architecture smell

Практический вывод:

✅ For config, manifests and host-facing paths, `camino` looks very healthy.  
⚠️ For low-level shell/OS edges you still may need raw `Path`.

## 11. `signal-hook` and `process-wrap` - the process lifecycle layer is bigger than it looks

### `signal-hook`

- `signal-hook 0.4.4`
- safe and correct Unix signal handling
- explains why signals are global and race-prone

### `process-wrap`

- `process-wrap 9.1.0`
- successor to deprecated `command-group`
- composable wrappers, one concern each
- `ProcessGroup`, `ProcessSession`, `JobObject`, `KillOnDrop`, `ResetSigmask`
- explicit std and Tokio frontends

Самый важный вывод:

🔥 Для terminal runtime process supervision не должен быть самодельным набором `pre_exec` hacks и ad-hoc kill logic.

`process-wrap` выглядит намного здоровее старого `command-group`, потому что:

- it is explicit about platform-specific concerns
- wrappers compose by responsibility
- process-group/session/job-object semantics are not hidden behind fake uniformity

## Самые важные architectural выводы

### 1. Separate library UX from app UX

Library core:

- typed errors
- stable contracts
- no fancy printers in public signatures

App/CLI leafs:

- `miette`
- colorful diagnostics
- operator-oriented help

### 2. Make configuration self-describing

If the runtime is meant to be embedded in many hosts, configs and manifests should not only deserialize. They should also produce machine-readable schema.

### 3. Treat semver as a CI-enforced invariant

Reusable package claims are not credible without automatic API compatibility gates.

### 4. Process supervision is part of the domain boundary

Terminal runtime owns:

- process trees
- sessions/groups/jobs
- signal-safe shutdown behavior

This deserves explicit crates and explicit libraries.

### 5. Release engineering is architecture

When one project ships:

- Rust crates
- standalone binaries
- maybe `cdylib`
- maybe Node/Electron adapter

then artifact planning and distribution become part of the architecture, not afterthought ops glue.

## Current Practical Recommendation

Если собирать world-class terminal package сейчас, я бы добавил к previously recommended runtime stack такой productization layer:

1. **Public library ergonomics**
   - `thiserror`
   - `serde`
   - `schemars`
   - `camino`

2. **Process supervision**
   - `process-wrap`
   - `signal-hook`

3. **Public API discipline**
   - `cargo-semver-checks`
   - `cargo-public-api`

4. **Dependency and supply-chain gates**
   - `cargo-deny`
   - `cargo-audit`
   - optionally `cargo-vet`

5. **Release and target matrix**
   - `cargo-dist`
   - `cross`
   - `cargo-zigbuild`

6. **Quality gates**
   - `criterion`
   - `cargo-fuzz`
   - plus earlier recommended `nextest`, `insta`, `proptest`, `loom`

## Sources

- [thiserror](https://github.com/dtolnay/thiserror)
- [miette](https://github.com/zkat/miette)
- [schemars](https://github.com/GREsau/schemars)
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks)
- [cargo-public-api](https://github.com/Enselic/cargo-public-api)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [cargo-vet](https://github.com/mozilla/cargo-vet)
- [cargo-dist](https://github.com/axodotdev/cargo-dist)
- [cross](https://github.com/cross-rs/cross)
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)
- [Criterion.rs](https://github.com/criterion-rs/criterion.rs)
- [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
- [camino](https://github.com/camino-rs/camino)
- [signal-hook](https://github.com/vorner/signal-hook)
- [process-wrap](https://github.com/watchexec/process-wrap)
