# Deep Dive - Rust Job Control, Foreground Groups, and Signal Routing

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для terminal runtime мирового уровня мало разделить:

- PTY capability
- process supervision
- shell launch

Нужно ещё честно моделировать:

- кто сейчас владеет foreground process group
- куда должен пойти `SIGINT`
- как работает suspend/resume
- почему `SIGWINCH` и resize semantics завязаны на shell/job-control reality
- где заканчивается runtime truth и начинаются Unix-specific leaves

🔥 Именно тут терминалы часто скатываются в хрупкую магию:

- prompt UX показывается, хотя shell не foreground owner
- interrupt шлётся "процессу", а надо process group
- resume/stop semantics guessed indirectly from output
- host UI начинает знать про `setpgid`, `tcsetpgrp`, `killpg`

Для reusable embeddable package это недопустимо.

## Primary Sources

### Unix syscall wrappers and signal plumbing

- [`nix` crate](https://crates.io/crates/nix)
- [`nix` repo](https://github.com/nix-rust/nix)
- [`rustix` crate](https://crates.io/crates/rustix)
- [`rustix` repo](https://github.com/bytecodealliance/rustix)
- [`signal-hook` crate](https://crates.io/crates/signal-hook)
- [`signal-hook` repo](https://github.com/vorner/signal-hook)

### PTY-attached process layers

- [`portable-pty` crate](https://crates.io/crates/portable-pty)
- [`pty-process` crate](https://crates.io/crates/pty-process)
- [`wezterm` repo](https://github.com/wezterm/wezterm)

### Related supervision donor

- [`process-wrap` crate](https://crates.io/crates/process-wrap)

## Freshness signals

- `nix 0.31.2` - repo `nix-rust/nix`, `3019` stars, pushed `2026-04-17`
- `rustix 1.1.4` - repo `bytecodealliance/rustix`, `1966` stars, pushed `2026-04-18`
- `signal-hook 0.4.4` - repo `vorner/signal-hook`, `848` stars, pushed `2026-04-04`
- `pty-process 0.5.3`
- `portable-pty 0.9.0`
- `wezterm` repo `25k+` stars, pushed `2026-04-18`
- `process-wrap 9.1.0`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**job control is its own runtime concern above PTY and below host UX**

Healthiest shape сейчас выглядит так:

1. PTY/session layer owns controlling-terminal setup
2. job-control layer owns foreground process-group semantics
3. signal routing is explicit policy, not guessed from UI state
4. host sends typed intents like interrupt, suspend, continue, resize
5. Unix-specific job-control plumbing stays in Unix leaves

То есть не:

- "spawn + kill is enough"

и не:

- "UI can infer foreground ownership from prompt visibility"

а:

- `JobControlPolicy`
- `ForegroundOwnerState`
- `SignalIntent`
- Unix leaf for `setpgid` / `tcsetpgrp` / `killpg`

## Top 3 directions for job-control architecture

### 1. `Typed JobControlPolicy + explicit foreground-owner state + Unix leaf for pgid/tcsetpgrp/killpg`

`🎯 10   🛡️ 8   🧠 9`
Примерно `7000-15000` строк.

Это strongest default.

Идея:

- runtime models foreground ownership explicitly
- UI sends typed intents, not raw signals
- Unix adapter handles real process-group and terminal control details
- shell UX, prompt UX and interrupt behavior all read from the same truth

Почему это лучший путь:

- removes a lot of prompt/interrupt heuristics
- keeps Unix-specific complexity out of public host APIs
- lets prompt intelligence obey actual foreground ownership
- keeps resize and signal policy testable

### 2. `Supervision-only runtime + heuristic foreground inference`

`🎯 6   🛡️ 5   🧠 5`
Примерно `4000-9000` строк.

Идея:

- runtime knows process/session lifecycle
- foreground ownership is inferred from shell markers, output and app state
- signals are routed with best-effort guesses

Почему это выглядит проще:

- smaller implementation
- less Unix-specialized plumbing

Почему это weakens the product:

- prompt UX becomes flaky
- TUI/shell transitions get misdetected
- interrupts and resume behavior can drift from real terminal semantics

### 3. `Expose raw pgid and signal primitives to hosts`

`🎯 3   🛡️ 4   🧠 7`
Примерно `3000-7000` строк.

Это плохой путь.

Симптомы:

- host adapters need Unix knowledge
- Node/Electron layer starts carrying terminal semantics
- public API leaks platform-specific internals
- cross-language embed story gets worse immediately

## 1. PTY/session setup is not the same thing as job control

This boundary became much clearer after revisiting `portable-pty`, `pty-process`, and the earlier PTY/supervision research.

PTY/session setup gives you things like:

- child attached to a PTY
- session leader semantics
- controlling terminal setup

But job control adds another layer:

- foreground process group
- stop/continue flows
- signal routing to the right group
- shell regaining foreground ownership after child completion

🔥 Strong rule:

**controlling terminal setup is not enough to model foreground ownership**

## 2. `nix` is still the strongest ergonomic Unix leaf for job-control primitives

`nix 0.31.2` remains the strongest practical donor for the Unix leaf here.

Why:

- mature Unix ergonomics
- process/signal/session APIs are explicit
- fits adapter-level code better than trying to invent your own wrappers from scratch

This is the layer where functions around:

- process groups
- sessions
- waiting
- signaling

feel most natural.

Healthy interpretation:

- let `nix` power Unix job-control adapters
- do not let `nix` become the public mental model of the package

## 3. `rustix` is an excellent lower-level companion, but not the whole job-control story

`rustix 1.1.4` is very strong and active.

Why it matters:

- safe syscall-oriented control
- great internal leaf for precise Unix behavior
- likely better long-term low-level base than many ad hoc libc calls

But the practical boundary is:

- `rustix` gives you low-level building blocks
- the package still needs a higher-level job-control model above that

So the best role here is:

- internal Unix leaf
- maybe precision-oriented path for specific syscalls
- not the public API shape

## 4. `signal-hook` should stay a narrow bridge, not the control plane

`signal-hook 0.4.4` is valuable, but the earlier lesson remains true:

- signals are global
- handlers are constrained
- signal ownership is dangerous to spread around the codebase

For this package that means:

- use signal helpers in narrow Unix adapters when truly needed
- keep public runtime semantics command/state oriented
- do not turn signal handlers into architecture center

🔥 Practical rule:

**runtime should model `Interrupt`, `Suspend`, `Continue`, `ResizeNotice`, not raw signal-wrangling as public truth**

## 5. `portable-pty` and `pty-process` help launch PTY-attached children, but they do not eliminate job-control policy

This is where many designs get confused.

`portable-pty` and `pty-process` help with:

- PTY-backed child launch
- session/control-terminal setup
- process attachment to PTY

They do **not** automatically solve:

- which process group owns foreground
- when shell has regained control
- how to route group-directed interrupts
- when prompt UX may safely assume shell ownership

So the right reading is:

- PTY crates give necessary lower layers
- job-control semantics still need a dedicated runtime seam

## 6. Prompt UX and intelligence should obey foreground ownership

This pass strongly reinforces a product rule that already surfaced in prior research:

- prompt suggestions
- shell ghost text
- command-entry assumptions

should only engage when shell truly owns foreground again.

Why:

- TUI app in alt-screen may still be running
- shell output may be visible without shell being the actual owner
- process exit and redraw timing can lag

🔥 This is exactly why foreground-owner state should be explicit instead of inferred from "looks like a prompt".

## 7. Resize and `SIGWINCH` semantics are part of the same reality

We already had signals from previous restore/hydration passes that:

- startup resize races are real
- shell may install its `SIGWINCH` handling late
- early reveal can show broken state

This pass sharpens the conclusion:

- resize is not just a UI event
- it participates in terminal/job-control timing

So the healthiest design is:

- host expresses resize intent
- runtime routes it according to actual session/job state
- prompt UX and recovery gates remain aware that shell may not yet have processed the change

## 8. Windows should not fake Unix job-control semantics

This layer is especially important because it highlights another cross-platform truth:

- Unix has real foreground process groups and job control
- Windows has different supervision/job semantics

So a good universal package should:

- expose one host-neutral intent model
- let Unix leaves implement true job control
- let Windows leaves expose capability differences honestly

⚠️ Trying to pretend the same low-level semantics exist on both sides creates worse APIs.

## Practical verdict

If I were designing this layer right now:

### V1

- explicit `JobControlPolicy`
- explicit `ForegroundOwnerState`
- `SignalIntent::{Interrupt, Suspend, Continue, ResizeNotice}`
- Unix adapter using `nix` first, `rustix` where lower-level precision helps
- `signal-hook` only in narrow glue
- no raw Unix process-group concepts in host-facing APIs

### V2

- stronger per-shell job-control detection
- richer restart/rehydrate behavior keyed by foreground-owner truth
- capability negotiation for hosts that cannot support certain semantics

## Чего я бы избегал

- ❌ Treating PTY launch as equivalent to full job control
- ❌ Inferring foreground ownership only from prompt-looking output
- ❌ Sending raw Unix semantics into host adapters
- ❌ Making signal handlers the architecture center
- ❌ Pretending Windows and Unix have the same low-level job-control model

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- job control deserves its own bounded context
- host APIs should speak in typed intents, not raw signals or pgids
- foreground ownership should be explicit runtime truth
- prompt UX and shell intelligence should gate on that truth
- Unix plumbing belongs in Unix adapters, not in cross-language SDK surfaces

## Sources

- [nix crate](https://crates.io/crates/nix)
- [nix repo](https://github.com/nix-rust/nix)
- [rustix crate](https://crates.io/crates/rustix)
- [rustix repo](https://github.com/bytecodealliance/rustix)
- [signal-hook crate](https://crates.io/crates/signal-hook)
- [signal-hook repo](https://github.com/vorner/signal-hook)
- [portable-pty crate](https://crates.io/crates/portable-pty)
- [pty-process crate](https://crates.io/crates/pty-process)
- [wezterm repo](https://github.com/wezterm/wezterm)
- [process-wrap crate](https://crates.io/crates/process-wrap)
