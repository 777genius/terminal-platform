# Deep Dive - Rust Restore, Rehydrate, And Alt-Screen Ordering

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для embeddable terminal package мало уметь:

- хранить snapshot
- держать replay queue
- рендерить screen state

Самые неприятные баги начинаются в момент:

- reattach
- workspace restore
- pane remount
- reconnect after overflow
- resize during or after restore

И особенно это критично для:

- alternate screen
- scrollback
- cursor/title/input modes
- pending PTY writes

Здесь уже важны не только crates, а **порядок операций и ownership model**.

## Primary Sources

### Rust terminal cores and screen layers

- [`alacritty_terminal/src/term/mod.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/mod.rs)
- [`alacritty_terminal/src/grid/storage.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/grid/storage.rs)
- [`vt100-rust/src/screen.rs`](https://github.com/doy/vt100-rust/blob/main/src/screen.rs)
- [`vt100-rust/src/parser.rs`](https://github.com/doy/vt100-rust/blob/main/src/parser.rs)
- [`asciinema/avt/src/terminal.rs`](https://github.com/asciinema/avt/blob/main/src/terminal.rs)
- [`shadow-terminal` README](https://github.com/tattoy-org/shadow-terminal/blob/main/README.md)

### Related recovery/runtime notes already researched

- [deep-dive-hydration-and-prompt-lifecycle.md](./deep-dive-hydration-and-prompt-lifecycle.md)
- [deep-dive-foundation-resource-lifecycle.md](./deep-dive-foundation-resource-lifecycle.md)
- [deep-dive-stream-recovery-persistence.md](./deep-dive-stream-recovery-persistence.md)
- [deep-dive-rust-snapshots-replay-and-durable-formats.md](./deep-dive-rust-snapshots-replay-and-durable-formats.md)

## Freshness signals

- `alacritty_terminal 0.26.0`
- `vt100 0.16.2`
- `avt 0.17.0`
- `shadow-terminal 0.2.3`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**restore is not serialization. restore is an ordered runtime protocol.**

Для world-class terminal package самый здоровый shape сейчас выглядит так:

- committed emulator snapshot
- bounded replay tail
- explicit alternate-screen policy
- resize-aware rehydrate ordering
- separate external projection for tools

То есть не:

- `deserialize_state(); replay_everything(); done`

а:

- validate execution context
- establish dimensions
- restore committed state
- apply terminal/input mode deltas
- only then replay safe tail
- reveal/attach after size and state converge

## Top 3 Restore Directions

### 1. Committed snapshot + bounded replay tail + explicit ordering

`🎯 10   🛡️ 9   🧠 8`  
Примерно `6000-12000` строк.

Это мой текущий **лучший default**.

Идея:

- emulator-owned committed state is restored first
- replay tail is small and bounded
- alternate-screen replay is gated by explicit policy
- resize happens in a controlled phase, not opportunistically

Почему это strongest path:

- best fit for durable sessions
- easiest to reason about after remount/reattach
- prevents "last bytes destroy last visible frame" bugs

### 2. ANSI/state-formatted rehydrate from `vt100`-style projections

`🎯 7   🛡️ 8   🧠 6`  
Примерно `4000-9000` строк.

Useful when:

- you need a host-neutral recovery projection
- you want a format easy to feed back into a parser
- you want simpler export/debug tooling

Why it is good:

- `vt100::Screen::state_formatted()` and `state_diff(...)` are strong building blocks

Why it is not enough alone:

- still a projection
- weaker as sole durable truth for rich runtime semantics

### 3. Structured cell-surface restore for tooling and verification

`🎯 6   🛡️ 7   🧠 5`  
Примерно `3000-7000` строк.

Useful for:

- tests
- inspectors
- web tooling
- external language hosts

Why it matters:

- `shadow-terminal` proves that a rich surface schema is practical

Why it is not core truth:

- cell schemas are better as derived export/read-models
- they do not naturally capture full runtime restore semantics

## 1. Alternate screen is not a detail

Both `alacritty_terminal` and `vt100` confirm this strongly.

## `alacritty_terminal`

`Term::swap_alt()` in `term/mod.rs` is very revealing:

- alternate screen has its own inactive grid
- cursor handling changes on swap
- saved cursor state is reset differently
- selection is cleared
- terminal is fully damaged after swap

This means:

🔥 alternate screen is a distinct modeful state transition, not just "other lines in the same buffer"

## `vt100`

`Screen` explicitly has:

- `grid`
- `alternate_grid`

and resizes both in `set_size(...)`.

That is another strong signal:

- primary and alternate screen need distinct ownership in restore logic

## 2. Resize is part of restore semantics

This is one of the most important Rust-level findings.

`alacritty_terminal/src/grid/storage.rs` shows:

- ring-buffer-like storage behavior
- `grow_visible_lines`
- `shrink_visible_lines`
- `truncate`
- rotation-based storage management

This means resize is not a harmless visual afterthought.
It can change:

- visible line packing
- scrollback boundaries
- ring-buffer layout

So:

⚠️ restore-after-resize and resize-after-restore are not equivalent

For a serious package, resize must be a deliberate stage in the rehydrate protocol.

## 3. `vt100` gives strong recovery projections

`vt100::Screen` already exposes:

- `state_formatted()`
- `state_diff(prev)`
- `contents_formatted()`
- `contents_diff(prev)`

This is extremely useful.

### Why it matters

It gives you a recovery surface that:

- is parser-friendly
- is renderer-neutral
- can be persisted as a projection
- can be diffed against prior committed state

This makes `vt100` a strong donor for:

- snapshot export
- deterministic rehydrate tests
- alt-screen recovery experiments
- safe tooling/debug layers

But:

⚠️ do not confuse projection with truth

## 4. External tooling surface should stay derived

`shadow-terminal` is useful here because it exposes a rich output schema with:

- `cells`
- `cursor`
- `width`
- `height`
- `title`
- `mode`

This is great for:

- external inspectors
- CI tooling
- frontend diagnostics
- cross-language hosts

But architecturally:

- this should remain a read-model/export surface
- not the only committed runtime state

## 5. Recommended restore ordering

The current best shape after this pass looks like:

1. Validate session identity and execution context
2. Re-establish or estimate terminal dimensions
3. Restore committed emulator snapshot
4. Re-apply terminal/input mode state
5. Gate alternate-screen replay by explicit policy
6. Flush or classify pending app-originated writes
7. Replay only the safe bounded tail
8. Reveal and attach host UI after state and size converge

### Why this ordering

- dimensions influence buffer interpretation
- alt-screen needs different policy than primary-screen replay
- pending writes can race with restore and fake a newer truth than the committed state
- host reveal before convergence produces visible corruption and misleading UX

## 6. What should be persisted separately

The research now points to at least these separate persisted concepts:

- committed primary screen snapshot
- committed alternate screen snapshot or alt-screen projection
- cursor and input mode state
- title/cwd/session metadata
- bounded replay tail metadata
- durable scrollback mirror

If these are collapsed into one untyped blob, future migrations and recovery logic get ugly fast.

## 7. What I would choose now

If choosing right now for the future package:

1. primary restore truth - committed emulator-owned snapshot
2. recovery projection - `vt100`-style `state_formatted` / `state_diff`
3. external tooling surface - `shadow-terminal`-style structured schema
4. rehydrate ordering - explicit runtime protocol, not incidental host lifecycle
5. resize - treated as restore-phase input, not cosmetic post-processing

## Final architectural rule

🔥 **The last visible frame is a product truth, not an implementation accident.**

For a world-class terminal package:

- alternate screen must be explicit
- resize must be explicit
- replay must be bounded and policy-driven
- reveal must happen after convergence

That is what turns restore from "best effort" into a reliable platform capability.

