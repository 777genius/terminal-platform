# Deep Dive - Rust Render Models, Diffs, And Host Projections

**Проверено**: 2026-04-19

## Зачем этот слой важен

Если UI должен быть:

- на JS сегодня
- на чём угодно завтра
- не привязан к конкретному terminal crate

тогда нужен не просто emulator core, а **host-neutral projection contract**.

Именно здесь многие terminal проекты делают скрытую архитектурную ошибку:

- внутренний grid emulator-а начинают считать public UI API
- host UI сам начинает решать width/graphemes/wrap semantics
- live render, snapshot export и test surfaces смешиваются в один слой

Для universal package это плохой путь.

Нужно чётко разделить:

1. internal emulator state
2. live render projection
3. dirty/diff projection
4. snapshot/export projection

## Primary Sources

### Core terminal/view-model crates

- [`vt100` README](https://github.com/doy/vt100-rust/blob/master/README.md)
- [`shadow-terminal` README](https://github.com/tattoy-org/shadow-terminal/blob/main/README.md)
- [`shadow-terminal` output schema](https://github.com/tattoy-org/shadow-terminal/blob/main/output-schema.json)
- [`libghostty-vt` terminal.rs](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/terminal.rs)
- [`libghostty-vt` screen.rs](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/screen.rs)
- [`libghostty-vt` fmt.rs](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/fmt.rs)
- [`libghostty-vt` style.rs](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/style.rs)
- [`alacritty_terminal` term/mod.rs](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/mod.rs)

### Text layout helpers

- [`unicode-width`](https://github.com/unicode-rs/unicode-width)
- [`unicode-segmentation`](https://github.com/unicode-rs/unicode-segmentation)

### Small internal optimization helpers

- [`compact_str`](https://github.com/ParkMyCar/compact_str)
- [`smallvec`](https://github.com/servo/rust-smallvec)

## Freshness signals

- `vt100 0.16.2` - repo `doy/vt100-rust`, `112` stars, pushed `2025-07-12`
- `avt 0.17.0` - repo `asciinema/avt`
- `shadow-terminal 0.2.3` - repo `tattoy-org/shadow-terminal`
- `libghostty-vt 0.1.1` - repo `Uzaaft/libghostty-rs`
- `alacritty_terminal 0.26.0` - repo `alacritty/alacritty`, `63.5k` stars, pushed `2026-04-14`
- `unicode-segmentation 1.13.2`
- `unicode-width 0.2.2`
- `compact_str 0.9.0`
- `smallvec 2.0.0-alpha.12`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**emulator core не должен быть public UI contract**

Самый здоровый shape сейчас такой:

1. emulator owns terminal truth
2. runtime exposes a stable host projection model
3. live UI consumes diffs or dirty regions
4. tooling/export consumes full snapshots or formatted projections

То есть не:

- "UI directly reads emulator internals"

а:

- `ScreenSnapshot`
- `ScreenDelta`
- `CursorState`
- `ModeState`
- `StyleProjection`
- `Selection/Link/Prompt metadata`

## Top 3 directions for Rust host projection layer

### 1. Explicit projection layer above emulator core

`🎯 10   🛡️ 9   🧠 8`
Примерно `7000-14000` строк.

Это мой текущий **лучший default**.

Идея:

- `alacritty_terminal` or `libghostty-vt` stays internal truth
- runtime creates its own stable `ScreenSnapshot` and `ScreenDelta`
- UI never depends directly on emulator-specific grid structs

Почему это strongest path:

- decouples UI from emulator choice
- lets Electron/web/native all consume the same contract
- keeps restore/testing/export flows coherent

### 2. `libghostty-vt` render/state API as the strongest integrated donor

`🎯 8   🛡️ 7   🧠 8`
Примерно `5000-11000` строк.

Почему это интересно:

- `screen.rs` already exposes `Screen`, `GridRef`, `Row`, `Cell`, `Style`
- rows know `is_dirty`, `is_wrapped`, `semantic_prompt`, hyperlinks, grapheme clusters
- `fmt.rs` already formats terminal state to `Plain`, `Vt`, and `Html`

Почему не public default:

- the crate itself warns `GridRef` is not for the core render loop
- references are invalidated after terminal updates
- still too tied to one emulator surface

### 3. `shadow-terminal` style structured full-surface schema

`🎯 8   🛡️ 8   🧠 6`
Примерно `4000-9000` строк.

Почему это очень полезный donor:

- already has a host-neutral JSON schema
- models `cells`, `cursor`, `title`, `mode`, `width`, `height`
- clearly separates full structured output from the internal terminal implementation

Почему не enough as the whole answer:

- stronger for snapshots, testing and tooling than for high-frequency live UI
- schema by itself does not solve efficient dirty region streaming

## 1. `alacritty_terminal` proves that damage tracking belongs in runtime

The most useful part of `alacritty_terminal` for this layer is not only the emulator itself.

It is the explicit damage model:

- `LineDamageBounds`
- `TermDamage`
- viewport conversion helpers

This is a very strong signal:

🔥 **dirty tracking should live with terminal state, not in the host UI**

Why:

- only the runtime truly knows wrap semantics, cursor semantics, and mode transitions
- host UI should not infer redraw regions by diffing random cell arrays if runtime already knows better

## 2. `vt100` is excellent proof for snapshot/diff projections

`vt100` is still one of the clearest sources for the projection idea.

From the README alone:

- `screen().contents_formatted()`
- `screen().contents_diff(&old_screen)`

This is very valuable because it shows:

- a terminal parser/emulator can expose explicit derived output surfaces
- diff and formatted output can be first-class APIs
- projection is not the same thing as core mutable state

🔥 Practical takeaway:

**full snapshot and incremental diff should both be first-class runtime outputs**

## 3. `shadow-terminal` is the clearest proof of a host-neutral screen schema

`shadow-terminal` is especially useful because it already externalizes a schema.

Its `output-schema.json` models:

- `cells`
- `cursor`
- `height`
- `width`
- `title`
- `mode`

And each cell has:

- `text`
- `foreground`
- `background`

Cursor has:

- position
- shape
- visibility

Screen mode distinguishes:

- `Primary`
- `Alternate`

This is extremely useful for us because it proves a very important point:

🔥 **a stable external terminal surface can be described without leaking the emulator's internal storage model**

## 4. `libghostty-vt` is strong because it exposes rich query seams but warns about lifetime boundaries

`libghostty-vt` has some of the best low-level host-facing query APIs I found.

Useful parts:

- `Screen::{Primary, Alternate}`
- `GridRef`
- `Row`
- `Cell`
- `Style`
- `Formatter`

But the important lesson is in the warning itself:

- `GridRef` is only valid until the next terminal update
- it is not meant to be the core render loop API

That warning is gold.

It strongly suggests our package should:

- use emulator-native query APIs internally
- copy/project into stable host-facing structures externally

not hand out ephemeral internal references as its main contract.

## 5. `fmt.rs` confirms that formatted projections are their own surface

`libghostty-vt` `Formatter` can emit:

- `Plain`
- `Vt`
- `Html`

with:

- trim
- unwrap
- selection-based formatting

This is another strong architectural hint:

- formatted output is a separate use case from live rendering
- export/copy/selection APIs deserve their own formatter seam

So our package should probably have both:

- render projection API
- formatting/export API

## 6. `screen.rs` suggests what a rich host model should include

`libghostty-vt` `screen.rs` is one of the best hints for the fields a host projection may need:

- primary vs alternate screen
- row wrap and wrap continuation
- dirty flag
- grapheme-cluster presence
- hyperlink presence
- semantic prompt state
- style lookup

This suggests a future host projection model like:

- `ScreenSnapshot { screen_mode, width, height, rows, cursor, title }`
- `RowProjection { wrapped, wrap_continuation, dirty, semantic_prompt, cells }`
- `CellProjection { grapheme, style_id, hyperlink_id, width, flags }`
- `StyleTable { id -> style }`

That is much healthier than shoving duplicated style data into every event if we can avoid it.

## 7. Unicode width and grapheme semantics belong in runtime, not UI

This pass reinforced an important rule.

`unicode-width` and `unicode-segmentation` are useful, but not as public host responsibilities.

Why:

- grapheme composition and width directly affect cell occupancy
- soft wrap, cursor placement and selection depend on runtime-consistent semantics
- if host UIs recompute this differently, projections drift

🔥 Practical rule:

**runtime should own grapheme and width semantics; host should consume resolved projections**

## 8. `compact_str` and `smallvec` are internal micro-optimizers only

`compact_str 0.9.0` is interesting as an internal optimization for:

- titles
- short style keys
- small command labels

But it should not shape the public contract.

`smallvec` is currently on `2.0.0-alpha.12`, which makes it a poor foundational dependency for a public package contract.

Useful internal idea:

- maybe use stack-friendly buffers in hot paths

But do not let those choices define the host-facing API.

## 9. Recommended projection stack now

At this point, the healthiest shape looks like:

### Internal runtime truth

- emulator grid/state
- terminal modes
- scrollback
- selection and prompt semantics

### Live render projection

- `ScreenDelta`
- dirty rows / damaged regions
- cursor + mode deltas
- style table diffs

### Snapshot projection

- full `ScreenSnapshot`
- suitable for attach/recovery/tooling

### Formatting projection

- plain text
- VT-preserving text
- HTML
- optional selection-only output

## 10. What I would explicitly avoid

- ❌ exposing raw emulator grid references to host UI as the primary API
- ❌ making the host recompute grapheme width and wrap semantics independently
- ❌ using the same structure for live UI diffs and export snapshots
- ❌ duplicating full style payloads in every cell event if a style table/model can be projected separately
- ❌ tying public API to one emulator crate's lifetime model
- ❌ adopting `smallvec` alpha as a foundational public dependency

## Final recommendation

If building the host projection layer today, I would choose:

- internal truth: `alacritty_terminal` or `libghostty-vt`
- live diff donor: `alacritty_terminal` damage model
- snapshot/diff donor: `vt100`
- structured external schema donor: `shadow-terminal`
- rich cell/style semantics donor: `libghostty-vt screen.rs`
- Unicode helpers only inside runtime: `unicode-width`, `unicode-segmentation`

🔥 Most important practical takeaway:

**the package should publish a stable render/snapshot contract of its own, not treat any emulator's internal query API as the host UI boundary**

That is what will let the same Rust core power:

- Electron UI
- standalone terminal app
- testing harnesses
- future native/mobile/web shells
- other-language adapters

## Sources

- [vt100](https://github.com/doy/vt100-rust)
- [shadow-terminal](https://github.com/tattoy-org/shadow-terminal)
- [shadow-terminal output schema](https://github.com/tattoy-org/shadow-terminal/blob/main/output-schema.json)
- [libghostty-rs](https://github.com/Uzaaft/libghostty-rs)
- [alacritty_terminal](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/mod.rs)
- [unicode-width](https://github.com/unicode-rs/unicode-width)
- [unicode-segmentation](https://github.com/unicode-rs/unicode-segmentation)
- [compact_str](https://github.com/ParkMyCar/compact_str)
- [smallvec](https://github.com/servo/rust-smallvec)
