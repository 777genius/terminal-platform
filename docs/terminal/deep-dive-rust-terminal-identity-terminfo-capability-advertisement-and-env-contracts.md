# Deep Dive - Rust Terminal Identity, Terminfo, Capability Advertisement, and Env Contracts

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для embeddable terminal package мирового уровня недостаточно иметь:

- good PTY layer
- strong emulator core
- good host protocol

Нужно ещё честно отвечать на вопрос:

**что именно child process думает о terminal, внутри которого он запущен?**

Здесь всплывают отдельные слои:

- `TERM`
- `COLORTERM`
- `TERM_PROGRAM`
- shell-visible env markers
- terminfo compatibility story
- host capability negotiation

🔥 Главная ошибка тут - смешать:

- host capabilities
- runtime capabilities
- child-visible terminal identity

Если это смешать, пакет очень быстро начинает:

- обещать child process больше, чем реально умеет host
- использовать неправильные fallbacks
- ломать color/style/media assumptions
- плодить ad hoc env hacks без общей модели

## Primary Sources

### Terminfo and terminal capability crates

- [`terminfo` crate](https://crates.io/crates/terminfo)
- [`meh/rust-terminfo` repo](https://github.com/meh/rust-terminfo)
- [`termini` crate](https://crates.io/crates/termini)
- [`pascalkuthe/termini` repo](https://github.com/pascalkuthe/termini)
- [`terminfo-lean` crate](https://crates.io/crates/terminfo-lean)
- [`proski/terminfo-lean` repo](https://github.com/proski/terminfo-lean)
- [`tinf` crate](https://crates.io/crates/tinf)
- [`edmccard/tvis` repo](https://github.com/edmccard/tvis)
- [`termprofile` crate](https://crates.io/crates/termprofile)
- [`aschey/termprofile` repo](https://github.com/aschey/termprofile)

### Related modern terminal stack

- [`termwiz` crate](https://crates.io/crates/termwiz)
- [`wezterm/wezterm` repo](https://github.com/wezterm/wezterm)

## Freshness signals

- `terminfo 0.9.0` - repo `meh/rust-terminfo`, `79` stars, pushed `2025-10-30`
- `termini 1.0.0` - repo `pascalkuthe/termini`, `14` stars, pushed `2024-03-20`
- `terminfo-lean 0.1.2` - repo `proski/terminfo-lean`, `1` star, pushed `2025-12-29`
- `tinf 0.14.0` - repo `edmccard/tvis`, `1` star, pushed `2017-12-17`
- `termprofile 0.2.2` - repo `aschey/termprofile`, `33` stars, pushed `2026-04-19`
- `termwiz 0.23.3` - repo `wezterm/wezterm`, `25k+` stars, pushed `2026-04-01`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**child-visible terminal identity should be a typed runtime contract, not a side effect of whichever host happens to embed the runtime**

Healthiest shape сейчас выглядит так:

1. runtime owns `TerminalIdentityPolicy`
2. host capability negotiation stays separate
3. child-visible env contract is explicit and versioned
4. terminfo support is an adapter/helper layer, not the sole source of truth
5. richer runtime capabilities should only be advertised when actually supported end-to-end

То есть не:

- "если host умеет X, поставим какой-нибудь env var и будет норм"

а:

- `TerminalIdentity`
- `AdvertisedCapabilitySet`
- `ChildEnvContract`
- optional `TerminfoAdapter`

## Top 3 directions for terminal identity architecture

### 1. `Typed TerminalIdentityPolicy + explicit child env contract + optional terminfo adapter`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- runtime has an explicit model of what terminal it is advertising
- env variables are derived from that model
- terminfo is consulted as helper/compatibility data, not as the only truth
- host capability negotiation stays a separate layer

Почему это лучший путь:

- avoids lying to child processes
- keeps identity stable across host apps
- lets package evolve capabilities without env spaghetti
- supports standalone app and embedded Electron host with the same contract

### 2. `Pragmatic env-first model with limited typed policy`

`🎯 7   🛡️ 7   🧠 5`
Примерно `4000-9000` строк.

Это workable compromise.

Идея:

- keep a documented env contract
- only lightly formalize capability sets
- rely on a few stable conventions like `TERM`, `COLORTERM`, host-specific markers

Почему это иногда нормально:

- simpler to ship
- matches many existing terminal products

Почему это weaker:

- easier to drift across hosts
- capability semantics remain less testable

### 3. `Host-driven identity with ad hoc env injection`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-7000` строк.

Это плохой путь.

Симптомы:

- each host decides its own `TERM` and marker env vars
- capability truth diverges
- terminfo and runtime behavior stop matching

## 1. `terminfo` ecosystem is useful, but should not become the architecture center

This pass made one thing very clear:

there are multiple terminfo-related crates, but none of them should become the core identity model by themselves.

### `terminfo`

- mature enough to be useful
- real terminfo orientation
- but quieter ecosystem gravity and old licensing vibe mean it is more helper than center

### `termini`

- intentionally minimal
- attractive if you want a smaller adapter/helper

### `terminfo-lean`

- modern and tiny
- interesting precisely because it tries to stay lean

### `tinf`

- historical and useful as a signal
- much weaker freshness signal

🔥 Practical rule:

**terminfo belongs in compatibility/adaptation code, not as the only runtime truth about what the package supports**

## 2. `termprofile` is a strong signal that capability detection and styling support deserve their own layer

`termprofile 0.2.2` is one of the most interesting findings in this pass.

Why:

- explicitly about terminal color/styling support
- modern enough and actively updated
- already thinks in terms of detection and handling support variation

This is useful not because it replaces your identity model, but because it reinforces the need for:

- explicit capability sets
- separation between discovered support and advertised support

That distinction matters a lot.

Example:

- host may support rich rendering internally
- but child-visible terminal contract may intentionally advertise a smaller capability surface for compatibility

## 3. `termwiz` remains the strongest living reference for modern capability reality

Even though this pass is about identity rather than renderer/toolkit design, `termwiz 0.23.3` is still a very important signal.

Why:

- serious living modern terminal stack
- capability-aware worldview
- strong monorepo gravity through WezTerm

The key takeaway is not "use `termwiz` for everything".

It is:

🔥 serious terminal software treats capabilities as a real subsystem, not as a couple of env vars

## 4. Host capability negotiation and child-visible identity must stay separate

This is probably the single most important architecture point in this pass.

### Host capability negotiation

This is about:

- what JS/Electron host can render
- whether media/search/clipboard/status bridges are supported
- what attach role can do

### Child-visible identity

This is about:

- what spawned shells and TUI apps believe the terminal is
- what `TERM` and related env say
- what compatibility story the runtime is promising

These are **not** the same thing.

If you merge them:

- the package starts over-advertising
- compatibility regressions get hard to diagnose
- embedded hosts get divergent behavior

## 5. Env contracts should be explicit and versioned

The package will almost certainly want some shell-visible markers of its own.

We already saw examples of products exporting custom env markers.

Healthy approach:

- document them
- version them
- keep them intentionally narrow
- separate identity markers from capability claims

Examples of the distinction:

- identity marker: "this runtime is our package"
- capability marker: "this runtime supports feature X"
- route marker: "this session is remote/local"

Do not collapse all of these into one vague env story.

## 6. `TERM` should be a product decision, not an adapter accident

This is where many packages go wrong.

If `TERM` is just "whatever worked in one host", then:

- compatibility drifts
- tests become host-specific
- downstream consumers cannot trust the package contract

Healthy rule:

- choose an intentional `TERM` strategy
- make it part of `TerminalIdentityPolicy`
- test it with capability/conformance harnesses

## 7. Capability advertisement should be conservative

Because this package aims to be reusable across many hosts, the right bias is:

🔥 **advertise only what is truly supported end-to-end**

That means:

- not what the Rust core could theoretically do
- not what one reference host can do
- not what one experimental renderer leaf can do

but:

- what the actual runtime + host + policy stack guarantees

This is especially important for:

- colors/style
- hyperlinks
- clipboard side effects
- notifications/status bridges
- inline media protocols

## Practical verdict

If I were designing this layer right now:

### V1

- explicit `TerminalIdentityPolicy`
- explicit `ChildEnvContract`
- typed `AdvertisedCapabilitySet`
- terminfo used only via helper/adapters
- `termprofile`-style ideas informing detection logic, not replacing policy
- one stable documented env contract across standalone and embedded hosts

### V2

- richer capability profiles by route/host
- compatibility-matrix testing across env contracts
- optional deeper terminfo integration where real compatibility wins justify it

## Чего я бы избегал

- ❌ Letting each host choose its own `TERM` story ad hoc
- ❌ Equating host render capability with child-visible terminal identity
- ❌ Treating terminfo as the only truth source
- ❌ Over-advertising features because one host can support them
- ❌ Smearing package-specific markers into vague undocumented env hacks

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- terminal identity deserves its own bounded context
- env contracts must be explicit and documented
- capability advertisement should be conservative
- terminfo belongs to adapter/helper layer
- host capability negotiation and child-visible identity must stay separate

## Sources

- [terminfo crate](https://crates.io/crates/terminfo)
- [meh/rust-terminfo](https://github.com/meh/rust-terminfo)
- [termini crate](https://crates.io/crates/termini)
- [pascalkuthe/termini](https://github.com/pascalkuthe/termini)
- [terminfo-lean crate](https://crates.io/crates/terminfo-lean)
- [proski/terminfo-lean](https://github.com/proski/terminfo-lean)
- [tinf crate](https://crates.io/crates/tinf)
- [edmccard/tvis](https://github.com/edmccard/tvis)
- [termprofile crate](https://crates.io/crates/termprofile)
- [aschey/termprofile](https://github.com/aschey/termprofile)
- [termwiz crate](https://crates.io/crates/termwiz)
- [wezterm/wezterm](https://github.com/wezterm/wezterm)
