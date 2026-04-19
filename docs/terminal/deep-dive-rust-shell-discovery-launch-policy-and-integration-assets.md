# Deep Dive - Rust Shell Discovery, Launch Policy, and Integration Assets

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для embeddable terminal package мирового уровня мало уметь:

- открыть PTY
- заспавнить child process
- прокинуть input/output

Нужно ещё правильно решать:

- какой shell вообще запускать
- как различать login / interactive / command / bootstrap modes
- как не превратить launch API в `String` soup
- как доставлять shell integration scripts и bootstrap assets
- как не смешать shell discovery, authority policy и UX defaults

🔥 Именно здесь многие terminal runtimes становятся хрупкими:

- `shell -lc "..."` превращается в универсальный молоток
- quoting и escaping начинает жить в random helpers
- shell integration scripts тащатся как raw string constants
- executable discovery путается с authorization
- разные host apps получают разное bootstrap behavior без явной модели

Для universal package этот слой должен быть отдельным bounded context.

## Primary Sources

### Discovery and shell-like parsing helpers

- [`which` crate](https://crates.io/crates/which)
- [`which-rs` repo](https://github.com/harryfei/which-rs)
- [`shell-words` crate](https://crates.io/crates/shell-words)
- [`shell-words` repo](https://github.com/tmiasko/shell-words)
- [`shlex` crate](https://crates.io/crates/shlex)
- [`shlex` repo](https://github.com/comex/rust-shlex)

### Embedded script and asset delivery

- [`rust-embed` crate](https://crates.io/crates/rust-embed)
- [`include_dir` crate](https://crates.io/crates/include_dir)
- [`include_dir` repo](https://github.com/Michael-F-Bryan/include_dir)

### Child-process extras

- [`command-fds` crate](https://crates.io/crates/command-fds)
- [`command-fds` repo](https://github.com/google/command-fds)

### Related runtime context

- [`portable-pty` crate](https://crates.io/crates/portable-pty)
- [`process-wrap` crate](https://crates.io/crates/process-wrap)

## Freshness signals

- `which 8.0.2` - repo `harryfei/which-rs`, `260` stars, pushed `2026-03-08`
- `shell-words 1.1.1` - repo `tmiasko/shell-words`, `79` stars, pushed `2025-12-10`
- `shlex 1.3.0`
- `rust-embed 8.11.0`
- `include_dir 0.7.4` - repo `Michael-F-Bryan/include_dir`, `390` stars, pushed `2024-07-05`
- `command-fds 0.3.3` - repo `google/command-fds`, `50` stars, pushed `2026-04-15`
- `portable-pty 0.9.0`
- `process-wrap 9.1.0`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**shell launch must be modeled as typed policy and typed assets, not as shell strings with convenience flags**

Healthiest shape сейчас выглядит так:

1. discovery is a helper adapter, not authority
2. launch intent is represented as a typed `ShellLaunchSpec`
3. shell integration assets are embedded or versioned resources, not ad hoc strings
4. shell-like parsing stays peripheral
5. host apps choose defaults, but runtime owns launch semantics

То есть не:

- `"bash -lc ..."` как public API

а:

- `ShellProfile`
- `ShellKind`
- `ShellLaunchMode`
- `ShellLaunchSpec`
- `ShellIntegrationAsset`

## Top 3 directions for shell bootstrap architecture

### 1. `Typed ShellLaunchSpec + discovery adapter + embedded integration assets + argv-first launch`

`🎯 10   🛡️ 9   🧠 8`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- shell is discovered or selected through an explicit adapter
- launch mode is typed: login, interactive, command, integration bootstrap
- actual spawn path is argv-first, not shell-string-first
- integration scripts are shipped as versioned embedded assets
- helper parsers stay at product edges only

Почему это лучший путь:

- eliminates a lot of quoting bugs
- keeps behavior stable across hosts
- allows package to ship shell integration as real assets
- makes standalone app and embedded Electron host consume the same launch model

### 2. `Typed core with shell-string helpers around product edges`

`🎯 8   🛡️ 7   🧠 6`
Примерно `5000-10000` строк.

Это рабочий компромисс.

Идея:

- core still uses typed launch spec
- some product edges accept shell-like text and translate it via `shlex` or `shell-words`
- good for CLI UX or import paths from other tools

Почему это нормально:

- shell syntax can be supported where product explicitly wants it
- parsing is pushed to boundary adapters, not core

Почему не strongest:

- easy to let boundary convenience leak inward
- different shells still need explicit launch policies

### 3. `String command model with shell -lc everywhere`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-8000` строк.

Это плохой путь.

Симптомы:

- quoting and escaping become product logic
- launch behavior differs by host and shell family
- integration bootstrap scripts become raw string concatenation
- discovery, authority and execution get blurred together

## 1. `which` is useful, but only as discovery helper

`which 8.0.2` now looks like a healthy helper for:

- resolving shell executable paths
- diagnostics
- optional shell auto-discovery UX

But its role must stay narrow.

🔥 Strong rule:

**`which` is not authority**

It can help answer:

- "where is `bash`?"
- "is `zsh` present?"

It must not answer:

- "is this allowed to run?"
- "is this the canonical launch behavior?"

That policy belongs elsewhere.

## 2. `shell-words` and `shlex` should stay peripheral

We already had a security-oriented conclusion here. This pass reinforces it from the launch-design angle.

### `shell-words`

Useful for:

- explicit Unix-shell style parsing when product accepts shell syntax

### `shlex`

Useful for:

- simple POSIX-like splitting
- lightweight parsing helpers

But the architectural rule remains:

🔥 **parsing shell syntax is not the same thing as modeling shell launch**

That means:

- parse only at explicit product edges
- never let parsers define the spawn contract
- keep core launch model structured

## 3. `rust-embed` and `include_dir` are interesting for shell integration assets

For a world-class terminal package, shell integration scripts are not "random text files".

They are:

- versioned runtime assets
- part of bootstrap compatibility
- sometimes needed by multiple host apps

### `include_dir`

`include_dir 0.7.4` looks like the cleaner default for this package right now.

Why:

- small conceptual footprint
- clearly about embedding directory assets
- fits script/template bundles well

### `rust-embed`

`rust-embed 8.11.0` is powerful, but feels broader and more web/static oriented.

Why it is interesting:

- compile-time asset embedding
- dev-vs-release asset behavior
- richer asset delivery worldview

Why I would not default to it here:

- more capability than we need for simple shell integration assets
- stronger framework-ish feel

Practical takeaway:

- shell integration assets should be packaged as explicit embedded resources
- `include_dir` looks like the lighter default
- `rust-embed` is a valid richer alternative if asset pipeline needs grow

## 4. `command-fds` is a strong optional Unix leaf

`command-fds 0.3.3` is very interesting for advanced bootstrap flows.

Where it can matter:

- passing extra file descriptors to child processes
- richer local control/data planes without temp files
- advanced bootstrap of shell helpers or sidecars

Why this is not core default:

- advanced and platform-shaped
- easy to overfit architecture around a clever low-level seam

Healthy interpretation:

- keep it as optional Unix-oriented infra leaf
- do not build the whole public shell-launch story around inherited FDs

## 5. Shell integration assets should be versioned and typed

This is the most important product-level insight from this pass.

Package likely needs to ship:

- shell integration fragments
- prompt marker helpers
- maybe shell-specific bootstrap snippets

Those should not be modeled as:

- random `&'static str`
- concatenated string templates
- host-specific hardcoded files

Better shape:

- `ShellIntegrationAsset { shell_kind, version, content, install_mode }`
- typed asset registry
- host-independent asset resolution

That makes:

- package upgrades safer
- compatibility reasoning easier
- Electron host and standalone app consistent

## 6. Shell family differences should be explicit in policy

Even without going deep into every shell implementation, one architectural point is already obvious:

- different shells want different flags
- login vs interactive semantics differ
- bootstrap injection points differ

So the right model is:

- explicit `ShellKind`
- explicit `ShellLaunchMode`
- explicit `IntegrationStrategy`

Not:

- "we have one generic shell string"

## 7. Launch policy should sit above PTY and below host UX

Healthy layering for this package now looks like:

### Host/product layer

- chooses desired shell profile or default shell UX
- maybe lets user configure shell selection

### Shell launch layer

- resolves shell executable if needed
- computes argv and env rules
- selects integration asset
- decides login/interactive/bootstrap behavior

### PTY/process layer

- only receives structured executable + argv + env + cwd + launch policy outputs

🔥 This prevents PTY layer from becoming a shell-policy layer.

## Practical verdict

If I were designing this layer right now:

### V1

- typed `ShellLaunchSpec`
- `which` only as optional discovery helper
- `shell-words` / `shlex` only in UX adapters
- `include_dir` as default asset-embedding brick for shell integration files
- `command-fds` not in minimal core, only as optional advanced leaf
- structured argv/env/cwd model flowing into PTY/process adapters

### V2

- shell-specific integration asset registry
- richer per-shell compatibility testing
- optional advanced FD-passing flows where real need exists
- host-configurable shell profile resolution

## Чего я бы избегал

- ❌ `shell -lc` as the main public launch API
- ❌ String concatenation for bootstrap scripts
- ❌ Letting parser helpers define runtime launch semantics
- ❌ Using `which` as authorization or trust model
- ❌ Making shell integration files host-specific ad hoc assets

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- shell launch needs its own bounded context
- launch intent should be typed, not stringly
- integration scripts should be embedded/versioned assets
- discovery and authorization must stay separate
- shell parsers should remain narrow UX helpers

## Sources

- [which crate](https://crates.io/crates/which)
- [which-rs repo](https://github.com/harryfei/which-rs)
- [shell-words crate](https://crates.io/crates/shell-words)
- [shell-words repo](https://github.com/tmiasko/shell-words)
- [shlex crate](https://crates.io/crates/shlex)
- [shlex repo](https://github.com/comex/rust-shlex)
- [rust-embed crate](https://crates.io/crates/rust-embed)
- [include_dir crate](https://crates.io/crates/include_dir)
- [include_dir repo](https://github.com/Michael-F-Bryan/include_dir)
- [command-fds crate](https://crates.io/crates/command-fds)
- [command-fds repo](https://github.com/google/command-fds)
- [portable-pty crate](https://crates.io/crates/portable-pty)
- [process-wrap crate](https://crates.io/crates/process-wrap)
