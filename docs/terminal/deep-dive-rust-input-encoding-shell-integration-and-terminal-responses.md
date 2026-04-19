# Deep Dive - Rust Input Encoding, Shell Integration, And Terminal Responses

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для terminal package мирового уровня input path нельзя сводить к:

- `keydown -> send bytes`

В реальности здесь минимум 4 разных слоя:

1. host UI events
2. terminal-aware input encoding
3. shell integration protocol and compatibility policy
4. terminal-generated replies back to the PTY peer

Если их смешать, быстро получаются плохие архитектуры:

- UI знает про xterm/kitty escape details
- runtime hardcodes keyboard sequences without reading terminal modes
- shell integration прячется в renderer hacks
- device/status replies идут в тот же канал, что и user input

Для reusable embeddable package это критичная boundary.

## Primary Sources

### Rust input and encoding layers

- [`termwiz/src/input.rs`](https://github.com/wezterm/wezterm/blob/main/termwiz/src/input.rs)
- [`termwiz/src/keymap.rs`](https://github.com/wezterm/wezterm/blob/main/termwiz/src/keymap.rs)
- [`wezterm-input-types/src/lib.rs`](https://github.com/wezterm/wezterm/blob/main/wezterm-input-types/src/lib.rs)
- [`libghostty-vt/src/key.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/key.rs)
- [`libghostty-vt/src/mouse.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/mouse.rs)
- [`libghostty-vt/src/paste.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/paste.rs)
- [`libghostty-vt/src/focus.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/focus.rs)
- [`libghostty-vt/src/osc.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/osc.rs)
- [`libghostty-vt/src/terminal.rs`](https://github.com/Uzaaft/libghostty-rs/blob/master/crates/libghostty-vt/src/terminal.rs)

### Emulator mode state

- [`alacritty_terminal/src/term/mod.rs`](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/mod.rs)

### Leaf host references

- [`crossterm` README](https://github.com/crossterm-rs/crossterm/blob/master/README.md)

## Freshness signals

- `termwiz 0.23.3`
- `wezterm-input-types 0.1.0`
- `libghostty-vt 0.1.1` - repo `254` stars, pushed `2026-04-09`
- `crossterm 0.29.0` - repo `4010` stars, pushed `2026-04-08`
- `alacritty_terminal 0.26.0`
- `wezterm` repo `25.6k` stars, pushed `2026-04-01`
- `alacritty` repo `63.5k` stars, pushed `2026-04-14`

## Короткий вывод

🔥 Самый важный новый вывод этого прохода:

**input path должен быть terminal-state-aware и host-neutral одновременно**

Лучший shape сейчас выглядит так:

- host sends typed input intents
- Rust encoder reads current terminal modes
- shell integration lives in its own adapter/policy layer
- terminal-generated replies go through a separate output channel

То есть не:

- UI encodes bytes itself

а:

- `HostInputEvent`
- `InputEncoderPort`
- `ShellIntegrationPort`
- `TerminalResponsePort`

## Top 3 Input / shell-integration directions

### 1. Host-neutral `InputPort` + terminal-aware encoder adapters

`🎯 10   🛡️ 9   🧠 8`  
Примерно `5000-10000` строк.

Это мой текущий **лучший default**.

Идея:

- JS or any host emits typed events
- Rust owns encoding based on terminal modes
- shell integration is an adjacent adapter, not part of UI

Recommended shape:

- `HostInputEvent::Key`
- `HostInputEvent::Text`
- `HostInputEvent::Paste`
- `HostInputEvent::Mouse`
- `HostInputEvent::Focus`
- `HostInputEvent::Resize`

Почему это strongest path:

- perfect fit for Ports/Adapters
- lets Electron, web, native and tests all share the same runtime truth
- keeps protocol stable even if one UI stack changes

### 2. `termwiz + wezterm-input-types` as the strongest mature donor

`🎯 9   🛡️ 8   🧠 7`  
Примерно `4000-9000` строк.

Почему это очень сильный reference:

- `termwiz::input` already models:
  - `InputEvent`
  - `KeyboardEncoding`
  - `KeyCodeEncodeModes`
  - `Paste`
  - mouse events
  - resize events
- `wezterm-input-types` already has a rich `KeyCode` and `Modifiers` model
- `termwiz::keymap` explicitly handles ambiguous prefix sequences and `NeedData`

Это очень зрелый signal для вашего package:

- typed input model should exist as its own seam
- encoding modes deserve first-class types

### 3. `libghostty-vt` input modules as the most coherent integrated stack

`🎯 8   🛡️ 7   🧠 8`  
Примерно `4000-9000` строк.

Почему это интересно:

- separate `key`, `mouse`, `paste`, `focus`, `osc` modules
- `Encoder::set_options_from_terminal(...)`
- `Terminal::on_pty_write(...)` for replies
- strong fit if later using `libghostty-vt` as emulator core

Почему это не мой universal default:

- ecosystem and maturity still smaller than WezTerm/Alacritty gravity
- higher MSRV and younger bindings
- better as powerful backend option than the only source of architecture truth

## 1. `termwiz::input` - one of the best donor APIs in the whole ecosystem

`termwiz` turned out to be more important here than as a generic toolkit.

From `input.rs` we already get:

- `KeyboardEncoding`
  - `Xterm`
  - `CsiU`
  - `Win32`
  - `Kitty(KittyKeyboardFlags)`
- `KeyCodeEncodeModes`
  - `encoding`
  - `application_cursor_keys`
  - `newline_mode`
  - `modify_other_keys`
- `InputEvent`
  - `Key`
  - `Mouse`
  - `PixelMouse`
  - `Resized`
  - `Paste`
  - `Wake`

This is a huge architecture clue.

### Why this matters

It says the runtime should not think in raw browser keyboard events.
It should think in a host-neutral input model whose encoding depends on terminal state.

## 2. `wezterm-input-types` - rich host-neutral key model

`wezterm-input-types` is strangely named for what we need, but the content is valuable.

It already models:

- `KeyCode::Char`
- `KeyCode::Composed`
- `KeyCode::Physical`
- many function/navigation/media keys
- `Modifiers`

This is useful because it shows a realistic public DTO shape for:

- text vs composed text
- logical vs physical keys
- modifier-rich events

🔥 Very important point:

`Composed(String)` is a strong reminder that IME/composition cannot be collapsed into "one key press".

## 3. `termwiz::keymap` - ambiguous sequence handling is first-class

`termwiz::keymap` explicitly models:

- `Found::None`
- `Found::Exact`
- `Found::Ambiguous`
- `Found::NeedData`

That is not just parser trivia.
It is a strong runtime lesson:

- input and terminal replies may be incremental
- some sequences are prefix-ambiguous
- buffering policy should be explicit

This is especially relevant if the package later offers:

- low-level terminal protocol tools
- replay inspectors
- external protocol adapters

## 4. `libghostty-vt` - strongest integrated input/output contract

`libghostty-vt` is very compelling here because input/output pieces are separated cleanly.

## `key::Encoder`

It already supports:

- reusable encoder instance
- `encode_to_vec`
- fixed-buffer `encode`
- `set_options_from_terminal`
- options like:
  - cursor key application
  - keypad application
  - alt escape prefix
  - `modifyOtherKeys`

This is exactly the right direction:

🔥 key encoding should read terminal state, not hardcode escape rules in the host UI

## `mouse::Encoder`

It supports:

- tracking mode
- output format
- renderer size context
- "any button pressed" state
- motion dedup by last cell

That is very valuable because mouse encoding is not just button + x/y.

It depends on:

- protocol mode
- renderer size context
- dedup policy

## `paste`

`paste::is_safe` and `paste::encode` are particularly useful.

They make explicit that paste handling needs its own policy:

- newline safety
- bracketed paste wrapping
- unsafe control byte treatment
- `\x1b[201~` injection guard

This is a very strong finding.

⚠️ Paste should not be "just send a string".

## `focus`

`focus::Event::encode` makes focus reporting explicit via `CSI I` / `CSI O`.

This is important because focus reporting should be modeled as a capability-driven event, not random UI noise.

## `osc`

`osc::Parser` is one of the most interesting parts:

- streaming parser
- explicit reset
- finalization with terminator awareness
- typed `CommandType`

And `CommandType` already includes things like:

- `SemanticPrompt`
- `ReportPwd`
- `ClipboardContents`
- title changes
- desktop notification-related commands

🔥 This is extremely relevant for shell integration.

It suggests shell integration can be treated as typed command handling, not regex over byte streams.

## 5. `Terminal::on_pty_write` - terminal responses are a separate lane

`libghostty-vt::Terminal` documentation is explicit:

- by default, sequences requiring output are ignored
- use `on_pty_write` for replies
- callbacks are synchronous during `vt_write`

This is one of the strongest architectural rules of the whole pass.

🔥 terminal-generated replies must not be mixed with user input

Examples of such replies:

- device status reports
- DA/secondary DA
- OSC response payloads
- clipboard or pwd reports

This should become a separate runtime seam, something like:

- `TerminalResponsePort`

## 6. `alacritty_terminal` confirms which modes matter

`alacritty_terminal::TermMode` is very revealing.
It already tracks:

- `BRACKETED_PASTE`
- `FOCUS_IN_OUT`
- mouse modes
- `ALT_SCREEN`
- kitty keyboard protocol flags:
  - `DISAMBIGUATE_ESC_CODES`
  - `REPORT_EVENT_TYPES`
  - `REPORT_ALTERNATE_KEYS`
  - `REPORT_ALL_KEYS_AS_ESC`
  - `REPORT_ASSOCIATED_TEXT`

It also has:

- title stack
- `osc52` policy in config
- semantic escape chars

This tells us something very important:

⚠️ input encoding and shell integration cannot be owned by a stateless helper function

They depend on terminal modes and runtime policy.

## 7. `crossterm` is good, but only in leaf hosts

`crossterm` remains good at:

- terminal manipulation
- local terminal events
- raw mode
- bracketed paste feature support
- OSC52 feature flag

But for your architecture:

- JS UI is the main host
- Rust runtime must be host-neutral

So:

✅ `crossterm` is good for standalone Rust app shells  
⚠️ `crossterm` should not become the center of runtime truth

## Recommended layered model

```text
Host UI
  -> logical key/text/mouse/paste/focus events

Rust host-neutral input DTO layer
  -> Key
  -> Text / ComposedText
  -> Paste
  -> Mouse
  -> Focus
  -> Resize

Input encoder adapter
  -> reads current terminal modes
  -> emits PTY bytes

Shell integration adapter
  -> handles OSC 7 / semantic prompt / pwd / clipboard / title policies
  -> owns shell compatibility/conflict policy

Terminal response lane
  -> device/status replies
  -> OSC replies
  -> other terminal-generated PTY output
```

## What I would choose now

If choosing right now:

1. public host-facing input contract - custom typed DTO inspired by `wezterm-input-types`
2. encoding/reference behavior - heavily borrow from `termwiz::input`
3. if using Ghostty path later - adopt `libghostty-vt` encoders and response hooks
4. terminal-mode truth - owned by emulator/runtime, never by host UI
5. shell integration - separate adapter/policy layer, not "just some OSC markers"

## Final architectural rule

🔥 **The host should describe intent. The runtime should encode protocol.**

That is the cleanest way to keep:

- JS UI replaceable
- Rust runtime host-neutral
- shell integration explicit
- terminal replies correct

