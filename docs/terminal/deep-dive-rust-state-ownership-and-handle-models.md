# Deep Dive - Rust State Ownership, Handle Models, And Internal Storage Choices

**Проверено**: 2026-04-19

## Зачем этот deep dive

Universal embeddable terminal runtime почти неизбежно приходит к большому набору долгоживущих сущностей:

- sessions
- panes
- subscriptions
- replays
- profiles
- routes
- workstreams

И здесь очень легко испортить архитектуру двумя способами:

1. потащить внутренние индексы наружу как public API
2. смешать durable identity, internal storage key и UI projection id в одну сущность

Для такого проекта это критично, потому что stale handles, ABA-like bugs и accidental public key stability потом очень дорого чинятся.

## Primary Sources

- [`slotmap` README](https://github.com/orlp/slotmap/blob/master/README.md)
- [`generational-arena` README](https://github.com/fitzgen/generational-arena/blob/master/README.md)
- [`slab` README](https://github.com/tokio-rs/slab/blob/master/README.md)
- [`uuid` README](https://github.com/uuid-rs/uuid/blob/main/README.md)
- [`ulid-rs` README](https://github.com/dylanhart/ulid-rs/blob/master/README.md)
- [`parking_lot` README](https://github.com/Amanieu/parking_lot/blob/master/README.md)
- [`Moka` README](https://github.com/moka-rs/moka/blob/main/README.md)

## Freshness Signals

На `2026-04-19` сигналы такие:

- `slotmap 1.1.1`, repo `1308` stars, push `2025-12-06`
- `generational-arena 0.2.9`, repo `687` stars, push `2023-08-18`
- `slab 0.4.12`, repo `887` stars, push `2026-01-31`
- `uuid 1.23.1`, repo `1204` stars, push `2026-04-16`
- `ulid 1.2.1`, repo `468` stars, push `2025-03-17`
- `parking_lot 0.12.5`, repo `3319` stars, push `2026-02-21`
- `moka 0.12.15`, repo `2506` stars, push `2026-03-22`
- `smallvec 2.0.0-alpha.12`, repo `1634` stars, push `2026-04-15`
- `compact_str 0.9.0`, repo `826` stars, push `2025-12-24`

## Короткий вывод

🔥 Для такого terminal runtime healthiest shape сейчас такой:

1. **public handles are stable textual IDs like `UUID` or `ULID`**
2. **internal storage uses generational keys or arenas**
3. **derived read models may use caches**
4. **small-string and small-vec optimizations stay internal only**

## Top 3 State/Handle Strategies

### 1. `Public UUID/ULID handles + internal generational storage`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `4000-9000` строк.

Что это значит:

- public API sees opaque session IDs
- internal state registry uses `slotmap` or `generational-arena`
- stale internal references are prevented by generations
- secondary state can hang off internal keys through secondary maps

Почему это strongest path:

- public compatibility does not depend on internal storage shape
- internal mutation stays cheap
- stale-handle bugs become much less likely

### 2. `Public UUID/ULID handles + simple slabs inside isolated actor-owned registries`

`🎯 8   🛡️ 8   🧠 6`  
Примерно `3000-7000` строк.

Что это значит:

- actors/services own their local `slab`
- slabs are never leaked outside the owner boundary
- public layer still uses opaque IDs

Почему это может быть enough:

- simple and fast
- good when ownership is already strongly partitioned

Где риск:

- slab indices themselves do not solve stale reference problems
- once references cross subsystem boundaries, you will miss generational safety

### 3. `Expose internal indices or storage keys as public IDs`

`🎯 2   🛡️ 3   🧠 3`  
Примерно `1000-3000` строк initially, much more later to undo.

Почему это плохо:

- internal storage choices become part of public compatibility
- stale handle bugs leak outside the process boundary
- persistence, restore and cross-host embedding become brittle

Практический вывод:

❌ Never do this for the universal package.

## Library-by-library findings

## 1. `slotmap` - strongest default for internal generational object graphs

- `slotmap 1.1.1`
- persistent unique keys
- `SlotMap`, `HopSlotMap`, `DenseSlotMap`
- `SecondaryMap` and `SparseSecondaryMap`
- insertion/deletion/access all O(1)

Почему это отлично подходит:

- sessions and panes often need stable internal references with no strict tree ownership
- secondary maps are great for adjunct state like attention, telemetry counters, subscriptions and derived flags

Самый важный вывод:

🔥 `slotmap` looks like the healthiest default for internal registries where multiple runtime submodels hang off the same entities.

## 2. `generational-arena` - simpler and very honest about the ABA problem

- `generational-arena 0.2.9`
- explicitly explains deletion without ABA problem
- zero `unsafe`
- well tested, including quickchecks

Почему это интересно:

- documentation is very explicit about the actual bug class it prevents
- strong fit for dynamic-lifetime entities

Где уступает:

- less ecosystem gravity and less adjunct-map ergonomics than `slotmap`

Итог:

✅ Strong alternative if you want a simpler arena mental model.  
⚠️ I still prefer `slotmap` for richer runtime graphs.

## 3. `slab` - good primitive, but too low-level to be identity strategy

- `slab 0.4.12`
- pre-allocated storage for a uniform type
- simple index-based access

Почему это полезно:

- owner-local registries
- dense ephemeral objects
- IO/subscription tables

Почему это не identity model:

- stale index protection is your responsibility
- once indices leak across boundaries, problems start

Итог:

✅ Great local storage primitive.  
❌ Not enough by itself for long-lived cross-subsystem entity identity.

## 4. `uuid` vs `ulid`

### `uuid`

- `uuid 1.23.1`
- standard 128-bit identifier
- no central allocator required
- strong distributed-systems semantics
- RFC-backed ecosystem

### `ulid`

- `ulid 1.2.1`
- lexicographically sortable identifier
- canonical 26-char representation
- serde and uuid conversion support

Главный вывод:

- `UUID` is the conservative universal default
- `ULID` is attractive when sortability and log readability matter

Практический вывод:

✅ If public IDs need maximum ecosystem familiarity, use `UUID`.  
✅ If you want naturally sortable textual IDs in logs, snapshots and host tooling, `ULID` is very compelling.

### What I would do

For this package I currently lean to:

- `UUID` for the broadest default compatibility, or
- `ULID` if operator-facing tooling and ordered logs matter more than absolute familiarity

Either way:

🔥 public IDs should stay opaque strings or structured ID value types, not internal keys.

## 5. `parking_lot` - strong internal synchronization, not public architecture

- `parking_lot 0.12.5`
- smaller and faster `Mutex`, `RwLock`, `Condvar`, `Once`
- fairness options
- deadlock detection feature
- low-level parking lot core

Почему это полезно:

- internal runtime synchronization
- smaller lock footprint
- better performance characteristics than std primitives in many cases

Почему осторожно:

- better locks do not fix poor ownership design
- public API should not expose lock semantics

Итог:

✅ Good internal primitive.  
⚠️ Not an excuse to replace actor/ownership boundaries with global lock soup.

## 6. `moka` - useful for derived read models, not for truth

- `moka 0.12.15`
- concurrent cache inspired by Caffeine
- rich eviction and expiration policies
- explicit note that it can be overkill
- major breaking changes happened in `0.12.0`

Почему это полезно:

- local search indexes
- rendered previews
- transcript snippets
- expensive derived metadata

Почему не стоит делать из него more than that:

- caches are derived state
- terminal runtime truth must survive cache invalidation and cache misses

Итог:

✅ Good optional read-model acceleration.  
❌ Do not let cache become the owner of session truth.

## 7. `smallvec` and `compact_str` - internal hot-path tools only

### `smallvec`

- current top published line exposed by `cargo search` is `2.0.0-alpha.12`
- repository README explicitly says 2.0 is not yet ready for release

### `compact_str`

- `compact_str 0.9.0`
- memory-efficient string type that stores short strings on stack when possible

Главный вывод:

- these are micro-optimization tools
- they are good for internal hot paths such as:
  - small arg lists
  - OSC marker lists
  - tab titles
  - short command names
  - tiny structured labels

Но:

❌ They should not leak into public host-facing contracts.  
⚠️ And for `smallvec`, I would avoid betting on the 2.0 alpha line as a foundational public dependency story right now.

## Самые важные architectural выводы

### 1. Public identity and internal storage must be different layers

Public IDs:

- stable
- opaque
- serializable
- host/tool friendly

Internal keys:

- optimized
- replaceable
- process-local
- free to change between minor versions

### 2. Generational safety is worth paying for

Terminal runtimes have enough dynamic lifetimes that stale references are not an edge case.

### 3. Secondary state deserves secondary maps

Attention, UI badges, replay cursors, subscriptions and semantic hints should often hang off entity keys without bloating the core entity structs.

### 4. Caches are read models, not durable fact

This is consistent with the earlier OpenCove-style truth model conclusions.

### 5. Micro-optimized internal types should never shape the public API

Host-neutral package contracts should keep using boring stable types.

## Current Practical Recommendation

Если выбирать сейчас, я бы делал так:

1. **Public IDs**
   - `UUID` or `ULID`
   - opaque in public contracts

2. **Internal registries**
   - `slotmap` as primary default
   - `generational-arena` as simpler alternative
   - `slab` only for owner-local dense tables

3. **Adjunct state**
   - `SecondaryMap`-style approach where appropriate

4. **Synchronization**
   - `parking_lot` as an internal primitive when needed

5. **Derived read models**
   - `moka` only for optional caches

6. **Micro-optimizations**
   - `compact_str` and maybe `smallvec` only inside hot internal paths

## Sources

- [slotmap](https://github.com/orlp/slotmap)
- [generational-arena](https://github.com/fitzgen/generational-arena)
- [slab](https://github.com/tokio-rs/slab)
- [uuid](https://github.com/uuid-rs/uuid)
- [ulid-rs](https://github.com/dylanhart/ulid-rs)
- [parking_lot](https://github.com/Amanieu/parking_lot)
- [moka](https://github.com/moka-rs/moka)
