# Deep Dive - Rust Cross-Platform PTY Backends And OS Seams

**Проверено**: 2026-04-19

## Зачем этот deep dive

Для world-class terminal package очень легко сказать себе:

- "PTY у нас уже кроссплатформенный"
- "возьмём один crate и забудем про OS differences"
- "Windows и Unix пусть будут просто разными adapters"

Но в реальности PTY layer почти всегда тащит за собой очень важные platform seams:

- controlling terminal и session leader semantics на Unix
- ConPTY lifecycle, synchronous pipes и version gating на Windows
- resize semantics
- file descriptor / handle hygiene
- child-process launch oddities
- signal vs job/session behavior

🔥 Если этот слой спроектировать небрежно, то потом поверх него уже нельзя честно построить universal terminal runtime без platform debt.

## Primary Sources

- [`portable-pty` crate docs](https://docs.rs/portable-pty)
- [`portable_pty::unix` docs](https://docs.rs/portable-pty/latest/portable_pty/unix/index.html)
- [`pty-process` docs](https://docs.rs/pty-process/latest/pty_process/)
- [Microsoft `CreatePseudoConsole` docs](https://learn.microsoft.com/en-us/windows/console/createpseudoconsole)
- [`windows` crate docs](https://microsoft.github.io/windows-docs-rs/)
- [`rustix` crate docs](https://docs.rs/rustix)
- [`nix` crate docs](https://docs.rs/nix)
- [`winpty-rs` crate docs](https://docs.rs/winpty-rs/1.0.5/winptyrs/pty/index.html)
- [`conpty` crate docs](https://docs.rs/conpty)

## Freshness signals

- `portable-pty 0.9.0` - repo `wezterm/wezterm`, `25634` stars, updated `2026-04-19`
- `pty-process 0.5.3`
- `rustix 1.1.4` - repo `bytecodealliance/rustix`, `1966` stars, updated `2026-04-18`
- `nix 0.31.2` - repo `nix-rust/nix`, `3019` stars, updated `2026-04-18`
- `windows 0.62.2` - repo `microsoft/windows-rs`, `12116` stars, updated `2026-04-18`
- `winpty-rs 1.0.5` - repo `andfoy/winpty-rs`, `41` stars, updated `2026-03-31`
- `conpty 0.7.0` - repo `zhiburt/conpty`, `22` stars, updated `2025-12-09`
- `pseudoterminal 0.2.1` - repo `michaelvanstraten/pseudoterminal`, `16` stars, updated `2026-03-04`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**кроссплатформенный PTY layer должен быть одним public port, но не одним фальшиво-одинаковым internal implementation**

То есть healthiest architecture выглядит так:

1. hosts and upper runtime see one `PtyPort`
2. Unix and Windows stay explicit infrastructure leaves
3. platform-specific launch, resize, cleanup and teardown rules stay below the port
4. public runtime contract never leaks raw OS handles or platform-specific quirks

Если сделать наоборот, получится либо:

- fake "unified PTY", который ломается на Windows edge cases
- либо слишком low-level API, который уже нельзя удобно встраивать в другие приложения

## Top 3 directions for the PTY/backend layer

### 1. `portable-pty + dedicated supervision layer + OS-specific leaf adapters under one port`

`🎯 10   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк.

Что это значит:

- public runtime depends on one `PtyPort`
- default adapter uses `portable-pty`
- process supervision, detach/kill/timeout policy остаётся отдельным слоем
- Windows and Unix specifics stay hidden in platform leaves

Почему это strongest path:

- `portable-pty` already gives a trait-shaped abstraction and runtime-selectable implementation idea
- it has real cross-platform gravity via WezTerm
- it hides a lot of ugly PTY setup detail without pretending all OS semantics are identical

Где риск:

- crate всё равно не отменяет platform differences
- Windows-specific lifecycle and Unix session semantics нужно моделировать явно выше/рядом

Практический вывод:

✅ Это мой лучший default для reusable terminal package.

### 2. `Direct rustix/nix on Unix + windows-rs ConPTY on Windows`

`🎯 8   🛡️ 8   🧠 9`  
Примерно `8000-16000` строк.

Что это значит:

- Unix side built with `rustix` and selected `nix` helpers
- Windows side built directly with `windows` bindings for `CreatePseudoConsole` and friends
- your package owns the entire PTY adapter layer

Почему это интересно:

- maximum control over semantics
- easiest way to build a truly deliberate world-class PTY adapter if you are willing to pay the cost
- no dependency on one higher-level abstraction’s design compromises

Где риск:

- much higher maintenance load
- you own every platform regression
- Windows and Unix codepaths become serious product subsystems, not just adapters

Практический вывод:

✅ Сильный long-term path, если PTY layer itself becomes a differentiator.  
⚠️ Для v1 это уже тяжёлый bet.

### 3. `Helper-stack composition from narrower crates`

`🎯 5   🛡️ 6   🧠 6`  
Примерно `4000-9000` строк.

Что это значит:

- `pty-process` on Unix
- `winpty-rs` / `conpty` on Windows
- maybe `pseudoterminal` as a younger cross-platform option

Почему это интересно:

- good for experiments and donor code
- sometimes simpler if one host needs only one narrow use-case

Почему it is weaker for your goal:

- this is not one strong universal package story
- maturity and adoption are much lower
- you start composing your own portability guarantees from smaller building blocks

Практический вывод:

⚠️ Хороший R&D path and donor pool.  
❌ Не мой default for a world-class reusable package.

## Tool-by-tool findings

## 1. `portable-pty` - still the strongest default PTY port

- Crate: [`portable-pty`](https://crates.io/crates/portable-pty)
- Latest: `0.9.0`
- Repo stars: `25634` through `wezterm/wezterm`
- Repo updated: `2026-04-19`

Что особенно важно:

- docs explicitly say it provides a cross-platform API for system PTY interfaces
- unlike many smaller crates, it is shaped around traits and runtime-selected implementations
- `CommandBuilder` is intentionally similar to `std::process::Command`

Ещё более важный low-level signal:

- `portable_pty::unix` docs explicitly mention platform cleanup workarounds
- on Big Sur Cocoa leaks file descriptors into child processes
- on Linux GNOME/Mutter shell extensions may leak FDs too

Почему это важно:

- world-class PTY layer is not just `openpty + spawn`
- descriptor hygiene is part of the platform adapter contract

Итог:

✅ Самый сильный practical default for `PtyPort`.

## 2. `pty-process` - very useful Unix donor, but not a universal core

- Crate: [`pty-process`](https://crates.io/crates/pty-process)
- Latest: `0.5.3`

Что особенно важно:

- docs are unusually explicit
- it wraps `tokio::process::Command` or `std::process::Command`
- child becomes a session leader of a new session
- controlling terminal of that session is set to the PTY
- docs.rs platforms shown are Unix targets, not Windows

Почему это важно:

- this crate captures a real Unix semantic seam cleanly
- but it is not pretending to be a complete cross-platform product truth

Итог:

✅ Очень хороший Unix-specific donor.  
⚠️ Не лучший universal default.

## 3. Windows ConPTY is a real architectural seam, not just another backend

Microsoft docs for `CreatePseudoConsole` are unusually direct:

- it creates a pseudoconsole object
- input and output are streams carrying UTF-8 text plus VT sequences
- handles must be closed with `ClosePseudoConsole`
- input and output streams are currently restricted to synchronous I/O
- minimum supported client is Windows 10 version 1809
- `PSEUDOCONSOLE_INHERIT_CURSOR` requires asynchronous handling on a background thread or the caller may hang

Почему это важно:

- Windows PTY is not "Unix PTY but on another OS"
- the API shape itself pushes you toward:
  - different lifecycle concerns
  - different reader/writer plumbing
  - careful background handling

Итог:

🔥 ConPTY should be treated as a first-class Windows adapter seam with its own policy and tests.

## 4. `windows` - the right low-level Windows leaf, not a public SDK truth

- Crate: [`windows`](https://crates.io/crates/windows)
- Latest: `0.62.2`
- Repo stars: `12116`
- Repo updated: `2026-04-18`

Что особенно важно:

- this is the right low-level binding layer if you go direct on Windows
- it keeps your Windows adapter honest and explicit

Но:

- it should stay in the infrastructure ring
- host-neutral runtime contract should never expose `HPCON`, `HANDLE` or other Windows-specific identities

Итог:

✅ Strong Windows adapter foundation.  
⚠️ Not the public PTY contract.

## 5. `rustix` and `nix` - powerful Unix tools, but not the same thing

### `rustix`

- Crate: [`rustix`](https://crates.io/crates/rustix)
- Latest: `1.1.4`
- Repo stars: `1966`
- Repo updated: `2026-04-18`

Best role:

- low-level, explicit syscall-safe Unix leaf
- better fit when you want tight deliberate adapters

### `nix`

- Crate: [`nix`](https://crates.io/crates/nix)
- Latest: `0.31.2`
- Repo stars: `3019`
- Repo updated: `2026-04-18`

Best role:

- pragmatic Unix helper layer
- still valuable for selected APIs or migration paths

Practical lesson:

✅ `rustix` looks like the healthier long-term low-level foundation.  
✅ `nix` is still useful, but I would avoid letting it define the entire Unix portability story.

## 6. `winpty-rs` and `conpty` - useful Windows donors, not my main cross-platform bet

### `winpty-rs`

- Latest: `1.0.5`
- supports both `ConPTY` and `WinPTY`

Why interesting:

- useful if legacy Windows compatibility matters
- gives a concrete reference for Windows-specific PTY packaging

Why limited:

- Windows-only
- much smaller ecosystem gravity

### `conpty`

- Latest: `0.7.0`
- focused interface around ConPTY

Why interesting:

- narrow adapter donor for direct Windows work

Why limited:

- too narrow to be the center of a universal package

Итог:

✅ Good Windows donor crates.  
⚠️ Not the main public PTY story.

## 7. `pseudoterminal` - interesting younger cross-platform alternative

- Crate: [`pseudoterminal`](https://crates.io/crates/pseudoterminal)
- Latest: `0.2.1`
- Repo stars: `16`
- Repo updated: `2026-03-04`

Почему интересно:

- explicitly positions itself as cross-platform
- async support is attractive

Почему пока не default:

- ecosystem gravity is still very small
- much less proven than `portable-pty`

Итог:

⚠️ Good to watch, not my current default.

## Recommended architecture rules

### 1. Keep one public PTY port, but separate OS leaves

Public runtime should see one `PtyPort`.

Inside infrastructure:

- Unix leaf
- Windows leaf
- maybe future mock/testing leaf

### 2. Never leak OS-native PTY identities into the host-neutral contract

Do not expose:

- raw FDs
- raw Windows `HANDLE`
- `HPCON`
- Unix session/process group details as public truth

Instead expose:

- `SessionId`
- `PtyRoute`
- `Resize`, `Write`, `Kill`, `Attach`, `Detach`
- capability and health metadata

### 3. Initial dimensions are not cosmetic

Both ConPTY and Unix PTY flows make early size part of real semantics.

So:

- PTY should start with explicit dimensions
- late resize is not a substitute for correct initial setup

### 4. Descriptor and handle hygiene belong to the platform adapter

If one platform leaks inherited FDs or handles into children, that is not "someone else’s bug".  
It becomes your PTY adapter responsibility.

### 5. Process supervision still stays separate from PTY capability

Even with a strong PTY crate:

- kill policy
- timeout escalation
- detach semantics
- orphan/session handling

must remain a separate layer.

### 6. Windows minimum support must be a product decision, not a surprise

ConPTY means:

- minimum Windows 10 1809 / Server 2019
- explicit policy for older systems
- explicit decision on whether you support WinPTY fallback or not

## Bottom line

Если свести весь deep dive к одной фразе:

🔥 **a universal Rust terminal package should unify PTY capability at the port level, not by pretending Unix PTYs and Windows ConPTY are the same subsystem internally**

Это тот случай, где честная platform-specific architecture делает package более reusable, а не менее.
