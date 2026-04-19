# Deep Dive - Rust Mux, Layout Trees, And Workspace Shell Topology

**Проверено**: 2026-04-19

## Зачем этот слой важен

Когда terminal package проектируется как reusable Rust platform, очень легко сделать одну из двух ошибок:

- начать думать, что tabs/panes/splits - это просто UI layout
- или наоборот построить runtime truth вокруг слишком общего tree/graph crate

Но у серьёзного terminal runtime topology itself is domain truth:

- порядок окон и вкладок
- active and last-active semantics
- split tree per tab
- zoom / floating / suppressed pane modes
- pane groups and multi-select state
- restore metadata and dirty-layout semantics

🔥 Если этот слой смоделировать неаккуратно, потом очень быстро появляются:

- god-object `Tab`
- путаница между live topology и persisted layout
- невозможность нормально сделать restore, attach/detach и multi-host embedding
- слишком общий graph API там, где домен на самом деле ordered and split-oriented

## Primary Sources

### WezTerm mux internals

- [`wezterm/mux/src/window.rs`](https://github.com/wezterm/wezterm/blob/main/mux/src/window.rs)
- [`wezterm/mux/src/tab.rs`](https://github.com/wezterm/wezterm/blob/main/mux/src/tab.rs)

### Zellij server topology internals

- [`zellij-server/src/pane_groups.rs`](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/pane_groups.rs)
- [`zellij-server/src/session_layout_metadata.rs`](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/session_layout_metadata.rs)
- [`zellij-server/src/tab/mod.rs`](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/tab/mod.rs)

### Candidate topology/data-structure crates

- [`slotmap` docs](https://docs.rs/slotmap)
- [`indextree` docs](https://docs.rs/indextree)
- [`id_tree` docs](https://docs.rs/id_tree)
- [`petgraph` docs](https://docs.rs/petgraph)
- [`im` docs](https://docs.rs/im)
- [`bintree` docs](https://docs.rs/bintree)

## Freshness signals

- `wezterm/wezterm` - `25.6k` stars, pushed `2026-04-01`
- `zellij-org/zellij` - `31.7k` stars, pushed `2026-04-17`
- `raphamorim/rio` - `6.7k` stars, pushed `2026-04-19`
- `slotmap 1.1.1` - repo `orlp/slotmap`, `1308` stars, pushed `2026-04-18`
- `indextree 4.8.1` - repo `saschagrunert/indextree`, `785` stars, pushed `2026-04-17`
- `petgraph 0.8.3` - repo `petgraph/petgraph`, `3851` stars, pushed `2026-04-04`
- `im 15.1.0` - repo `bodil/im-rs`, `1583` stars, pushed `2024-08-19`
- `id_tree 1.8.0` - repo `iwburns/id-tree`, `56` stars, pushed `2024-04-17`
- `bintree 0.1.0` - old thin crate, still useful as a donor signal because WezTerm uses a binary-tree shape for pane splits

## Короткий вывод

🔥 Самый сильный topology pattern для reusable terminal runtime сейчас выглядит так:

1. ordered containers for workspaces, windows and tabs
2. dedicated binary split tree per tab
3. stable pane and tab identities
4. explicit projections for positioned panes and geometry
5. separate persisted layout metadata from live runtime topology
6. separate bounded contexts for pane groups, zoom/floating state and restore dirtiness

То есть healthiest architecture is **not**:

- generic graph everywhere
- one mega `Tab` aggregate
- persistent immutable structure as the hot runtime truth

## Top 3 directions for runtime topology

### 1. `Ordered containers + dedicated split tree + explicit layout metadata`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Это мой текущий **лучший default**.

Идея:

- `Workspace` owns ordered windows
- `Window` owns ordered tabs
- `Tab` owns a split tree and active pane identity
- projections like `PositionedPane` or `LayoutSnapshot` are derived
- restore metadata is a separate aggregate, not the same object as live tab state

Почему это strongest path:

- matches the real terminal domain much better than generic graph abstractions
- keeps ordering first-class where order is actual truth
- makes restore, persistence and multi-host projections easier to reason about
- follows the strongest donor patterns from WezTerm and the best separated parts of Zellij

### 2. `Slotmap-owned aggregates + arena tree helper where needed`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `8000-16000` строк.

Идея:

- stable IDs through `slotmap`
- tree nodes in an arena-like helper such as `indextree`
- owner aggregates still define semantics
- generic tree helper is only infrastructure, not domain truth

Почему это интересно:

- good if layout model may later grow beyond a very strict binary split tree
- explicit `NodeId`-style handles can be convenient for editing and restore tools
- works well if you want host-independent topology editing APIs

Где риск:

- easy to over-generalize
- helper API can start dictating your domain model
- more room for generic tree mutation logic creeping into business rules

### 3. `Graph-first or persistent-collection-first topology`

`🎯 4   🛡️ 5   🧠 7`  
Примерно `9000-18000` строк.

Идея:

- model most runtime topology through `petgraph`, `im` or similar general structures
- rely on generic graph/tree algorithms as the center of the runtime model

Почему это weaker path:

- tabs and panes are not a generic graph problem most of the time
- domain ordering becomes too implicit
- active/last-active/focus/zoom semantics become harder to keep obvious
- restore and projection code starts compensating for the abstraction mismatch

Практический вывод:

⚠️ Good for side indexes, topology analysis, persisted snapshot helpers or experimentation.  
❌ Bad as the core runtime truth for panes and tab layouts.

## 1. WezTerm shows the healthiest basic shape: ordered windows/tabs, tree-shaped panes

`mux/src/window.rs` is very revealing.

`Window` stores:

- `tabs: Vec<Arc<Tab>>`
- `active`
- `last_active`
- `workspace`
- title and initial position metadata

Это очень важный signal:

🔥 **window/tab order is domain truth, not a graph traversal problem**

Tabs are ordered because users feel that order directly:

- tab strip order
- recent/active movement
- restore order
- keyboard navigation

This is why `Vec<Tab>` is actually a healthy domain choice here.

Then `mux/src/tab.rs` shows the next level:

- `Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>`
- `Cursor = bintree::Cursor<Arc<dyn Pane>, SplitDirectionAndSize>`
- dedicated `SplitDirection`, `SplitSize`, `SplitRequest`
- derived `PositionedPane` and `PositionedSplit`

That is the strongest donor pattern in this whole layer:

🔥 **tab layout tree and positioned geometry are separate concepts**

Do not collapse:

- live split topology
- focus semantics
- computed pane rectangles

into one structure.

## 2. The exact `bintree` dependency matters less than the domain shape it implies

WezTerm currently uses `bintree 0.1.0`.

Important signal:

- the crate is tiny and old
- it is not the thing that makes WezTerm good

What matters is the **shape**:

- a strict split tree
- explicit split metadata on internal edges
- panes as leaves
- derived positioned output on demand

Practical takeaway:

✅ copy the domain shape  
⚠️ do not blindly copy the exact dependency choice

For our package this likely means:

- either a small custom split-tree domain type
- or a narrow helper behind an internal adapter seam

not making `bintree` itself a foundational public dependency.

## 3. Zellij proves pane groups deserve a separate bounded context

`zellij-server/src/pane_groups.rs` is one of the most useful smaller files in the research.

It keeps:

- `HashMap<ClientId, Vec<PaneId>>`
- explicit grouping and ungrouping operations
- all-clients vs per-client behavior

That gives a very strong architectural hint:

🔥 **group selection is not "just another flag on a pane"**

For a reusable terminal runtime this should likely be its own bounded context:

- `PaneGroupService`
- `SelectionGroupStore`
- or equivalent

Why:

- multi-pane selection is client-facing state
- may vary per client/viewer
- should not bloat the core split-tree aggregate

## 4. Zellij also proves restore metadata should not be the same thing as live runtime topology

`session_layout_metadata.rs` is another very valuable donor.

It keeps:

- `default_layout`
- `global_cwd`
- `default_shell`
- `default_editor`
- `tabs: Vec<TabLayoutMetadata>`
- an explicit `is_dirty()` comparison against the base layout

This is one of the cleanest restore lessons in the whole research:

🔥 **persisted layout metadata is a different aggregate from live runtime topology**

That means our package should separate:

- `LiveWorkspaceTopology`
- `PersistedLayoutMetadata`
- `RestoreDiff` or equivalent dirty/layout delta concept

instead of serializing the hottest in-memory `Tab` object and pretending that is the durable truth.

## 5. Zellij `Tab` is also a warning: do not let it become a god aggregate

`zellij-server/src/tab/mod.rs` is useful and dangerous at the same time.

It gathers a lot:

- tiled panes
- floating panes
- suppressed panes
- viewport state
- display area
- connected clients
- swap layouts
- mouse handling
- clipboard
- image/sixel stores
- pending VTE events

This is understandable for a terminal product, but it is also a warning.

⚠️ If our `Tab` aggregate absorbs:

- split topology
- renderer viewport state
- selection groups
- clipboard state
- shell integration phase
- layout persistence
- host-specific interaction details

then reusable architecture starts collapsing fast.

Practical takeaway:

- `TabAggregate` should stay small and topology-focused
- view/interactions should live in adjacent services or projections
- restore/layout metadata should be separate
- selection groups should be separate

## 6. `slotmap` is still the best internal identity backbone

For this layer `slotmap 1.1.1` remains the strongest default.

Why:

- pane IDs and tab IDs need stable opaque identity
- internal removals should not create naive ABA-like confusion
- runtime services can hold stable handles without leaking raw indices outward

This lines up very well with the earlier handle-model conclusions:

- public hosts see UUID/ULID-style opaque handles
- runtime internals can use `slotmap` keys

🔥 This is especially useful when layout trees and ownership tables need cross-references without exposing implementation indices.

## 7. `indextree` is the most interesting helper if we ever need a more explicit tree editor seam

`indextree 4.8.1` is the healthiest tree-helper candidate I found for this layer.

Why it is interesting:

- arena-based
- index-driven, not ref-count driven
- good parent/child mutation ergonomics
- active repo and reasonable adoption

Why it is not my default center:

- terminal pane layout is often stricter than a generic n-ary tree
- once binary split semantics are first-class, a generic tree can be more freedom than value

Practical takeaway:

✅ Good helper if we expose layout-edit operations or more general tree tools.  
⚠️ Not a reason by itself to abandon a stricter split-tree domain model.

## 8. `id_tree` is weaker than `indextree` for this use

`id_tree 1.8.0` is not unusable, but current signals are weaker:

- much smaller repo gravity
- less fresh activity
- less compelling fit for a serious long-lived runtime core

Practical takeaway:

⚠️ Fine for small apps or experiments.  
❌ Not my preferred tree-helper baseline for a world-class reusable package.

## 9. `petgraph` is powerful, but pane layout truth is not a graph problem

`petgraph 0.8.3` is a very solid crate.

But for terminal topology the key question is not "can this be represented as a graph?"

It obviously can.

The key question is:

**does a generic graph make the domain clearer?**

Usually for panes/tabs/splits the answer is no.

What `petgraph` is better for:

- remote route graphs
- workspace relationship analysis
- diagnostics and topology tooling
- side indexes

What it is weaker for:

- ordered tab strips
- focus/active/last-active semantics
- binary split layout truth

## 10. `im` is better for snapshots and history than for hot runtime truth

`im 15.1.0` is still interesting, but its best fit here is not the main runtime graph.

Good uses:

- topology snapshots
- undo/redo-like editing flows
- restore drafts
- immutable layout metadata revisions

Weaker use:

- hot mutable session topology
- frequently changing focus and split operations
- ownership-heavy runtime truth

🔥 Practical takeaway:

persistent immutable collections are much better as **snapshot/history tools** than as the live core of terminal mux state.

## What I would actually build

If I were shaping this layer for the package right now:

1. `Workspace` and `Window` use ordered containers
2. `Tab` owns a strict split-tree domain object
3. `PaneId`, `TabId`, `WindowId` use stable internal keys with opaque public wrappers
4. `PositionedPane` and `LayoutProjection` are derived objects
5. `PaneGroups` is a separate bounded context
6. `PersistedLayoutMetadata` is a separate aggregate from live topology
7. tree-helper crates remain optional helpers, not the source of truth

That gives the cleanest path for:

- Electron embedding
- future standalone app
- future remote runtime
- future restore/snapshot tooling
- multi-language host SDKs

## Practical recommendations

- ✅ Treat tab and window order as first-class domain truth
- ✅ Keep split topology separate from projected geometry
- ✅ Keep pane groups separate from core layout aggregates
- ✅ Keep persisted layout metadata separate from live runtime topology
- ✅ Prefer `slotmap` for internal identity
- ✅ Use `indextree` only if a more general tree editor seam is truly needed
- ⚠️ Treat `petgraph` as an auxiliary tool, not the pane-layout center
- ⚠️ Treat `im` as a snapshot/history helper, not the hot runtime truth
- ❌ Do not let `Tab` become a god aggregate

## Sources

- [WezTerm window.rs](https://github.com/wezterm/wezterm/blob/main/mux/src/window.rs)
- [WezTerm tab.rs](https://github.com/wezterm/wezterm/blob/main/mux/src/tab.rs)
- [Zellij pane_groups.rs](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/pane_groups.rs)
- [Zellij session_layout_metadata.rs](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/session_layout_metadata.rs)
- [Zellij tab/mod.rs](https://github.com/zellij-org/zellij/blob/main/zellij-server/src/tab/mod.rs)
- [slotmap](https://docs.rs/slotmap)
- [indextree](https://docs.rs/indextree)
- [id_tree](https://docs.rs/id_tree)
- [petgraph](https://docs.rs/petgraph)
- [im](https://docs.rs/im)
- [bintree](https://docs.rs/bintree)
