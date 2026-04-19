# Deep Dive - Rust Font Shaping, Glyph Cache And Render Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для terminal package с универсальной embed-story очень легко совершить одну дорогую архитектурную ошибку:

🔥 **перепутать terminal truth с text rendering stack**

Как только это происходит, package начинает незаметно срастаться с:

- font discovery
- shaping engine
- glyph cache
- rasterization backend
- GPU / canvas / native text stack

Для standalone terminal app это кажется нормальным.  
Для universal embeddable package это уже опасно.

Потому что у вас цель другая:

- UI today on JS
- tomorrow on anything
- same runtime inside Electron, standalone app, or another host

Значит нужно жёстко понять:

1. what belongs to terminal/runtime truth
2. what belongs to renderer leafs
3. what can exist as an optional reference renderer stack

## Primary Sources

- [`swash` docs](https://docs.rs/swash/0.2.7/swash/)
- [`skrifa` docs](https://docs.rs/skrifa/0.42.0/skrifa/)
- [`rustybuzz` docs](https://docs.rs/rustybuzz/)
- [`fontdb` docs](https://docs.rs/fontdb/)
- [`cosmic-text` docs](https://docs.rs/cosmic-text/latest/cosmic_text/)
- [`parley` docs](https://docs.rs/parley/0.8.0/parley/)
- [`glyphon` docs](https://docs.rs/glyphon/0.11.0/glyphon/)
- [`ab_glyph` docs](https://docs.rs/ab_glyph/)

## Freshness signals

- `swash 0.2.7` - repo `dfrg/swash`, `839` stars, updated `2026-04-16`
- `skrifa 0.42.0` - repo `googlefonts/fontations`, `749` stars, updated `2026-04-16`
- `rustybuzz 0.20.1` - repo `harfbuzz/rustybuzz`, `658` stars, updated `2026-04-10`
- `fontdb 0.23.0` - repo `RazrFalcon/fontdb`, `167` stars, updated `2026-04-07`
- `cosmic-text 0.18.2` - repo `pop-os/cosmic-text`, `2038` stars, updated `2026-04-19`
- `parley 0.8.0` - repo `linebender/parley`, `575` stars, updated `2026-04-17`
- `glyphon 0.11.0` - repo `grovesNL/glyphon`, `715` stars, updated `2026-04-17`
- `ab_glyph 0.2.32` - repo `alexheretic/ab-glyph`, `441` stars, updated `2026-04-17`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**font shaping, font discovery and glyph caching should not live in the universal terminal core**

Healthy shape now looks like:

1. runtime owns cells, graphemes, styles, wraps and projections
2. renderer leaves own fonts, shaping and glyph caches
3. optional Rust reference renderers may exist as separate crates
4. host-neutral API should expose text clusters and style projections, not glyph IDs or atlas coordinates

Иначе package быстро перестаёт быть universal runtime и становится terminal app framework with one preferred renderer worldview.

## Top 3 directions for the render/text stack boundary

### 1. `Glyph-agnostic runtime core + optional renderer leaves`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк.

Что это значит:

- runtime core exports:
  - cells
  - grapheme/text clusters
  - style attributes
  - cursor and selection state
  - dirty regions / deltas
- renderer leaf decides:
  - font fallback
  - shaping
  - glyph atlas/cache
  - raster backend

Почему это strongest path:

- keeps runtime truly host-neutral
- keeps Electron/web/native adapters equally possible
- lets you ship a standalone app later without forcing the same text stack on every embedder

Практический вывод:

✅ Это мой strongest default для вашего проекта.

### 2. `Shared optional Rust text stack for reference renderer crates`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `6000-12000` строк.

Что это значит:

- runtime core still stays glyph-agnostic
- but you publish optional crates like:
  - `terminal-render-text`
  - `terminal-render-wgpu`
  - `terminal-render-canvas`
- they may standardize around one Rust text stack

Почему это интересно:

- gives a strong default renderer path for standalone app and some hosts
- avoids each adapter reinventing font fallback and shaping

Где риск:

- if this stack becomes too central, it may start pressuring the core contract
- some hosts may not want your default text engine at all

Практический вывод:

✅ Хороший phase 2 direction.  
⚠️ But still keep it outside the core.

### 3. `Font shaping and glyph cache inside core runtime`

`🎯 3   🛡️ 4   🧠 8`  
Примерно `7000-15000` строк на старт и потом дорого чинить.

Что это значит:

- runtime owns font lookup, shaping and maybe raster cache
- host gets glyph-oriented output or renderer-coupled artifacts

Почему это плохо:

- couples runtime to rendering worldview
- complicates host-neutrality
- makes non-Rust or web hosts pay for decisions they did not ask for
- blurs terminal truth with font engine behavior

Практический вывод:

❌ Для universal package я бы этот путь не брал.

## Tool-by-tool findings

## 1. `swash` - strongest low-level shaping and glyph rendering brick

- Crate: [`swash`](https://crates.io/crates/swash)
- Latest: `0.2.7`
- Repo stars: `839`
- Repo updated: `2026-04-16`

Что особенно важно:

- crate explicitly positions itself for:
  - font introspection
  - complex text shaping
  - glyph rendering

Почему это важно:

- this is exactly the kind of capability you may want in an optional renderer leaf
- it is powerful enough that, if pulled into core too early, it can accidentally define architecture

Итог:

✅ Strong optional renderer brick.  
⚠️ Not core runtime truth.

## 2. `skrifa` - strongest low-level font metadata/scaling brick

- Crate: [`skrifa`](https://crates.io/crates/skrifa)
- Latest: `0.42.0`
- Repo stars: `749`
- Repo updated: `2026-04-16`

Что особенно важно:

- explicitly a metadata reader and glyph scaler for OpenType fonts
- part of a serious modern font stack

Итог:

✅ Great low-level optional renderer dependency.  
⚠️ Should stay below a renderer or shaping leaf, not in the universal terminal contract.

## 3. `rustybuzz` - strongest focused shaping engine donor

- Crate: [`rustybuzz`](https://crates.io/crates/rustybuzz)
- Latest: `0.20.1`
- Repo stars: `658`
- Repo updated: `2026-04-10`

Что особенно важно:

- explicitly a HarfBuzz shaping algorithm port to Rust

Почему это важно:

- if you ever need terminal-adjacent rich shaping in a Rust renderer leaf, this is a strong focused choice
- but for a cell-based terminal runtime, shaping should remain downstream of screen truth

Итог:

✅ Strong shaping donor.  
⚠️ Keep it in renderer leaves.

## 4. `fontdb` - useful system-font discovery, but definitely a leaf concern

- Crate: [`fontdb`](https://crates.io/crates/fontdb)
- Latest: `0.23.0`
- Repo stars: `167`
- Repo updated: `2026-04-07`

Что особенно важно:

- in-memory font database with CSS-like queries
- useful for system font resolution and fallback

Почему это not core:

- font discovery is host and platform specific
- embedders may already own their own font story

Итог:

✅ Useful renderer/app leaf tool.  
❌ Not something I would make part of the terminal core contract.

## 5. `cosmic-text` - strongest integrated Rust text engine, but too UI-oriented for core truth

- Crate: [`cosmic-text`](https://crates.io/crates/cosmic-text)
- Latest: `0.18.2`
- Repo stars: `2038`
- Repo updated: `2026-04-19`

Что особенно важно:

- positions itself as pure Rust multi-line text handling
- pulls together fontdb, shaping, locale and text layout concerns

Почему это very interesting:

- excellent candidate for a standalone Rust terminal app renderer leaf
- strong reference for optional text stack design

Почему it should stay out of core:

- it solves richer multi-line text layout, not just terminal cell truth
- it brings strong renderer assumptions

Итог:

✅ Strong optional renderer/reference stack.  
⚠️ Too high-level and UI-oriented for the universal core.

## 6. `parley` - powerful rich text layout engine, also a leaf concern

- Crate: [`parley`](https://crates.io/crates/parley)
- Latest: `0.8.0`
- Repo stars: `575`
- Repo updated: `2026-04-17`

Что особенно важно:

- explicitly provides an API for implementing rich text layout

Почему это полезно:

- strong future seam for richer standalone app surfaces
- useful if the project later grows not only a terminal, but adjacent text-heavy panes

Почему not core:

- rich text layout is a different problem from terminal state truth

Итог:

✅ Valuable renderer/app-shell donor.  
❌ Not part of the universal terminal core.

## 7. `glyphon` - renderer backend, not text truth

- Crate: [`glyphon`](https://crates.io/crates/glyphon)
- Latest: `0.11.0`
- Repo stars: `715`
- Repo updated: `2026-04-17`

Что особенно важно:

- explicitly targets fast 2D text rendering for `wgpu`

Почему это matters architecturally:

- it is exactly the kind of thing that belongs in a renderer backend crate
- not in host-neutral terminal runtime semantics

Итог:

✅ Good renderer-backend leaf.  
❌ Definitely not core contract.

## 8. `ab_glyph` - simple glyph/raster helper, but too primitive for shaping-heavy leafs

- Crate: [`ab_glyph`](https://crates.io/crates/ab_glyph)
- Latest: `0.2.32`
- Repo stars: `441`
- Repo updated: `2026-04-17`

Что особенно важно:

- useful for loading, scaling, positioning and rasterizing OpenType glyphs

Почему it is limited here:

- more useful for simpler rendering paths
- not enough by itself for a serious modern shaping-heavy renderer stack

Итог:

⚠️ Useful helper, not my first choice for the main text leaf.

## Recommended architecture rules

### 1. Keep glyph IDs and atlas details out of the host-neutral contract

Public contract should expose:

- cluster text
- cell/style metadata
- cursor/selection state
- dirty regions / snapshots

Not:

- glyph atlas slots
- font face handles
- shaping engine internals

### 2. Font discovery is a leaf concern

The core should not require:

- system font database assumptions
- fontconfig assumptions
- one platform’s font fallback rules

### 3. Keep cell truth above font truth

Terminal runtime decides:

- wrap
- width semantics
- occupancy
- selection and copy ranges

Renderer leaf decides:

- which font renders that cluster
- how the cluster is shaped
- how glyphs are cached and painted

### 4. Optional reference renderers are healthy

If you want:

- standalone Rust app
- reference desktop renderer
- performance comparison harness

then separate optional renderer crates are a good idea.

### 5. Renderer choice must not change terminal semantics

This is the real architecture test.

If changing renderer crate changes:

- selection
- wrap
- cursor movement
- search hit mapping

then too much truth leaked out of the core.

## Bottom line

Если свести весь deep dive к одной фразе:

🔥 **a universal Rust terminal package should stay glyph-agnostic at its core and treat shaping, font lookup and glyph caches as optional renderer-leaf concerns**

Именно это keeps the runtime:

- host-neutral
- embeddable
- cleanly layered
- and still capable of shipping strong optional standalone renderers later.
