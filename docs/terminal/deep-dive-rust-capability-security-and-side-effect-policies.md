# Deep Dive - Rust Capability Security And Side-Effect Policies

**Проверено**: 2026-04-19

## Зачем этот слой важен

Если terminal runtime должен быть:

- embeddable
- reusable in other apps
- controllable from multiple hosts
- potentially long-lived and daemonized

то security model нельзя оставлять на уровне:

- "host app как-нибудь сама проверит"
- "не будем логировать секреты, и ладно"
- "пути и URL просто строки"

🔥 Для world-class package нужен отдельный capability/effect слой:

- filesystem authority
- process launch authority
- URL/open authority
- clipboard authority
- secret storage and wiping
- shell parsing and command construction policy

Иначе runtime очень быстро превращается в ambient-authority blob.

## Primary Sources

### Capability-oriented filesystem and authority

- [`cap-std` README](https://github.com/bytecodealliance/cap-std/blob/main/README.md)
- [`cap-primitives` README](https://github.com/bytecodealliance/cap-std/blob/main/cap-primitives/README.md)

### Secret handling

- [`secrecy` README](https://github.com/iqlusioninc/crates/blob/main/secrecy/README.md)
- [`zeroize` README](https://github.com/RustCrypto/utils/blob/master/zeroize/README.md)

### URL and path parsing

- [`url` README](https://github.com/servo/rust-url/blob/master/README.md)
- [`camino`](https://github.com/camino-rs/camino)

### Shell splitting and command input

- [`shell-words`](https://github.com/tmiasko/shell-words)
- [`shlex` README](https://github.com/comex/rust-shlex/blob/master/README.md)
- [`which`](https://github.com/harryfei/which-rs.git)

## Freshness signals

- `cap-std 4.0.2` - repo `bytecodealliance/cap-std`, `766` stars, pushed `2026-02-15`
- `cap-primitives 4.0.2`
- `secrecy 0.10.3` - repo `iqlusioninc/crates`, `565` stars, pushed `2026-04-14`
- `zeroize 1.8.2`
- `url 2.5.8`
- `camino 1.2.2`
- `shell-words 1.1.1`
- `shlex 1.3.0`
- `which 8.0.2`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**security для reusable terminal runtime должна быть capability-first и effect-oriented, а не path-string-first**

На сейчас healthiest shape выглядит так:

1. runtime receives explicit authorities/capabilities
2. side effects go through typed ports
3. secrets use dedicated wrappers
4. paths and URLs are parsed into typed values early
5. shell-like string parsing stays peripheral, not central

## Top 3 directions for capability/security architecture

### 1. `cap-std + typed effect ports + secrecy/zeroize + url/camino`

`🎯 10   🛡️ 9   🧠 8`
Примерно `5000-11000` строк.

Это мой текущий **лучший default**.

Почему:

- `cap-std` gives a real capability-oriented filesystem model
- `secrecy` and `zeroize` give a disciplined secret boundary
- `url` and `camino` stop treating unsafe inputs as untyped strings
- typed ports keep open/clipboard/process side effects explicit

Это strongest path, если пакет реально должен жить в разных host apps.

### 2. `ambient std/fs but with strict effect policy and typed parsing`

`🎯 7   🛡️ 7   🧠 6`
Примерно `3500-8000` строк.

Это pragmatic fallback, если `cap-std` окажется слишком invasive for v1.

Почему:

- still possible to keep most effects explicit
- typed URL/path/secret handling already removes a lot of accidental slop
- easier migration from ordinary Rust code

Почему weaker:

- ambient authority remains in the core
- sandbox story is less composable
- harder to prove safe boundaries to downstream users

### 3. `host-only trust model with thin runtime checks`

`🎯 4   🛡️ 5   🧠 4`
Примерно `2000-5000` строк.

Это path of least resistance:

- host app decides everything
- runtime mostly trusts inbound requests

Почему это плохо for your goal:

- reusable package becomes hard to trust
- non-Electron hosts inherit hidden assumptions
- capability semantics disappear into adapters instead of living in the product core

## 1. `cap-std` is the strongest signal in this whole layer

`cap-std` matters because it attacks the right problem:

- not "how to validate a path string"
- but "how to avoid ambient authority in the first place"

Its README is unusually relevant for terminal runtimes:

- explicit capability-based `Dir`
- protection against path traversal / symlink escape classes
- application-configurable access without whole-process sandboxing

🔥 This is exactly the kind of model a reusable terminal package should want for:

- working directory roots
- allowed file access scopes
- attachment to worktrees
- session-local file operations

Practical lesson:

**do not make raw filesystem paths the primary authority token**

## 2. `cap-primitives` clarifies what should stay internal

`cap-primitives` is useful because it shows the lower layer beneath `cap-std`.

That suggests a healthy split:

- `cap-std` at the adapter or policy boundary
- `cap-primitives` only if truly needed internally

Meaning:

- public architecture should not expose primitive sandbox mechanics
- hosts should receive clear capabilities, not low-level sandbox internals

## 3. Secrets deserve first-class types

`secrecy` and `zeroize` together look like the strongest default here.

### `secrecy`

Good for:

- wrapping tokens, socket auth material, forwarded credentials
- reducing accidental logging and copying
- explicit `ExposeSecret` boundaries

### `zeroize`

Good for:

- deterministic memory wiping
- low-level secret buffers
- internal structs that need zero-on-drop

🔥 Practical takeaway:

**runtime secret handling should be opt-in explicit types, not plain `String` fields plus "be careful" comments**

## 4. URLs and paths should be typed very early

### `url`

Strong because:

- WHATWG-compliant URL parsing
- predictable handling across hosts
- better than ad hoc regex/substring parsing for external open/link surfaces

This matters for:

- OSC links
- browser-open surfaces
- callback/OAuth forwarding
- URL permission policies

### `camino`

Very useful because:

- explicit UTF-8 path policy
- clearer contracts at adapter boundaries
- especially good when configs and host APIs are expected to be string-oriented

Practical lesson:

- use typed URL and path values early
- avoid stringly-typed link/open/file policies as long as possible

## 5. Shell string parsing should stay peripheral

`shell-words` and `shlex` are both useful, but this pass made their boundary clearer.

### `shlex`

Interesting because:

- byte-oriented internals
- simple POSIX-like split behavior
- intentionally avoids customization that slows parsing

### `shell-words`

Fine as a UNIX shell-style parser helper.

### But the key lesson is:

🔥 **shell splitting is not a secure command model**

That means:

- use structured command builders where possible
- only parse shell-like input when the product actually accepts shell-like input
- never let shell splitting become the central process launch API

## 6. `which` is useful, but should not be authority

`which` is a good helper for:

- executable discovery
- diagnostics
- optional command resolution UX

But it should never become:

- proof that something is safe to execute
- the main authority model for process spawning

The runtime should still require explicit policies around:

- allowed executables
- cwd scopes
- env passthrough
- attach permissions

## 7. Recommended security/effect shape now

At this point, the healthiest shape looks like:

### Capability layer

- directory/worktree capabilities
- optional executable or environment capabilities
- session-scoped authorities

### Effect ports

- `ProcessLaunchPort`
- `ClipboardPort`
- `OpenUrlPort`
- `FileAccessPort`
- `SecretProviderPort`

### Typed input boundaries

- `Url`
- `Utf8PathBuf` or equivalent path model
- secret wrappers
- structured spawn specs instead of shell command strings

### Policy layer

- allow / deny / confirm semantics
- host-provided approval hooks
- explicit effect audit trail

## 8. What I would explicitly avoid

- ❌ raw `PathBuf` and `String` everywhere as security boundary
- ❌ ambient filesystem authority in the core if capability injection is feasible
- ❌ plain `String` secrets in long-lived session structs
- ❌ shell splitting as the default process launch model
- ❌ "host will validate it" as the only security story
- ❌ treating `which` resolution as an authorization mechanism

## Final recommendation

If building this runtime today, I would choose:

- capability-oriented file authority: `cap-std`
- lower-level primitives only as internal helpers: `cap-primitives`
- secrets: `secrecy + zeroize`
- URLs: `url`
- path boundary: `camino`
- shell splitting only as a narrow helper: `shlex` or `shell-words`

🔥 Most important practical takeaway:

**the runtime should model side effects as capabilities plus typed ports, not as convenient string APIs**

That is the only healthy way to make the package:

- reusable
- embeddable
- host-agnostic
- and still trustworthy

## Sources

- [cap-std](https://github.com/bytecodealliance/cap-std)
- [cap-primitives](https://github.com/bytecodealliance/cap-std/tree/main/cap-primitives)
- [secrecy](https://github.com/iqlusioninc/crates/tree/main/secrecy)
- [zeroize](https://github.com/RustCrypto/utils/tree/master/zeroize)
- [url](https://github.com/servo/rust-url)
- [camino](https://github.com/camino-rs/camino)
- [shell-words](https://github.com/tmiasko/shell-words)
- [shlex](https://github.com/comex/rust-shlex)
- [which](https://github.com/harryfei/which-rs.git)
