# Deep Dive - Rust Inline Graphics, Media Protocols, and Host Render Boundaries

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Если terminal package должен быть modern, sooner or later всплывают вопросы про:

- `SIXEL`
- `kitty graphics`
- `iTerm2 OSC 1337`-style inline images
- image placeholders and overlays
- host-side fallbacks for terminals without native media protocols

Именно здесь очень легко сделать архитектурную ошибку:

- засунуть media protocols в core terminal truth
- дать renderer crate диктовать public runtime contract
- смешать image decoding, protocol encoding и host UI fallback в один слой

🔥 Для universal embeddable package это особенно опасно, потому что media support почти всегда:

- capability-dependent
- host-dependent
- protocol-family-dependent
- much less universal than plain text terminal behavior

Поэтому этот слой надо проектировать отдельно.

## Primary Sources

### Rust image/graphics crates

- [`icy_sixel` crate](https://crates.io/crates/icy_sixel)
- [`mkrueger/icy_sixel` repo](https://github.com/mkrueger/icy_sixel)
- [`sixel-rs` crate](https://crates.io/crates/sixel-rs)
- [`image` crate](https://crates.io/crates/image)
- [`image-rs/image` repo](https://github.com/image-rs/image)
- [`base64` crate](https://crates.io/crates/base64)
- [`viuer` crate](https://crates.io/crates/viuer)
- [`atanunq/viuer` repo](https://github.com/atanunq/viuer)
- [`ratatui-image` crate](https://crates.io/crates/ratatui-image)
- [`ratatui/ratatui-image` repo](https://github.com/ratatui/ratatui-image)

### Related terminal/runtime context

- [`libghostty-vt` crate](https://crates.io/crates/libghostty-vt)
- [`Uzaaft/libghostty-rs` repo](https://github.com/Uzaaft/libghostty-rs)
- `restty` code references around kitty graphics / overlays

## Freshness signals

- `icy_sixel 0.5.0` - repo `mkrueger/icy_sixel`, `26` stars, pushed `2026-01-24`
- `sixel-rs 0.5.0`
- `image 0.25.10` - repo `image-rs/image`, `5723` stars, pushed `2026-04-19`
- `base64 0.22.1`
- `viuer 0.11.0` - repo `atanunq/viuer`, `346` stars, pushed `2025-12-09`
- `ratatui-image 10.0.6` - repo `ratatui/ratatui-image`, `316` stars, pushed `2026-04-10`
- `libghostty-vt 0.1.1` - repo `Uzaaft/libghostty-rs`, `254` stars, pushed `2026-04-09`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**inline graphics should be treated as optional render/media capability above terminal truth, not as part of minimal core semantics**

Healthiest shape сейчас выглядит так:

1. core runtime owns text/cell/projection truth
2. media protocols are optional capability adapters
3. image decoding/transcoding is separate from terminal semantics
4. host renderers or standalone leaves may choose protocol-specific output
5. unsupported hosts should degrade gracefully to text/fallback surfaces

То есть не:

- "terminal core обязательно должен понимать kitty images как first-class truth"

а:

- `MediaCapability`
- `InlineGraphicIntent`
- `MediaRenderAdapter`
- `FallbackRenderStrategy`

## Top 3 directions for media/graphics architecture

### 1. `Typed media capability layer + protocol-specific optional adapters + host fallback strategies`

`🎯 10   🛡️ 8   🧠 8`
Примерно `7000-15000` строк.

Это strongest default.

Идея:

- runtime exposes a typed media/graphics capability surface
- protocol-specific leaves handle `SIXEL`, `kitty`, `iTerm2`-style output where supported
- host may fallback to text placeholders, out-of-band preview, or no-op
- image decoding/preprocessing stays outside terminal core truth

Почему это лучший путь:

- host-neutral
- capability negotiation stays honest
- modern media features can evolve without contaminating text-terminal core
- standalone Rust app and JS/Electron host can choose different rendering stories

### 2. `Leaf-host widgets and viewers around a text-first runtime`

`🎯 8   🛡️ 8   🧠 6`
Примерно `5000-10000` строк.

Это practical path for:

- standalone Rust hosts
- demos
- reference renderers

Почему это интересно:

- `ratatui-image` already supports multiple image paths at the widget layer
- `viuer` shows terminal-image rendering as a host concern

Почему не strongest universal default:

- widget-oriented crates are not the same thing as host-neutral runtime contracts
- easy to accidentally let one host toolkit define architecture

### 3. `Media protocols inside minimal core runtime`

`🎯 3   🛡️ 4   🧠 7`
Примерно `5000-12000` строк.

Это плохой путь.

Симптомы:

- media support starts shaping emulator/runtime truth
- protocol-specific quirks leak into generic APIs
- unsupported hosts become second-class immediately

## 1. `SIXEL` currently has the clearest Rust bricks

This pass makes one ecosystem fact pretty clear:

🔥 In Rust today, `SIXEL` has meaningfully stronger crate-level building blocks than other inline graphics protocols.

### `icy_sixel`

Why it matters:

- pure Rust encoder/decoder story
- direct terminal-graphics orientation
- good fit for optional protocol adapter layer

### `sixel-rs`

Why it matters:

- wrapper around libsixel family
- useful if pure-Rust path is not enough for some hosts

Practical conclusion:

- if v1 ever supports one protocol-family first, `SIXEL` currently looks like the least speculative Rust path

## 2. `kitty graphics` support exists mostly as host/widget reality, not as mature core Rust foundation

This is one of the most important findings of the pass.

We found signals around kitty graphics usage and references, but not a clearly dominant Rust foundation crate with serious ecosystem gravity comparable to `image` or even `icy_sixel`.

That means:

- kitty graphics is real and relevant
- but for architecture today it looks more like an optional host/render leaf concern than a crate-led core foundation

🔥 Practical rule:

**do not let desire for kitty graphics support force a protocol-specific core design too early**

## 3. `ratatui-image` is a strong reference leaf, not a runtime foundation

`ratatui-image 10.0.6` is very useful because it already spans:

- `sixel`
- `kitty`
- `iterm2`
- unicode-halfblock fallback

That makes it a great signal for architecture:

- multiple media strategies belong naturally at the host/widget layer
- fallback strategy matters as much as protocol support

But it should still be interpreted as:

- reference leaf
- standalone Rust host tooling
- useful donor for capability/fallback modeling

not:

- core runtime truth model

## 4. `viuer` is useful as proof that image rendering is often a host concern

`viuer 0.11.0` is helpful mainly as a reminder:

- image rendering in terminal contexts is often a host-side presentational problem
- crates that solve "show image in terminal" are not automatically runtime architecture answers

This is valuable because it reinforces a separation:

- media decode/prepare/render belongs high in the stack
- terminal state truth belongs lower

## 5. `image` and `base64` are infrastructural helpers, not terminal semantics

### `image`

`image 0.25.10` clearly remains the default image decoding/processing brick.

Great for:

- decode source image
- resize/transcode
- prepare raster data for protocol-specific encoding

### `base64`

Useful because some inline media protocols and transport paths want encoded payloads.

But neither crate should define terminal semantics.

🔥 Strong rule:

**image decode/encode is media infrastructure, not terminal truth**

## 6. `libghostty-vt` and modern runtime designs reinforce the boundary

What matters here is not that `libghostty-vt` already solves inline graphics end-to-end.

What matters is the architecture signal:

- terminal runtime can stay focused on terminal state and protocol events
- richer graphics/overlay/media layers can sit above that

Combined with earlier `restty` observations around kitty graphics/overlays, this points to a healthy design:

- terminal core stays text/cell/projection oriented
- media/overlay adapters remain optional and surface-coordinator-scoped

## 7. Media support should be capability-negotiated and degradable

For this package, a healthy media model should probably distinguish:

- supported protocol family
- maximum payload/size policy
- whether inline render is allowed at all
- what fallback strategy host wants

Examples of fallback:

- textual placeholder
- clickable link/open action
- external preview pane
- no-op with visible unsupported marker

This matters because:

- JS/Electron host may prefer adjacent webview/image pane
- standalone Rust app may prefer protocol-native inline render
- some hosts may forbid media output entirely

## Practical verdict

If I were designing this layer right now:

### V1

- keep terminal core media-agnostic
- define typed media capability and inline-graphic intents
- treat `SIXEL` as the most plausible first protocol-family if protocol-native support becomes necessary
- use `image` as decode/preprocess brick
- treat `ratatui-image` and `viuer` as reference leafs, not foundations
- no protocol-specific media semantics in minimal public core

### V2

- optional protocol adapters per media family
- richer fallback policies
- explicit host capability negotiation for media classes
- maybe dedicated adjacent image/preview surface in richer hosts

## Чего я бы избегал

- ❌ Baking one graphics protocol into core terminal truth
- ❌ Letting one host widget crate define public runtime semantics
- ❌ Treating image decode/encode crates as terminal architecture
- ❌ Assuming all hosts must support inline graphics
- ❌ Making unsupported-media behavior implicit

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- inline media should be an optional capability layer
- text terminal truth must stay separate from media rendering
- protocol adapters should remain optional leaves
- fallback strategy should be explicit
- `SIXEL` currently looks like the strongest Rust-first protocol path if we ever pick one

## Sources

- [icy_sixel crate](https://crates.io/crates/icy_sixel)
- [mkrueger/icy_sixel](https://github.com/mkrueger/icy_sixel)
- [sixel-rs crate](https://crates.io/crates/sixel-rs)
- [image crate](https://crates.io/crates/image)
- [image-rs/image](https://github.com/image-rs/image)
- [base64 crate](https://crates.io/crates/base64)
- [viuer crate](https://crates.io/crates/viuer)
- [atanunq/viuer](https://github.com/atanunq/viuer)
- [ratatui-image crate](https://crates.io/crates/ratatui-image)
- [ratatui/ratatui-image](https://github.com/ratatui/ratatui-image)
- [libghostty-vt crate](https://crates.io/crates/libghostty-vt)
- [Uzaaft/libghostty-rs](https://github.com/Uzaaft/libghostty-rs)
