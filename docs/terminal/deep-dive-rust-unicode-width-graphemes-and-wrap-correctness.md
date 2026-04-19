# Deep Dive - Rust Unicode Width, Graphemes And Wrap Correctness

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для terminal runtime мирового уровня очень опасно недооценить один слой:

🔥 **Unicode semantics are not presentation sugar**

Если их спроектировать небрежно, то ломается не только красота рендера, а реальные terminal semantics:

- cursor position
- cell occupancy
- soft wrap
- selection
- copy
- search hit mapping
- link ranges
- restore correctness

UI потом уже не сможет это честно исправить, даже если он будет очень красивым.

Особенно это критично для вашей цели, где:

- UI is a separate layer
- Rust runtime must be host-neutral
- package should work across different hosts and languages

Это означает очень жёсткое архитектурное правило:

**host UI should not become the place where width, grapheme and wrap truth is decided**

## Primary Sources

- [`unicode-width` docs](https://docs.rs/unicode-width/0.2.2/unicode_width/)
- [`unicode-segmentation` docs](https://docs.rs/unicode-segmentation/1.13.2/unicode_segmentation/)
- [`unicode-normalization` docs](https://docs.rs/unicode-normalization/)
- [`unicode-bidi` docs](https://docs.rs/unicode-bidi/)
- [`unicode-linebreak` docs](https://docs.rs/unicode-linebreak/0.1.5/unicode_linebreak/)
- [`unicode-display-width` docs](https://docs.rs/unicode-display-width/0.3.0/unicode_display_width/)
- [`finl_unicode` docs](https://docs.rs/finl_unicode/1.4.0/finl_unicode/)
- [`textwrap` docs](https://docs.rs/textwrap/)

## Freshness signals

- `unicode-width 0.2.2` - repo `unicode-rs/unicode-width`, `298` stars, updated `2026-04-15`
- `unicode-segmentation 1.13.2` - repo `unicode-rs/unicode-segmentation`, `657` stars, updated `2026-04-17`
- `unicode-normalization 0.1.25`
- `unicode-bidi 0.3.18` - repo `servo/unicode-bidi`, `84` stars, updated `2026-04-14`
- `unicode-linebreak 0.1.5` - repo `axelf4/unicode-linebreak`, `39` stars, updated `2026-02-22`
- `unicode-display-width 0.3.0` - repo `jameslanska/unicode-display-width`, `24` stars, updated `2026-01-19`
- `finl_unicode 1.4.0` - repo `dahosek/finl_unicode`, `23` stars, updated `2025-10-17`
- `textwrap 0.16.2` - repo `mgeisler/textwrap`, `518` stars, updated `2026-03-27`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**Unicode width, grapheme and wrap semantics should live in the Rust runtime close to the emulator and projection layers, not in the host UI**

Healthy shape now looks like:

1. runtime resolves width and cell occupancy
2. runtime owns grapheme-aware cursor/selection/copy semantics
3. host UI consumes screen snapshots and deltas, not raw text plus "please rewrap"
4. normalization, bidi and generic line breaking stay narrow and explicit

Если сделать наоборот, то получится:

- different hosts rendering the same session differently
- broken selection on ZWJ emoji and combining sequences
- wrapped links and search hits drifting from actual screen cells
- restore and replay surfaces disagreeing with what the user saw

## Top 3 directions for Unicode/text correctness

### 1. `Emulator-owned width/wrap truth + unicode-width + unicode-segmentation + narrow normalization`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-10000` строк.

Что это значит:

- width and wrap truth stays inside runtime
- `unicode-width` is the default width oracle
- `unicode-segmentation` is the default grapheme boundary tool
- `unicode-normalization` is used only in narrow derived surfaces
- host renderers receive already-resolved screen projections

Почему это strongest path:

- fits terminal semantics much better than browser-first or UI-first text handling
- keeps multiple host UIs consistent
- lines up with how real emulator cores think about cells and wraps

Где риск:

- you need to model ambiguous width policy explicitly
- selection/copy/search must consume the same runtime truth, not local host heuristics

Практический вывод:

✅ Это мой лучший default for a reusable terminal package.

### 2. `Default runtime truth + alternate adjunct stack around unicode-display-width / finl_unicode`

`🎯 7   🛡️ 7   🧠 6`  
Примерно `4000-8000` строк.

Что это значит:

- core still owns semantics
- but you keep alternative width/segmentation tools available for:
  - comparison harnesses
  - export surfaces
  - future specialized renderers

Почему это интересно:

- `unicode-display-width` explicitly targets terminal-like display width
- `finl_unicode` gives another segmentation implementation to compare behavior

Где риск:

- ecosystem gravity is noticeably smaller
- adding multiple truth candidates too early can blur the contract

Практический вывод:

✅ Good as an evaluation seam or adjunct layer.  
⚠️ Not my first truth model for v1.

### 3. `Host-renderer-owned Unicode and wrap semantics`

`🎯 2   🛡️ 3   🧠 4`  
Примерно `2000-5000` строк на старт и потом дорого чинить.

Что это значит:

- Rust runtime sends mostly text and rough metadata
- each host renderer computes width, wrap, selection and maybe link mapping itself

Почему это заманчиво:

- seems simple
- browser and native toolkits already know a lot about text

Почему это плохой path:

- terminal cells are not the same thing as general text layout
- different hosts will disagree
- replay, restore and multi-host correctness get much worse

Практический вывод:

❌ Для world-class terminal runtime я бы этот путь не брал.

## Tool-by-tool findings

## 1. `unicode-width` - strongest default width oracle

- Crate: [`unicode-width`](https://crates.io/crates/unicode-width)
- Latest: `0.2.2`
- Repo stars: `298`
- Repo updated: `2026-04-15`

Что особенно важно:

- crate explicitly says it determines displayed width according to UAX #11 rules
- default feature set includes `cjk`
- docs.rs source notes ambiguous width chars are treated as wide under `cjk`

Почему это важно:

- ambiguous-width policy is not a minor detail
- it affects:
  - cursor movement
  - wrap
  - cell occupancy
  - selection ranges

Итог:

✅ Strongest default width brick.  
⚠️ CJK/ambiguous-width policy should be a deliberate product setting, not an accident.

## 2. `unicode-segmentation` - strongest default grapheme boundary tool

- Crate: [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)
- Latest: `1.13.2`
- Repo stars: `657`
- Repo updated: `2026-04-17`

Что особенно важно:

- crate explicitly implements grapheme, word and sentence boundaries according to UAX #29
- this is exactly what you need for:
  - cursor left/right by grapheme
  - selection expansion
  - copy semantics over emoji / ZWJ / combining marks

Итог:

✅ Strongest default grapheme tool for runtime semantics.

## 3. `unicode-normalization` - useful, but only in narrow derived seams

- Crate: [`unicode-normalization`](https://crates.io/crates/unicode-normalization)
- Latest: `0.1.25`

Что особенно важно:

- crate provides canonical and compatible decomposition/recomposition per UAX #15

Почему it is dangerous if overused:

- terminal transcript truth should not be silently rewritten
- normalization may be right for:
  - search matching
  - semantic analyzers
  - export/indexing
- but wrong as a blanket transform over live terminal bytes

Итог:

✅ Good derived-surface tool.  
⚠️ Not part of primary screen truth.

## 4. `unicode-linebreak` - strong adjunct for non-terminal line breaking

- Crate: [`unicode-linebreak`](https://crates.io/crates/unicode-linebreak)
- Latest: `0.1.5`
- Repo stars: `39`
- Repo updated: `2026-02-22`

Что особенно важно:

- crate explicitly implements the Unicode Line Breaking Algorithm

Почему это полезно:

- transcript export
- side panels
- rich summaries
- non-terminal text surfaces near the terminal

Почему это should stay narrow:

- terminal soft wrap is not generic prose line breaking
- cell occupancy and screen columns still dominate terminal semantics

Итог:

✅ Good adjunct tool.  
⚠️ Do not let it replace terminal wrap truth.

## 5. `textwrap` - good donor for adjacent text surfaces, not core terminal truth

- Crate: [`textwrap`](https://crates.io/crates/textwrap)
- Latest: `0.16.2`
- Repo stars: `518`
- Repo updated: `2026-03-27`

Что особенно важно:

- by default it already composes `unicode-linebreak` and `unicode-width`
- great for prose and export-oriented wrapping

Почему не core terminal tool:

- it solves text layout and wrapping, not emulator cell truth
- terminal wrap, wide chars, cursor state and damage tracking need stricter semantics

Итог:

✅ Excellent donor for export/help/transcript UI.  
❌ Not the core runtime wrap model.

## 6. `unicode-display-width` - interesting alternative width engine

- Crate: [`unicode-display-width`](https://crates.io/crates/unicode-display-width)
- Latest: `0.3.0`
- Repo stars: `24`
- Repo updated: `2026-01-19`

Что особенно важно:

- crate explicitly positions itself as Unicode 15.1 compliant display width utility
- interesting as a comparison or adjunct width layer

Почему not my first default:

- lower ecosystem gravity
- less battle-tested in terminal stacks than `unicode-width`

Итог:

⚠️ Strong adjunct or comparison harness tool.  
✅ Worth watching.

## 7. `finl_unicode` - interesting alternative segmentation donor

- Crate: [`finl_unicode`](https://crates.io/crates/finl_unicode)
- Latest: `1.4.0`
- Repo stars: `23`
- Repo updated: `2025-10-17`

Что особенно важно:

- provides Unicode categories and grapheme segmentation
- could be useful as an alternate evaluation seam

Почему not my first default:

- much lower ecosystem gravity
- narrower adoption

Итог:

⚠️ Good donor or comparison tool, not my default truth layer.

## 8. `unicode-bidi` - explicit future seam, not accidental core transform

- Crate: [`unicode-bidi`](https://crates.io/crates/unicode-bidi)
- Latest: `0.3.18`
- Repo stars: `84`
- Repo updated: `2026-04-14`

Что особенно важно:

- provides Unicode Bidirectional Algorithm implementation

Почему это delicate for terminals:

- bidi and text presentation are not the same thing as cell ownership and command-line semantics
- if you ever add bidi-aware export or presentation surfaces, it should be explicit
- it should not silently rewrite core emulator truth

Итог:

✅ Good future seam for adjunct presentation/export/search tooling.  
⚠️ Not something I would apply blindly to the core terminal model.

## Recommended architecture rules

### 1. Runtime owns width and wrap truth

Host UIs should not recompute:

- cell width
- ambiguous width policy
- soft wrap boundaries
- occupancy over wide or combining sequences

### 2. Grapheme-aware movement and selection belong in the runtime

That includes:

- cursor movement
- selection anchors
- copy range projection
- hit mapping from search/link ranges back to visible cells

### 3. Ambiguous-width policy must be explicit

This is a product decision:

- CJK-aware wide treatment
- non-CJK narrow treatment
- maybe per-session or per-profile policy

But it should never be accidental.

### 4. Normalization belongs to derived matching and indexing seams

Good places:

- search
- semantic timeline extraction
- transcript indexing
- export

Bad place:

- mutating live transcript or live screen truth globally

### 5. Generic line breaking is not terminal soft wrap

`unicode-linebreak` and `textwrap` are useful, but terminal wrap is still driven by:

- screen columns
- width policy
- current cursor/cell state
- emulator mode semantics

### 6. Bidi or advanced presentation should be explicit adjunct functionality

If ever added, it should live in:

- export
- transcript viewers
- optional presentation surfaces

not as a silent rewrite of live terminal truth.

## Bottom line

Если свести весь deep dive к одной фразе:

🔥 **a universal Rust terminal package should treat Unicode width, grapheme and wrap semantics as part of runtime truth, not as a renderer convenience**

Именно это потом делает possible:

- consistent multi-host rendering
- correct restore
- sane copy/search/link behavior
- and a host-neutral projection API that other apps can actually trust.
