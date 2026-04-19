# Deep Dive - Rust Snapshots, Replay Buffers, And Durable Formats

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для terminal package мирового уровня мало решить:

- какой PTY adapter взять
- какой emulator core взять
- какой protocol boundary сделать

Нужно ещё жёстко решить:

- чем живёт hot replay buffer
- в каком виде хранится durable snapshot
- что именно отдаётся наружу как tooling/export projection

Если эти три вещи смешать, очень быстро получаются плохие решения:

- durable store зависит от hot-path queue semantics
- внешние host apps завязываются на внутренний snapshot format
- recovery логика начинает replay-ить не то, что реально видел пользователь

## Primary Sources

### Screen and snapshot helpers

- [`vt100-rust` README](https://github.com/doy/vt100-rust/blob/main/README.md)
- [`vt100-rust/src/screen.rs`](https://github.com/doy/vt100-rust/blob/main/src/screen.rs)
- [`asciinema/avt` README](https://github.com/asciinema/avt/blob/main/README.md)
- [`shadow-terminal` README](https://github.com/tattoy-org/shadow-terminal/blob/main/README.md)
- [`shadow-terminal output-schema.json`](https://github.com/tattoy-org/shadow-terminal/blob/main/output-schema.json)

### Durable payload and buffer crates

- [`rmp-serde` README](https://github.com/3Hren/msgpack-rust/blob/master/rmp-serde/README.md)
- [`postcard` README](https://github.com/jamesmunns/postcard/blob/main/README.md)
- [`ringbuf` README](https://github.com/agerasev/ringbuf/blob/master/README.md)
- [`BLAKE3` README](https://github.com/BLAKE3-team/BLAKE3/blob/master/README.md)
- [`zstd-rs` README](https://github.com/gyscos/zstd-rs/blob/main/Readme.md)

### Existing session/recovery patterns

- [deep-dive-stream-recovery-persistence.md](./deep-dive-stream-recovery-persistence.md)
- [deep-dive-streaming-protocol-and-timeline-persistence.md](./deep-dive-streaming-protocol-and-timeline-persistence.md)

## Freshness signals

### Snapshot / screen crates

- `vt100 0.16.2`
- `avt 0.17.0`
- `shadow-terminal 0.2.3`

### Serialization / compression / buffering crates

- `rmp-serde 1.3.1`
- `postcard 1.1.3`
- `bincode 3.0.0`
- `zstd 0.13.3`
- `ringbuf 0.4.8`
- `blake3 1.8.4`
- `crc32fast 1.5.0`
- `memchr 2.8.0`

## Короткий вывод

🔥 Самый сильный вывод этого прохода:

**hot replay, durable snapshot и export projection должны быть разными слоями**

Самый здоровый shape сейчас выглядит так:

- hot replay buffer -> small, bounded, cheap, disposable
- durable snapshot blob -> typed, versioned, optionally compressed
- external/tooling projection -> human/tool friendly and explicitly derived

То есть не:

- one format for everything

а:

- `replay queue`
- `snapshot envelope`
- `export/read-model`

## Top 3 Snapshot / replay directions

### 1. Typed snapshot envelope + compressed durable blobs + separate replay ring

`🎯 9   🛡️ 9   🧠 7`  
Примерно `5000-10000` строк.

Это мой текущий **лучший default**.

Recommended stack:

- hot replay queue: `ringbuf`
- durable snapshot payload: `rmp-serde`
- snapshot compression: `zstd`
- snapshot fingerprint/integrity: `blake3` + optional `crc32fast`

Почему это strongest path:

- `ringbuf` is explicitly a lock-free SPSC FIFO with direct access to inner data
- `rmp-serde` gives typed payloads with a cross-language-friendly MessagePack family
- `zstd` gives stream encode/decode wrappers and is a very practical default for larger snapshot blobs
- `blake3` gives incremental hashing and verified-streaming-friendly semantics

Главный плюс:

🔥 этот путь хорошо делит hot path и durable path, не делая их зависимыми друг от друга

### 2. `postcard`-first compact binary records

`🎯 7   🛡️ 8   🧠 6`  
Примерно `4000-8000` строк.

Когда это интересно:

- нужен very compact binary format
- важен documented stable wire format
- хочется tighter control for local runtime records

Почему интересно:

- `postcard` прямо документирует stable wire format as of `1.0.0`
- designed for efficient serde-compatible binary payloads
- very useful for internal records and compact local payloads

Почему не мой default:

- format and tooling story weaker for desktop/server debugging than MessagePack-like payloads
- ergonomically feels more like a systems/internal format than a great default for a terminal package with long-lived external ecosystem ambitions

### 3. ANSI/state-formatted projection and cell-schema export

`🎯 8   🛡️ 7   🧠 5`  
Примерно `3000-7000` строк.

Это strongest compatibility/export path, но не primary truth model.

Useful ingredients:

- `vt100::Screen::state_formatted()`
- `vt100::Screen::state_diff(...)`
- `shadow-terminal`-style rich cell/cursor/title schema
- `avt`-style narrow parser + virtual buffer seam

Почему это полезно:

- easy to feed back into a raw parser
- good for tooling and debugging
- good for snapshot export and deterministic reproduction surfaces

Почему не primary truth:

- ANSI/state-formatted output is a **projection**
- it should not be the only durable domain representation

## 1. `vt100` - state-formatted and state-diff projections are a big deal

Этот проход сделал `vt100` ещё полезнее, чем раньше казалось.

Из `screen.rs` видно, что он уже умеет:

- `state_formatted()`
- `state_diff(prev)`
- `contents_formatted()`
- `contents_diff(prev)`

Это очень сильный pattern.

### Что это значит practically

Можно иметь минимум 2 полезных производных surface-а:

- full visible-state reproduction blob
- diff from previous visible-state snapshot

Они не обязаны быть вашим durable truth, но они очень хороши для:

- recovery projection
- export
- debugging
- replay
- test fixtures

🔥 Это особенно полезно потому, что такие blobs можно снова скормить raw terminal parser-у и получить тот же visual state.

## 2. `shadow-terminal` - external read-model should be rich and explicit

`shadow-terminal` README и `output-schema.json` дают очень сильную идею.

Они уже отдают output как rich structured surface:

- `width`
- `height`
- `cells`
- `cursor`
- `title`
- `mode`

`cells` при этом содержат:

- `text`
- `foreground`
- `background`

Это отличный donor pattern.

### Практический вывод

Если package потом будет использоваться:

- из Electron
- из other languages
- в CI/testing tools
- в external automation

то useful external read-model должен быть:

- explicit
- structured
- host-neutral

Но:

⚠️ это должен быть **derived read model**, not the core internal truth

## 3. `avt` confirms the right seam again

`avt` особенно полезен тем, что very clearly scopes itself to:

- parser
- virtual screen buffers
- querying virtual buffer and cursor

and explicitly leaves out:

- input handling
- rendering

Это хороший reminder:

✅ terminal snapshot/replay helpers do not need to own the entire runtime

## 4. `ringbuf` - very good hot replay building block

`ringbuf` README makes its strengths very explicit:

- lock-free SPSC FIFO
- direct access to inner data
- batch operations
- `Read` / `Write` implementation
- overwrite support

This is a strong fit for:

- PTY reader -> replay buffer
- replay buffer -> flush worker
- bounded hot-path queue between one producer and one consumer

### Why I like it here

For terminal runtime hot replay queues, **boring bounded SPSC** is often healthier than inventing a general event bus.

Important caveat:

- good for hot-path replay staging
- not your durable scrollback store
- not your general orchestration backbone

## 5. `rmp-serde` - good default for typed internal snapshot blobs

`rmp-serde` stays interesting because it gives:

- typed serde payloads
- MessagePack family format
- decent tooling/interoperability story

And the README contains one very practical warning:

⚠️ plain serde handling of byte arrays can waste space  
Use `serde_bytes` or explicit serializer configuration for efficient blob fields.

That matters a lot for terminal snapshots where blobs may contain:

- replay chunks
- serialized cells
- compressed payload fragments

### Why I currently prefer it over making `bincode` the default

- better host/tool friendliness
- healthier cross-language story
- easier to imagine future inspection tools

But:

⚠️ this should still be wrapped in an explicit snapshot envelope with `version`, `kind`, `compression`, `hash`, not used naked

## 6. `postcard` - strong if you want explicit stable wire format

`postcard` is more interesting than it first appears.

Important signals from README:

- stable wire format documented since `1.0.0`
- strong resource-efficiency goals
- serde-compatible
- varint encoding

This makes it a strong candidate for:

- compact local daemon records
- embedded-ish internal records
- highly disciplined binary envelopes

Why it is not my default here:

- less ergonomic debugging/tooling story for a desktop-first terminal package
- feels better for tightly controlled internal records than for the main durable snapshot ecosystem of a world-class package

## 7. `zstd` - compression belongs off the hot path

`zstd-rs` gives a very pragmatic story:

- `Read` / `Write` wrappers
- `copy_encode` / `copy_decode`
- easy stream integration

This makes it a strong fit for:

- periodic durable snapshot compression
- archival scrollback blobs
- exported replay bundles

But the architectural rule matters more than the crate:

🔥 compression should sit **after** snapshot formation, not inside the hot replay loop

## 8. `blake3` and `crc32fast` - different jobs

This pass made the distinction clearer.

## `blake3`

Useful for:

- stable snapshot fingerprinting
- dedupe
- identity of exported replay bundles
- stronger integrity semantics

Its README also highlights:

- incremental updates
- verified streaming friendliness

That makes it genuinely interesting for larger snapshot/replay systems.

## `crc32fast`

Useful for:

- cheap corruption detection
- lightweight guards on local framed blobs

But:

- do not confuse checksum with identity

Practical rule:

- `crc32fast` for quick corruption detection
- `blake3` for dedupe / identity / stronger verification

## 9. `bincode` - tempting, but not my default

`bincode 3.0.0` is clearly alive again and technically usable.
But for this package I still would not make it the default durable snapshot story.

Why:

- weaker tooling and ecosystem story for inspection
- less attractive for cross-language future
- easier to accidentally couple durable storage too tightly to Rust-native assumptions

It can still be acceptable for:

- very internal ephemeral blobs
- performance experiments

But not as my first recommendation for a package with "people will embed this in many hosts" ambitions.

## Recommended layered model

```text
PTY output
  -> bounded hot replay ring
  -> flush / batch worker

Replay ring
  -> short-term incremental recovery
  -> drop/overflow semantics

Snapshot builder
  -> emulator-owned visible state
  -> metadata
  -> versioned envelope
  -> fingerprint

Durable store
  -> optional compression
  -> revision / write policy
  -> mirror / archive

Export / tooling projection
  -> ANSI/state-formatted output
  -> cell/cursor/title schema
  -> debugging / external tooling
```

## What I would choose now

Если выбирать прямо сейчас:

1. hot replay queue - `ringbuf`
2. durable snapshot envelope - typed serde struct
3. payload encoding - `rmp-serde`
4. compression - `zstd` only after snapshot creation
5. snapshot fingerprint - `blake3`
6. lightweight corruption check where useful - `crc32fast`
7. external/tooling projection - `vt100` state formatting and `shadow-terminal`-style structured surface

## Final architectural rule

🔥 **Do not let your durable format become your live runtime model.**

Для terminal package мирового уровня:

- live replay needs bounded cheap structures
- durable snapshots need typed versioned envelopes
- external tooling needs explicit projections

Эти три слоя должны быть связаны, но не слеплены.

