# Deep Dive - Rust Daemon Readiness, Version Skew, and Upgrade Handshakes

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Если terminal runtime становится отдельным Rust daemon-ом, у продукта почти сразу появляется новая категория багов:

- host app новее daemon-а
- daemon новее host-а
- daemon уже запущен, но ещё не ready
- daemon стартует долго после migration/rebuild
- host не понимает, надо retry, attach, restart или fail fast
- upgrade сломал protocol, хотя binary "вроде бы запускается"

🔥 Это не transport detail и не release glue.

Это отдельный architectural layer, потому что reusable package должен уметь:

- объяснить host-у, ready ли он
- объяснить, compatible ли они вообще
- не превращать startup в endless ping loop
- отделять tactical retry от deliberate compatibility contract

## Primary Sources

### Version and compatibility helpers

- [`semver` crate](https://crates.io/crates/semver)
- [`semver` repo](https://github.com/dtolnay/semver)
- [`version-compare` crate](https://crates.io/crates/version-compare)
- [`version-compare` repo](https://github.com/timvisee/version-compare)

### Retry and readiness-adjacent helpers

- [`backon` crate](https://crates.io/crates/backon)
- [`backon` repo](https://github.com/Xuanwo/backon)
- [`backoff` crate](https://crates.io/crates/backoff)
- [`backoff` repo](https://github.com/ihrwein/backoff)
- [`tokio-retry` crate](https://crates.io/crates/tokio-retry)
- [`tokio-retry` repo](https://github.com/djc/tokio-retry)
- [`wait-timeout` crate](https://crates.io/crates/wait-timeout)
- [`wait-timeout` repo](https://github.com/alexcrichton/wait-timeout)

### Upgrade and binary replacement adjuncts

- [`self-replace` crate](https://crates.io/crates/self-replace)
- [`self-replace` repo](https://github.com/mitsuhiko/self-replace)

### Existing runtime transport baseline

- [`interprocess` crate](https://crates.io/crates/interprocess)
- [`interprocess` repo](https://github.com/kotauskas/interprocess)

## Freshness signals

- `semver 1.0.28` - repo `dtolnay/semver`, actively used ecosystem baseline
- `version-compare 0.2.0` - repo `timvisee/version-compare`
- `backon 1.6.0` - repo `Xuanwo/backon`, `1021` stars, pushed `2026-04-18`
- `backoff 0.4.0` - repo `ihrwein/backoff`, `342` stars, pushed `2026-03-20`
- `tokio-retry 0.3.1` - repo `djc/tokio-retry`, `140` stars, pushed `2026-04-15`
- `wait-timeout 0.2.1` - repo `alexcrichton/wait-timeout`, `74` stars, pushed `2026-03-29`
- `self-replace 1.5.0` - repo `mitsuhiko/self-replace`, `815` stars, pushed `2026-04-18`
- `interprocess 2.4.1` - repo `kotauskas/interprocess`, `551` stars, pushed `2026-04-18`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**readiness and compatibility should be declared by handshake, not inferred from "the socket answered"**

Healthiest shape сейчас выглядит так:

1. host and daemon perform explicit `hello` / `handshake`
2. handshake declares readiness phase and compatibility facts
3. retry helpers only help wait for the declared contract, not replace it
4. version skew is handled by explicit policy
5. upgrades and self-replace remain outer operational leaves

## Top 3 directions for this layer

### 1. `Explicit readiness/compatibility handshake + bounded retry helpers`

`🎯 10   🛡️ 9   🧠 8`
Примерно `7000-15000` строк.

Это strongest default.

Идея:

- daemon publishes `phase`, `protocol_version`, `binary_version`, `scope`, `capabilities`
- host performs handshake before assuming attach/readiness
- host uses bounded retry only while daemon is in expected startup window
- version skew policy is explicit instead of accidental

Почему это лучший путь:

- no fake "connected means ready"
- no guessing from socket existence alone
- compatibility rules become testable
- fits daemon-first architecture much better than ad hoc retries

### 2. `Same handshake + optional upgrade/reexec leaves`

`🎯 7   🛡️ 8   🧠 9`
Примерно `9000-17000` строк.

Это strong later path if the product eventually wants:

- managed daemon upgrades
- self-reexec
- binary replacement flows
- smoother standalone app shipping

Здесь useful:

- `self-replace`
- maybe controlled restart/handoff semantics

Но это не должно быть v1 center. Compatibility and readiness are needed earlier than upgrade choreography.

### 3. `Implicit startup ping loops and string-version heuristics`

`🎯 3   🛡️ 4   🧠 5`
Примерно `3000-7000` строк на старт и потом дорого чинить.

Это плохой default.

Симптомы:

- host keeps retrying connect until "it works"
- daemon is considered ready if ping answered at all
- version checks are string comparisons or absent
- failures show up as mysterious attach issues instead of explicit incompatibility

## 1. `semver` should stay the real compatibility brick

`semver 1.0.28` is still the right boring default when the package needs actual compatibility logic.

Это важно, потому что daemon/host relation has multiple versions:

- binary/package version
- control protocol version
- snapshot/state format version
- maybe capability generation

`semver` is useful for:

- declaring supported host/daemon ranges
- parsing version requirements correctly
- making compatibility rules machine-readable

What it should **not** do:

- replace protocol versioning
- replace explicit capability handshake

🔥 Strong rule:

**binary semver and protocol compatibility are related, but not identical truths**

## 2. `version-compare` is weaker than `semver` for real contracts

`version-compare` can be useful for small utility edges, but for this package it is not the right center.

Why:

- terminal runtime will need real compatibility policy
- not just loose lexical/numeric version ordering
- semver ranges and structured compatibility rules are more meaningful than simple comparisons

Healthy role:

- maybe tiny tooling/CLI helper
- maybe edge scripts

Unhealthy role:

- main host-daemon compatibility engine

## 3. Retry crates belong to startup tactics, not architecture center

This pass made a useful separation much sharper.

### `backon`

`backon 1.6.0` currently looks like the strongest tactical retry helper.

Why:

- modern project
- very active
- flexible sleep/runtime story
- nice API ergonomics

Healthy role:

- bounded readiness retries
- connect-after-spawn retry windows
- polling declared daemon phase until ready or timeout

### `backoff`

`backoff 0.4.0` is still credible and conceptually solid.

Healthy role:

- exponential backoff in operational clients
- reconnect helpers

But compared with `backon`, it feels a bit more like a classic utility than the most compelling modern default.

### `tokio-retry`

Useful but weaker as a default architecture recommendation here.

Why:

- narrower worldview
- more Tokio-shaped
- less attractive as the future-proof default for a universal package

🔥 Strong rule:

**retry helper crates should wait for explicit readiness, not define what readiness means**

## 4. `wait-timeout` is useful at the process-launch edge

`wait-timeout 0.2.1` is narrow, but healthy in exactly that narrow role.

Good role:

- host launches daemon child
- host bounds how long it waits for early exit/failure
- startup flow can distinguish:
  - child died immediately
  - child still booting
  - child became ready

This is useful for:

- launch wrappers
- standalone hosts
- integration tests

It is not a replacement for handshake.

## 5. `self-replace` is an operational leaf, not a readiness core primitive

`self-replace 1.5.0` is interesting because many daemon-based products eventually want:

- self-update
- binary swap
- self-uninstall or self-reexec behavior

That is valuable, but architecturally late.

Healthy role:

- packaged standalone daemon/app flows
- controlled operational upgrades

Unhealthy role:

- being pulled into v1 runtime architecture before readiness and compatibility contracts exist

🔥 Upgrade choreography is an outer product concern. Handshake correctness comes first.

## 6. What the handshake should actually say

For this package, a meaningful host-daemon handshake should likely declare:

- daemon instance id
- binary version
- control protocol version
- declared compatible host range or protocol family
- runtime phase
  - `starting`
  - `ready`
  - `draining`
  - `upgrading`
  - `failed`
- scope/workspace identity
- capability set
- maybe restart epoch

This is much healthier than:

- ping => pong
- therefore "ready"

## 7. Readiness is phase, not boolean

This connects strongly with earlier session-phase research.

A daemon can be:

- alive but not ready
- ready for control plane but not full attach
- draining because replacement is in progress
- incompatible with this host

So host behavior should depend on phase:

- retry
- attach
- reconnect later
- refuse and surface incompatibility
- trigger controlled restart path

## 8. Final verdict

The right question is not:

- "which crate should we use for retries?"

The right question is:

- "what explicit facts must the daemon declare so the host knows whether to wait, attach, fail, or upgrade?"

For your terminal package the strongest answer right now is:

- explicit readiness/compatibility handshake
- `semver` for real compatibility policy
- `backon`/`backoff` only as tactical retry tools
- `wait-timeout` at process-launch edges
- `self-replace` kept as an outer operational leaf
