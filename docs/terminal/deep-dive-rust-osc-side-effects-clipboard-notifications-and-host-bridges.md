# Deep Dive - Rust OSC Side Effects, Clipboard, Notifications, and Host Bridges

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Modern terminal package почти неизбежно сталкивается не только с bytes in / bytes out, но и с terminal-emitted intents:

- hyperlinks
- clipboard requests
- title updates
- cwd updates
- notification requests
- bell/attention signals
- status-bridge style long-lived state

Именно тут архитектура легко деградирует:

- core начинает сам открывать URL
- clipboard writes становятся ambient convenience
- notifications смешиваются с persistent status
- OSC parsing и host side effects оказываются в одном модуле

🔥 Для universal embeddable package это особенно опасно, потому что разные host apps хотят разную policy:

- Electron host может сам рисовать notification center
- standalone app может делать system notification fallback
- другой host может вообще запретить clipboard writes or URL open

Поэтому этот слой должен быть отдельным bounded context.

## Primary Sources

### Host-side effect crates

- [`arboard` crate](https://crates.io/crates/arboard)
- [`1Password/arboard` repo](https://github.com/1Password/arboard)
- [`notify-rust` crate](https://crates.io/crates/notify-rust)
- [`hoodie/notify-rust` repo](https://github.com/hoodie/notify-rust)
- [`open` crate](https://crates.io/crates/open)
- [`Byron/open-rs` repo](https://github.com/Byron/open-rs)
- [`opener` crate](https://crates.io/crates/opener)
- [`Seeker14491/opener` repo](https://github.com/Seeker14491/opener)
- [`url` crate](https://crates.io/crates/url)
- [`servo/rust-url` repo](https://github.com/servo/rust-url)

### Terminal-side parsing and policy context

- [`libghostty-vt` crate](https://crates.io/crates/libghostty-vt)
- [`Uzaaft/libghostty-rs` repo](https://github.com/Uzaaft/libghostty-rs)
- [cmux notifications docs](https://github.com/manaflow-ai/cmux/blob/main/docs/notifications.md)

## Freshness signals

- `arboard 3.6.1` - repo `1Password/arboard`, `922` stars, pushed `2026-01-25`
- `notify-rust 4.16.0` - repo `hoodie/notify-rust`, `1385` stars, pushed `2026-04-19`
- `open 5.3.4` - repo `Byron/open-rs`, `391` stars, pushed `2026-04-19`
- `opener 0.8.4` - repo `Seeker14491/opener`, `74` stars, pushed `2026-01-25`
- `url 2.5.8` - repo `servo/rust-url`, `1538` stars, pushed `2026-04-16`
- `libghostty-vt 0.1.1` - repo `Uzaaft/libghostty-rs`, `254` stars, pushed `2026-04-09`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**terminal-emitted side effects should become typed intents in the runtime, not immediate host actions**

Healthiest shape сейчас выглядит так:

1. terminal parser/runtime produces typed side-effect intents
2. capability/policy layer decides allow, deny or transform
3. host adapters execute clipboard/open/notify operations if allowed
4. notifications and status bridges remain separate concepts
5. hyperlinks stay data until host chooses interaction

То есть не:

- runtime sees OSC 52 and writes clipboard immediately

а:

- `SideEffectIntent::ClipboardWrite`
- `SideEffectIntent::OpenUrl`
- `SideEffectIntent::Notify`
- `SideEffectIntent::SetStatus`

## Top 3 directions for side-effect architecture

### 1. `Typed side-effect intents + capability-gated host ports + separate notification/status model`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- runtime parses terminal-emitted control sequences into typed intents
- capability/policy layer decides whether these intents are allowed
- host ports execute them if permitted
- long-lived status bridge and transient notifications are modeled separately

Почему это лучший путь:

- host-neutral
- security policy remains explicit
- Electron and standalone hosts can implement different UX without changing runtime truth
- easy to test parser/policy separately from actual side effects

### 2. `Core parses intents, host UI handles most effects heuristically`

`🎯 7   🛡️ 7   🧠 5`
Примерно `4000-9000` строк.

Это workable compromise.

Идея:

- runtime emits rich side-effect events
- UI layer decides much of the behavior locally

Почему это иногда нормально:

- simpler integration for a single known host
- lets JS layer own more product UX

Почему это weaker for universal package:

- policy starts drifting per host
- side-effect behavior becomes less portable
- capabilities become host-specific assumptions

### 3. `Immediate side effects in core runtime`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-7000` строк.

Это плохой путь.

Симптомы:

- OSC parsing and effect execution live together
- clipboard/URL/notification behavior depends on platform-specific crates in core
- host neutrality is lost immediately

## 1. `url` should be the typed URL boundary

`url 2.5.8` remains the strongest obvious default for link/open surfaces.

Why it matters:

- hyperlinks should not stay raw strings forever
- open-url policy needs typed parsed values
- same URL should behave consistently across hosts

🔥 Strong rule:

**parse URLs early, open them late**

That means:

- runtime turns hyperlink-ish data into typed URL values where possible
- host adapter decides if opening is allowed and how

## 2. `arboard` is the strongest clipboard leaf, not a core dependency center

`arboard 3.6.1` looks like the healthiest clipboard brick today.

It is good because:

- cross-platform clipboard support
- text and image handling
- meaningful ecosystem signal

But the architectural rule matters more:

- clipboard authority must be explicit
- clipboard writes should be routed through a port
- runtime core should not assume clipboard is always available or allowed

So the healthy role is:

- default host clipboard adapter leaf
- not a mandatory center of runtime truth

## 3. `notify-rust` is a useful desktop-notification leaf, but very host-shaped

`notify-rust 4.16.0` is active and serious, but its role should stay narrow.

Where it is good:

- standalone desktop app
- Linux/BSD/mac terminal host
- system notification fallback

Where it should **not** sit:

- inside minimal reusable runtime crates
- as the only notification model

Because:

- notifications are not the same as attention/status
- many hosts will want in-app notification UX instead

So the right model is:

- runtime emits `Notify` intent
- host may render in-app notification
- standalone app may fallback to `notify-rust`

## 4. `open` vs `opener`

This pass makes the boundary clearer.

### `open`

`open 5.3.4` currently looks like the stronger default leaf.

Why:

- fresher signal
- clearer default-program opening model
- stronger activity signal today

### `opener`

`opener 0.8.4` is still interesting, especially because of its `reveal` feature and narrower UX helpers.

But as a general default, I would currently rank it below `open`.

🔥 Practical rule:

**opening files or URLs should remain a host leaf, regardless of which crate wins**

The runtime should expose intent, not embed a launcher worldview.

## 5. `libghostty-vt` reinforces the parser/policy boundary

`libghostty-vt 0.1.1` remains interesting not only as emulator path, but as a signal that modern terminal runtime can keep:

- parser state
- terminal mode truth
- terminal-emitted writes/replies

in one place before host side effects happen.

That makes it a strong donor for:

- treating OSC streams as typed terminal events
- keeping policy decisions above parser output

This is exactly the separation we want.

## 6. Notifications and status bridges are not the same thing

This was already visible in `cmux`, but this pass makes the Rust package implication more explicit.

### Notifications

- announce an event
- may be transient
- may be rendered as toast, badge or panel entry

### Status bridge

- represents ongoing state
- may need explicit `set-status` / `clear-status`
- should be queryable and persistent enough for current UX

🔥 Strong rule:

**`Notify` and `SetStatus` should be different runtime concepts**

Otherwise:

- attention UX gets muddy
- hosts cannot render good workspace-level state
- long-running task visibility degrades

## 7. Clipboard, hyperlinks, notifications and bell should all remain capability-aware

Healthy capability-aware model now looks like:

- `ClipboardPort`
- `OpenUrlPort`
- `NotificationPort`
- `AttentionPort`

And policy above them can decide:

- allow
- deny
- transform into app-local surface
- log but ignore

This is especially important for:

- foreign-language hosts
- sandboxed mode
- headless daemon routes

## Practical verdict

If I were designing this layer right now:

### V1

- typed `SideEffectIntent` model
- typed `NotificationIntent` vs `StatusIntent`
- `url` as typed URL boundary
- `arboard` as default clipboard leaf
- `open` as stronger default URL/file open leaf
- `notify-rust` only in standalone desktop leaf
- no immediate side effects in core runtime

### V2

- richer OSC support surface
- per-host capability negotiation for side-effect classes
- better in-app attention center modeling
- optional advanced clipboard/image flows

## Чего я бы избегал

- ❌ Immediate clipboard writes in parser/runtime core
- ❌ Treating notifications and status as one thing
- ❌ Opening URLs directly from core runtime code
- ❌ Letting platform crates define public runtime semantics
- ❌ Keeping hyperlinks and side effects as raw strings forever

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- OSC side effects deserve their own bounded context
- parser output should become typed side-effect intents
- host adapters should execute side effects under explicit capabilities
- status bridge and notifications should be split
- side-effect leaves should stay optional and host-specific

## Sources

- [arboard crate](https://crates.io/crates/arboard)
- [1Password/arboard](https://github.com/1Password/arboard)
- [notify-rust crate](https://crates.io/crates/notify-rust)
- [hoodie/notify-rust](https://github.com/hoodie/notify-rust)
- [open crate](https://crates.io/crates/open)
- [Byron/open-rs](https://github.com/Byron/open-rs)
- [opener crate](https://crates.io/crates/opener)
- [Seeker14491/opener](https://github.com/Seeker14491/opener)
- [url crate](https://crates.io/crates/url)
- [servo/rust-url](https://github.com/servo/rust-url)
- [libghostty-vt crate](https://crates.io/crates/libghostty-vt)
- [Uzaaft/libghostty-rs](https://github.com/Uzaaft/libghostty-rs)
- [cmux notifications docs](https://github.com/manaflow-ai/cmux/blob/main/docs/notifications.md)
