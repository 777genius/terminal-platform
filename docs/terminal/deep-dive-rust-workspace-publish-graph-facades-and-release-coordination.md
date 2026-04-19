# Deep Dive - Rust Workspace Publish Graph, Facade Crates, and Release Coordination

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для reusable terminal platform мало просто иметь хороший workspace shape внутри репо.

Нужно ещё отдельно решить:

- какие crates вообще должны быть public
- какие crates должны остаться internal
- где нужны facade crates
- как координировать semver across many crates
- как не превратить workspace в случайный набор publishable атомов
- как держать dependency graph and workspace maintenance вменяемыми

🔥 Именно здесь сильные Rust-проекты часто ломаются двумя способами:

- либо публикуют почти всё и получают неуправляемый semver surface
- либо делают один giant crate и теряют модульность и host-specific leaves

Для universal embeddable terminal package это не косметика, а часть architecture contract.

## Primary Sources

### Workspace graph and dependency tooling

- [`guppy` crate](https://crates.io/crates/guppy)
- [`guppy` repo](https://github.com/guppy-rs/guppy)
- [`cargo-hakari` crate](https://crates.io/crates/cargo-hakari)
- [`guppy` repo](https://github.com/guppy-rs/guppy)
- [`cargo-machete` crate](https://crates.io/crates/cargo-machete)
- [`cargo-machete` repo](https://github.com/bnjbvr/cargo-machete)
- [`cargo-deny` crate](https://crates.io/crates/cargo-deny)

### Workspace release coordination

- [`release-plz` crate](https://crates.io/crates/release-plz)
- [`release-plz` repo](https://github.com/release-plz/release-plz)
- [`cargo-workspaces` crate](https://crates.io/crates/cargo-workspaces)
- [`cargo-workspaces` repo](https://github.com/pksunkara/cargo-workspaces)
- [`cargo-release` crate](https://crates.io/crates/cargo-release)
- [`cargo-nextest` crate](https://crates.io/crates/cargo-nextest)
- [`nextest` repo](https://github.com/nextest-rs/nextest)

## Freshness signals

- `guppy 0.17.25` - repo `guppy-rs/guppy`, `265` stars, pushed `2026-04-15`
- `cargo-hakari 0.9.37` - lives in `guppy-rs/guppy`
- `release-plz 0.3.157` - repo `release-plz/release-plz`, `1345` stars, pushed `2026-04-19`
- `cargo-workspaces 0.4.2` - repo `pksunkara/cargo-workspaces`, `584` stars, pushed `2026-01-17`
- `cargo-machete 0.9.2` - repo `bnjbvr/cargo-machete`, `1281` stars, pushed `2026-04-15`
- `cargo-deny 0.19.4`
- `cargo-nextest latest 0.9.133` - installed line `0.9.128`, repo `nextest-rs/nextest`, `2913` stars, pushed `2026-04-19`
- `cargo-release latest 1.1.2`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**publish graph is part of product design, not repo hygiene**

Healthiest shape сейчас выглядит так:

1. many internal crates are fine
2. few public facade crates are better than many tiny public semver surfaces
3. host-specific leaves should publish separately
4. release coordination tooling should follow the graph, not define it
5. dependency-graph hygiene needs its own tools

То есть не:

- "раз уже crate есть, давайте опубликуем"

и не:

- "чтобы не мучиться, всё сольём в один crate"

а:

- explicit public graph
- explicit internal graph
- explicit facade boundaries
- explicit release coordination

## Top 3 directions for workspace publish strategy

### 1. `Small public facade graph + many internal crates + separate host leaves`

`🎯 10   🛡️ 9   🧠 8`
Примерно `8000-16000` строк.

Это strongest default.

Идея:

- domain/application/infra can stay split internally
- only a few host-neutral crates become public
- Node/C/daemon leaves publish separately
- semver surface stays intentional

Почему это лучший путь:

- keeps architecture modular without leaking every boundary to consumers
- lets you evolve internals faster
- avoids semver explosion
- maps well to multi-host product surfaces

### 2. `Many publishable low-level crates`

`🎯 7   🛡️ 6   🧠 7`
Примерно `7000-15000` строк.

Идея:

- almost every useful layer is public
- consumers compose their own stack
- workspace behaves more like a toolkit ecosystem

Почему это иногда привлекательно:

- maximal composability
- power users get finer control

Почему это weaker for this package:

- semver burden grows quickly
- docs/discovery burden grows quickly
- embedders get too many choices too early

### 3. `Single giant public crate`

`🎯 4   🛡️ 5   🧠 4`
Примерно `4000-9000` строк на старт и потом дорого чинить.

Это плохой default.

Симптомы:

- host-specific leaves bleed into core
- compile times and feature surfaces swell
- C ABI, daemon, Node and standalone concerns collide in one package

## 1. Not every internal crate deserves to be public

This is the first strong rule.

For this package, internal crates are likely useful for:

- domain modeling
- runtime orchestration
- projections
- store adapters
- PTY adapters
- test harnesses

But public crates should be much fewer.

Healthy public candidates look more like:

- `terminal-protocol`
- `terminal-runtime`
- `terminal-capi`
- `terminal-node`
- maybe `terminal-testing`

Healthy internal-only candidates look more like:

- `terminal-runtime-internal`
- `terminal-store-sqlite`
- `terminal-pty-portable`
- `terminal-emulator-alacritty`
- topology or orchestration helpers

🔥 Strong rule:

**crate modularity and publishability are different decisions**

## 2. Facade crates are a strength, not a smell, when the real graph is complex

For a workspace like this, facade crates are likely necessary.

Why:

- consumers need a small mental model
- internal graph will likely stay rich
- host-neutral API should not expose all architectural layers

Good facade roles:

- stable host-neutral runtime API
- re-export selected public types from narrower crates
- hide internal adapter churn

Bad facade role:

- giant dumping ground with no internal structure

So the healthy reading is:

- internal graph stays explicit
- facade crate is the curated front door

## 3. `release-plz` looks stronger than ever for multi-crate release coordination

`release-plz 0.3.157` is increasingly compelling for this specific problem:

- release PR workflow
- version coordination across crates
- changelog flow
- semver-aware workspace release ergonomics

Why it matters here:

- this package is likely to have many crates
- public subset may grow over time
- manual version choreography becomes noisy fast

Healthy role:

- release coordination above the already-designed public graph

Unhealthy role:

- deciding which crates should be public in the first place

## 4. `cargo-workspaces` is useful management glue, but should not become the architecture center

`cargo-workspaces 0.4.2` remains useful for workspace-level operations.

Good at:

- managing many crates
- version bump workflows
- workspace bookkeeping

Not good as:

- substitute for public-graph design
- substitute for release policy
- substitute for actual architecture docs

For this project it is best understood as:

- management helper
- not the source of product boundaries

## 5. `guppy` and `cargo-hakari` matter because dependency graph hygiene becomes architecture at scale

This layer is easy to underestimate.

### `guppy 0.17.25`

Strong role:

- reason about large dependency graphs
- query and inspect workspace relationships
- support higher-discipline tooling

### `cargo-hakari 0.9.37`

Strong role:

- workspace-hack package generation
- feature unification and build performance hygiene

🔥 Practical rule:

**large reusable workspaces need dependency-graph tooling, not just human discipline**

For this package especially:

- many host leaves
- many optional adapters
- many test and tooling crates

will eventually create graph pressure.

## 6. `cargo-machete` is valuable because dead deps are worse in a public multi-crate workspace

`cargo-machete 0.9.2` is more valuable here than in a small project.

Why:

- unused deps on one private crate may leak to many workspace builds
- public trust, compile times and review burden all get worse
- big workspaces accumulate stale dependencies quickly

Healthy role:

- periodic graph hygiene
- keep public and internal crates honest

This should stay:

- maintenance enforcement tool

not:

- runtime architecture concept

## 7. `cargo-nextest` still matters because publish graphs need serious verification gates

`cargo-nextest` is already in the testing story, but it matters here for another reason:

- public graph changes need fast reliable verification
- many crates and many features make plain `cargo test` a weaker operational default

So for this layer, `nextest` reinforces a broader point:

- publish graph discipline is only real if CI can exercise it quickly and predictably

## 8. `cargo-deny` belongs in this layer too

Even though `cargo-deny` was already discussed elsewhere, it matters here because:

- a bigger workspace means a more complex public and internal dependency surface
- publish graph decisions change what consumers effectively inherit

Healthy role:

- workspace-wide graph and policy gate
- advisories/licenses/sources guardrail

## 9. Recommended public graph for this terminal package

### Public host-neutral crates

- `terminal-protocol`
- `terminal-runtime`
- `terminal-projections`

### Public host-specific leaves

- `terminal-capi`
- `terminal-node`

### Public operational leaves

- `terminal-daemon`
- `terminal-cli`

### Internal crates

- PTY adapters
- emulator adapters
- storage adapters
- topology helpers
- runtime coordination helpers
- internal test harness helpers

### Optional later public crates

- `terminal-testing`
- `terminal-remote`

This gives:

- small public discovery surface
- large private modularity surface

## 10. Recommended tooling split for this layer

### Strong default

- `release-plz` - multi-crate release coordination
- `cargo-nextest` - verification runner
- `cargo-machete` - unused dep hygiene
- `cargo-deny` - graph policy
- `guppy` / `cargo-hakari` - graph/query/build hygiene

### Useful but secondary

- `cargo-workspaces` - management helper
- `cargo-release` - still useful in some release flows, but less compelling here than `release-plz`

## Things to avoid

- ❌ Publishing internal crates just because they look reusable
- ❌ Making one giant facade crate that re-exports architectural confusion
- ❌ Letting release tooling decide your public graph
- ❌ Letting build-performance hacks leak into public API decisions
- ❌ Treating workspace maintenance tools as substitutes for product architecture

## Final verdict

🔥 For this terminal package, the healthiest publish strategy is:

- rich internal workspace graph
- small intentional public graph
- facade crates for curated entry points
- host leaves published separately
- release coordination layered on top of that through `release-plz` and graph-hygiene tooling

That gives you a platform-shaped Rust project instead of either a giant monocrate or a semver minefield of tiny public crates.

## Sources

- [guppy](https://github.com/guppy-rs/guppy)
- [release-plz](https://github.com/release-plz/release-plz)
- [cargo-workspaces](https://github.com/pksunkara/cargo-workspaces)
- [cargo-machete](https://github.com/bnjbvr/cargo-machete)
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [nextest](https://github.com/nextest-rs/nextest)
- [cargo-release](https://github.com/crate-ci/cargo-release)
