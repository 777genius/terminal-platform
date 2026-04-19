# Deep Dive - Rust State Machines, Typestate, and Phase Modeling

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для terminal runtime такого уровня уже очевидно, что внутри будут многие отдельные state machines:

- session phase
- attach/detach lifecycle
- reconnect generation
- restore/rehydrate ordering
- launch failure and retry state
- search overlay state
- prompt/foreground gating
- remote route connection state

Но вопрос теперь уже не "нужны ли state machines", а:

- стоит ли тащить FSM/typestate crates в основу runtime
- где подходит compile-time typestate
- где лучше explicit runtime enum + transition methods
- где полезнее model checking, чем macro-FSM

🔥 Именно здесь легко сделать плохой выбор:

- macro DSL выглядит красиво, но плохо ложится на реальный async runtime
- typestate начинает диктовать ownership и lifetime shape
- один crate пытаются использовать и для compile-time protocol, и для runtime transitions, и для tests
- orthogonal state machines начинают насильно сливать в giant machine

Для вашего terminal package это важный слой, потому что продукт уже явно требует много маленьких, но строгих transition systems.

## Primary Sources

### Typestate and FSM crates

- [`typestate` crate](https://crates.io/crates/typestate)
- [`typestate-rs` repo](https://github.com/rustype/typestate-rs)
- [`state-machines` crate](https://crates.io/crates/state-machines)
- [`state-machines-rs` repo](https://github.com/state-machines/state-machines-rs)
- [`sm` crate](https://crates.io/crates/sm)
- [`sm` repo](https://github.com/rustic-games/sm)
- [`rust-fsm` crate](https://crates.io/crates/rust-fsm)
- [`rust-fsm` repo](https://github.com/eugene-babichenko/rust-fsm)
- [`state_machine_future` crate](https://crates.io/crates/state_machine_future)
- [`state_machine_future` repo](https://github.com/fitzgen/state_machine_future)

### Verification-adjacent seam

- [`stateright` crate](https://crates.io/crates/stateright)
- [`stateright` repo](https://github.com/stateright/stateright)

## Freshness signals

- `typestate 0.9.0-rc2` - repo `rustype/typestate-rs`, `155` stars, pushed `2023-07-04`
- `state-machines 0.9.0` - repo `state-machines/state-machines-rs`, `129` stars, pushed `2026-03-24`
- `sm 0.9.0` - repo `rustic-games/sm`, `194` stars, pushed `2020-12-10`
- `rust-fsm 0.8.0` - repo `eugene-babichenko/rust-fsm`, `253` stars, pushed `2025-07-21`
- `state_machine_future 0.2.0` - repo `fitzgen/state_machine_future`, `332` stars, pushed `2019-07-11`
- `stateright 0.31.0` - repo `stateright/stateright`, `1793` stars, pushed `2025-07-27`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**for this kind of terminal runtime, explicit small runtime state machines are stronger than making a macro-FSM crate the architecture center**

Healthiest shape сейчас выглядит так:

1. small explicit enums and transition methods per bounded context
2. one owner task owns each machine's truth
3. FSM/typestate crates may help only in narrow islands
4. model checking is more interesting for invariants than macro-generated runtime shape

То есть не:

- "весь terminal runtime описываем одной state-machine DSL"

а:

- many tiny explicit machines
- clear owners
- selective helpers
- verification where it matters

## Top 3 directions for state-machine strategy

### 1. `Explicit enums + transition methods + owner-task truth`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- each bounded context owns its own enum state
- transitions are explicit methods or command handlers
- runtime tasks own truth
- projections receive events or latest snapshots

Почему это лучший путь:

- easiest to debug
- easiest to evolve with real runtime constraints
- works naturally with async owner-task architecture
- avoids giant macro-generated hidden machinery

### 2. `Selective FSM crate usage for narrow bounded subsystems`

`🎯 7   🛡️ 7   🧠 6`
Примерно `5000-11000` строк.

Идея:

- most runtime machines stay explicit
- one or two tightly bounded machines may use a library like `state-machines` or `rust-fsm`

Почему это sometimes works:

- can document transitions nicely
- can help with guards/callbacks in bounded flows
- useful where the state graph is small and stable

Почему это не лучший общий путь:

- consistency suffers if overused
- async/runtime ownership still needs hand-written orchestration
- libraries can start dictating shape

### 3. `Typestate/macro-first architecture`

`🎯 4   🛡️ 5   🧠 8`
Примерно `7000-15000` строк.

Это плохой default.

Симптомы:

- API and ownership get overfit to compile-time transitions
- runtime composition becomes awkward
- orthogonal state dimensions are hard to model cleanly
- host-facing semantics become harder to project

## 1. Terminal runtime wants many small machines, not one giant machine

This is the main architectural lesson from the earlier product/runtime research, and the crate ecosystem reinforces it.

Examples of separate machines we likely want:

- `SessionPhase`
- `LaunchState`
- `AttachState`
- `RestoreState`
- `SearchUiState`
- `RemoteRouteState`
- `ForegroundOwnershipState`

🔥 Strong rule:

**orthogonal concerns should remain separate machines**

Trying to combine them into one giant formal state graph usually makes:

- transitions unreadable
- testing harder
- evolution harder

## 2. `state-machines` is the strongest runtime-FSM library candidate I found, but still not architecture center material

`state-machines 0.9.0` is the most compelling of the active runtime-oriented candidates.

Why:

- recent activity
- guards/callbacks support
- async and hierarchical features
- feels more modern than older FSM crates

Good role:

- bounded machine with a fairly formal lifecycle
- maybe one dedicated subsystem with readable graph

Bad role:

- foundation for the entire terminal platform

Why not:

- runtime still has owner tasks, queues, stores and projections outside the machine
- too many orthogonal concerns for one library-centered worldview

## 3. `rust-fsm` is respectable, but looks more like a bounded-subsystem helper than a platform foundation

`rust-fsm 0.8.0` remains plausible.

Good at:

- readable FSM specifications
- deliberate smaller formal machines

Less strong for this package because:

- terminal runtime complexity is less about one classic FSM and more about many interacting lifecycle slices
- integration with the rest of async/domain boundaries still needs hand design

So I read it as:

- useful helper candidate
- not main architecture

## 4. `sm` looks stale for this kind of product

`sm 0.9.0` may still be fine for some uses, but the repo signal is too stale for me to choose it as a foundation in a world-class 2026 package.

It can remain:

- donor/reference

but not:

- serious default recommendation

## 5. `typestate` is interesting for compile-time protocols, but runtime terminal truth is a weaker fit

`typestate 0.9.0-rc2` is conceptually attractive, especially for:

- compile-time transition guarantees
- protocol/session setup APIs
- preventing illegal call ordering

But for this package, the main runtime truth is:

- long-lived
- async
- eventful
- reconnectable
- externally observable

That is a weaker fit for typestate-heavy design.

Good role:

- maybe selected builder/setup protocols
- maybe internal compile-time API for one very narrow lifecycle

Bad role:

- core runtime/session truth model

## 6. `state_machine_future` is historically interesting, but no longer a convincing center

`state_machine_future 0.2.0` is a useful historical donor:

- it reminds us that futures can be expressed as state machines

But for this package:

- it is old
- too future-specific
- not where I would anchor modern async runtime architecture

So I would keep it only as:

- historical reference
- not an active recommendation

## 7. `stateright` is more interesting for this package than most macro-FSM crates

This is the most valuable non-obvious finding from this pass.

`stateright 0.31.0` is not a runtime-FSM library.  
It is a model checker.

Why it matters:

- some of our hard problems are really invariant problems
- attach/replay/reconnect/ownership logic may deserve formal scenario exploration
- race-heavy lifecycle bugs are often better attacked through verification than prettier macros

Good role:

- verify narrow critical transition systems
- check invariants like:
  - no double-attach authority
  - no lost final flush before dispose
  - no illegal reveal before restore convergence
  - no conflicting controller/viewer ownership states

🔥 Practical rule:

**for critical runtime invariants, verification tools may be more valuable than runtime DSLs**

## 8. Typestate is stronger for API protocols than for runtime truth

This split is important.

Typestate-like patterns are strongest when:

- object is short-lived
- sequence is strict
- illegal ordering is common
- async/eventful outside world is limited

That means typestate is better suited for things like:

- one-shot setup builders
- constrained open/prepare/finalize APIs
- compile-time safe configuration flow

It is weaker for:

- live session lifecycle
- reconnectable runtime state
- multi-client attach
- event-driven external transitions

## 9. Recommended modeling shape for this package

### Strong default

- explicit enums per bounded context
- explicit transition methods
- owner-task authoritative state
- projection/event layers separate

### Selective helpers

- `state-machines` or `rust-fsm` only for narrow formal subsystems
- maybe typestate for setup/configuration APIs
- `stateright` for critical invariant exploration

### Avoid as centers

- giant FSM DSL
- typestate-driven runtime core
- one-machine-to-rule-them-all design

## 10. If I were designing this layer right now

- `SessionPhase` stays a hand-written enum
- `LaunchState` stays a hand-written enum
- `RestoreState` stays a hand-written enum with very explicit ordered transitions
- `ForegroundOwnershipState` stays explicit and runtime-owned
- search and prompt UX get separate small machines
- one critical lifecycle like attach/reconnect/replay could justify `stateright` modeling
- no FSM crate becomes a public product dependency without very narrow reason

## Things to avoid

- ❌ One giant machine for many orthogonal concerns
- ❌ Letting macro DSL decide runtime boundaries
- ❌ Using typestate where runtime observability and async churn dominate
- ❌ Assuming compile-time transition safety replaces runtime invariant checks
- ❌ Pulling stale FSM crates into core because they look elegant in examples

## Final verdict

🔥 For this terminal package, the healthiest state-modeling path is:

- explicit small runtime machines by bounded context
- selective helper crates only where the state graph is truly narrow and stable
- typestate mostly for setup-like APIs
- `stateright` as the most interesting advanced verification seam

That gives you clarity, debuggability and future-proofing without making the runtime hostage to a state-machine DSL.

## Sources

- [typestate-rs](https://github.com/rustype/typestate-rs)
- [state-machines-rs](https://github.com/state-machines/state-machines-rs)
- [sm](https://github.com/rustic-games/sm)
- [rust-fsm](https://github.com/eugene-babichenko/rust-fsm)
- [state_machine_future](https://github.com/fitzgen/state_machine_future)
- [stateright](https://github.com/stateright/stateright)
