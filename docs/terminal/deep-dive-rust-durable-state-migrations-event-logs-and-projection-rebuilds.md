# Deep Dive - Rust Durable State, Migrations, Event Logs, And Projection Rebuilds

**Проверено**: 2026-04-19

## Зачем этот слой важен

У terminal runtime мирового уровня durable state почти никогда не бывает "одной базой с парой таблиц".

Очень быстро появляются разные классы истины:

- session/workspace metadata
- runtime route and authority metadata
- append-only operational history
- committed screen snapshots
- spill blobs and replay tails
- rebuildable read models

🔥 Главная ошибка здесь - смешать всё это в один storage shape и надеяться, что migrations потом "как-нибудь" спасут.

Для такого пакета важен не только выбор store, но и **discipline of evolution**:

- schema versioning
- projection rebuild policy
- append-only logs vs durable truth
- snapshot format versioning
- startup migration orchestration

## Primary Sources

### Embedded durable stores and migration tooling

- [`rusqlite` README](https://github.com/rusqlite/rusqlite/blob/master/README.md)
- [`rusqlite_migration` docs](https://docs.rs/rusqlite_migration)
- [`refinery` README](https://github.com/rust-db/refinery/blob/master/README.md)
- [`redb` README](https://github.com/cberner/redb/blob/master/README.md)
- [`heed` README](https://github.com/meilisearch/heed/blob/main/README.md)
- [`fjall` README](https://github.com/fjall-rs/fjall/blob/main/README.md)

### Advanced SQLite seams

- [`sqlite-vfs` docs](https://docs.rs/sqlite-vfs)
- [`sqlite-loadable` docs](https://docs.rs/sqlite-loadable)

### Async/ORM ecosystem references

- [`sqlx` docs](https://docs.rs/sqlx)
- [`sea-orm-migration` docs](https://docs.rs/sea-orm-migration)
- [`deadpool-sqlite` docs](https://docs.rs/deadpool-sqlite)

## Freshness signals

- `rusqlite 0.39.0` - repo `rusqlite/rusqlite`, `4148` stars, pushed `2026-04-19`
- `rusqlite_migration 2.5.0` - repo `cljoly/rusqlite_migration`, `107` stars, pushed `2026-04-17`
- `refinery 0.9.1` latest, cargo currently resolved info from `0.8.16` docs path, repo `rust-db/refinery`, `1633` stars, pushed `2026-04-15`
- `redb 4.0.0` - repo `cberner/redb`, `4420` stars, pushed `2026-04-19`
- `heed 0.22.1` - repo `Kerollmops/heed`, `874` stars, pushed `2026-04-16`
- `fjall 3.1.4` - repo `fjall-rs/fjall`, `2010` stars, pushed `2026-04-17`
- `sqlx 0.9.0-alpha.1` - pre-release
- `sea-orm-migration 2.0.0-rc.38` - release-candidate line
- `deadpool-sqlite 0.13.0`
- `sqlite-vfs 0.2.0`
- `sqlite-loadable 0.0.6-alpha.6`
- `bonsaidb 0.5.0` - repo `khonsulabs/bonsaidb`, `1060` stars, pushed `2024-07-25`

## Короткий вывод

🔥 Самый здоровый shape сейчас такой:

1. SQLite as the evolving system of record for structured truth
2. append-only operational logs stored explicitly, not confused with semantic timeline
3. snapshot/blob formats versioned separately from relational schema
4. rebuildable projections treated as disposable
5. optional KV/blob stores only where they truly beat SQLite

То есть healthiest architecture is **not**:

- one KV store for everything
- one giant serialized state blob
- event log as a replacement for all domain truth
- read-model caches treated as authoritative

## Top 3 directions for durable state

### 1. `rusqlite + rusqlite_migration + append tables + explicit projection rebuilds`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `7000-15000` строк.

Это мой текущий **лучший default**.

Идея:

- structured truth in SQLite tables
- migrations through `rusqlite_migration`
- append-only operational records in explicit tables
- snapshot/blob payloads stored as versioned binary fields or side blobs
- read models rebuilt when generation/version changes

Почему это strongest path:

- strongest embedded maturity
- easiest long-term schema evolution
- easiest ad-hoc inspection and recovery tooling
- easiest to separate truth tables from projections
- migration story stays boring and visible

### 2. `SQLite truth + redb/fjall for large mirrors or blob-heavy adjunct stores`

`🎯 8   🛡️ 8   🧠 8`  
Примерно `9000-17000` строк.

Идея:

- SQLite remains source of structured truth
- `redb` or `fjall` stores heavier append-ish mirrors, spill data or derived blobs
- runtime has a dual-store strategy with clear ownership boundaries

Почему это интересно:

- keeps metadata/query truth in the strongest place
- lets blob/replay-heavy paths use a storage model better suited for them
- can reduce pressure on the main relational store

Где риск:

- more moving parts
- migration orchestration becomes dual
- integrity and recovery paths must handle cross-store coordination

### 3. `KV/LMDB/LSM-first truth with custom migration discipline`

`🎯 5   🛡️ 6   🧠 8`  
Примерно `9000-18000` строк.

Идея:

- make `redb`, `heed` or `fjall` the primary durable truth
- invent custom schema/version discipline around records and buckets
- rebuild richer query models externally

Почему это weaker path:

- evolution discipline becomes more custom and less legible
- harder to inspect and repair manually
- query-heavy restore rules and dirty-layout logic become more work
- easier to blur truth and projection layers

Практический вывод:

⚠️ Good only if your durable truth is genuinely KV-first.  
❌ Not my default for this terminal package.

## 1. `rusqlite` is still the best center of evolving structured truth

This pass reinforces an earlier conclusion, but with a more specific reason.

It is not only that `rusqlite` is mature.

It is that for this package the durable truth shape is likely to include:

- sessions
- workspaces
- windows/tabs/panes metadata
- runtime routes
- authority/capability metadata
- restore bookkeeping
- dirty-layout / projection-generation bookkeeping

That is not "just blobs".

It is structured, queryable, evolving truth.

🔥 That is exactly where SQLite excels.

Practical takeaway:

- use SQLite for truth that must survive product evolution
- keep its role boring and explicit

## 2. `rusqlite_migration` is the cleanest embedded migration default

`rusqlite_migration 2.5.0` is the most convincing migration helper for this particular package shape.

Why:

- intentionally built for `rusqlite`
- uses SQLite `user_version`
- simple embedded migration story

That simplicity matters a lot here.

We are not building:

- multi-tenant SaaS
- many SQL backends
- migration-heavy async web servers

We are building:

- one embeddable local runtime
- one durable evolving state format

🔥 In that context, `rusqlite_migration` looks healthier than dragging in a heavier generalized migration worldview.

## 3. `refinery` is strong, but it fits better when SQL migration is itself a bigger product seam

`refinery 0.9.1` is good software.

It becomes attractive when you want:

- stronger SQL-file workflow
- broader RDBMS portability
- richer migration organization as its own subsystem

For this package, though, that is not obviously the default.

Why:

- SQLite is already the most plausible truth store
- broader DB portability is not a core requirement
- migration discipline should stay explicit but lightweight

Practical takeaway:

✅ Strong second option if migration workflow gets more elaborate.  
⚠️ I still prefer `rusqlite_migration` as the default for v1.

## 4. `redb`, `heed`, and `fjall` are better thought of as adjunct stores unless truth is truly KV-shaped

This pass made the distinction even sharper.

### `redb`

Very compelling for:

- pure Rust
- stable file format
- ACID KV truth
- append-ish mirrors

### `heed`

Strong if you explicitly want:

- LMDB semantics
- typed high-performance KV

### `fjall`

Strong signal if you want:

- log-structured engine
- heavy KV/write-oriented behavior

But for this package, the key question remains:

**is the source of truth fundamentally relational/queryable or primarily KV/LSM?**

Current answer:

🔥 It looks primarily relational/queryable, with some append/blob adjunct surfaces.

So these stores are strongest as:

- replay mirrors
- spill stores
- blob stores
- secondary projections

not as the first durable truth center.

## 5. Append-only event log is not the same as semantic timeline or full truth

This is one of the most important architectural rules of this pass.

For this package we likely need at least three durable concepts:

### 1. structured truth

Examples:

- session row
- workspace topology metadata
- route/security metadata
- projection generations

### 2. append-only operational log

Examples:

- attach/detach events
- process launch/exit
- replay checkpoints
- output-overflow markers

### 3. semantic timeline

Examples:

- command summaries
- tool usage summaries
- user-visible execution entries

🔥 These are not interchangeable.

If they collapse into one table or one blob:

- migrations get ugly
- rebuild rules get vague
- product semantics drift

## 6. Projection rebuild policy should be first-class

A strong durable design for this runtime should likely track at least:

- schema version
- snapshot format version
- projection generation

So startup can do something like:

1. open durable stores
2. run schema migrations
3. validate snapshot/blob format support
4. invalidate projections if generation changed
5. rebuild projections lazily or eagerly

🔥 This is much healthier than treating all read models as permanent truth.

It lines up with earlier conclusions:

- local search index is derived
- semantic analyzer cache is derived
- screen projections are derived

## 7. `sqlx` and `sea-orm-migration` are poor defaults here

This research pass made that clearer too.

### `sqlx`

Current latest crate line visible from cargo is `0.9.0-alpha.1`.

Even ignoring the alpha, the architectural fit is weaker because:

- async DB abstraction is not the central need
- multi-backend toolkit is unnecessary weight
- the runtime is embedded/local-first, not DB-platform-first

### `sea-orm-migration`

Current visible line is `2.0.0-rc.38`.

It looks much more like:

- ORM ecosystem support
- broader app/database platform tooling

than a tight fit for this package.

Practical takeaway:

⚠️ Useful reference points.  
❌ Not my default durable-state foundation.

## 8. `deadpool-sqlite` is an outer-leaf concern, not storage architecture

`deadpool-sqlite 0.13.0` can be useful when:

- outer daemon code wants async pool ergonomics
- host-facing async tasks should not juggle direct SQLite connection ownership

But this is not a reason to reshape the durable model.

It should stay:

- outer leaf
- operational convenience

not the center of storage design.

## 9. `sqlite-vfs` and `sqlite-loadable` are interesting future seams, not v1 truth-model decisions

### `sqlite-vfs`

This is interesting if later you need:

- custom backing medium
- special durability behavior
- alternative storage surfaces

### `sqlite-loadable`

Interesting if later you want:

- SQLite extensions
- custom SQL-side helpers
- local search/vector/index experiments

But both are advanced seams.

🔥 They do not change the main conclusion:

first get the durable truth model right, then decide whether SQLite itself needs exotic extension points.

## 10. `bonsaidb` is interesting software, but the wrong center of gravity here

`bonsaidb 0.5.0` is much more database-product-shaped than terminal-runtime-shaped.

It may be useful for someone building:

- a larger app platform
- a programmable DB-centric system

But for this package it is the wrong abstraction center.

Practical takeaway:

❌ Not my default path for a local terminal runtime.

## What I would actually build

For this package I would shape durable state like this:

### Structured truth in SQLite

- sessions
- workspace/window/tab/pane metadata
- route and authority metadata
- restore bookkeeping
- projection versions

### Operational append tables in SQLite

- lifecycle events
- replay/overflow markers
- attach/detach records

### Versioned snapshot/blob layer

- committed screen snapshots
- serialized replay checkpoints
- maybe binary payloads compressed and checksummed

### Optional adjunct store only if proven necessary

- `redb` or `fjall` for heavier replay/blob/spill surfaces

### Derived projections rebuilt deliberately

- search indices
- semantic timeline views
- screen/text read models

## Practical recommendations

- ✅ Use SQLite as the evolving structured truth center
- ✅ Prefer `rusqlite_migration` as the default embedded migration tool
- ✅ Keep append-only operational logs separate from semantic timeline
- ✅ Version snapshots/blobs separately from relational schema
- ✅ Treat read models as rebuildable
- ✅ Add `redb`/`fjall` only when blob/replay pressure proves it
- ⚠️ Keep `refinery` as a stronger-but-heavier migration option
- ⚠️ Keep `deadpool-sqlite` as an operational leaf only
- ⚠️ Keep `sqlite-vfs` and `sqlite-loadable` for later advanced seams
- ❌ Do not let one KV store become accidental truth for everything
- ❌ Do not let one serialized blob become the only durable model

## Sources

- [rusqlite](https://github.com/rusqlite/rusqlite)
- [rusqlite_migration](https://github.com/cljoly/rusqlite_migration)
- [refinery](https://github.com/rust-db/refinery)
- [redb](https://github.com/cberner/redb)
- [heed](https://github.com/meilisearch/heed)
- [fjall](https://github.com/fjall-rs/fjall)
- [deadpool-sqlite](https://docs.rs/deadpool-sqlite)
- [sqlite-vfs](https://github.com/rkusa/sqlite-vfs)
- [sqlite-loadable-rs](https://github.com/asg017/sqlite-loadable-rs)
- [sqlx](https://github.com/launchbadge/sqlx)
- [SeaORM](https://github.com/SeaQL/sea-orm)
- [bonsaidb](https://github.com/khonsulabs/bonsaidb)
