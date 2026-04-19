# Deep Dive - Rust Credential Stores, SSH Agent, and Forwarding Boundaries

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для universal terminal package remote/runtime story быстро упирается не только в transport, но и в credential reality:

- где хранить local secrets
- как говорить с OS keychain / secret service
- как использовать SSH agent
- как моделировать agent forwarding
- как не смешать topology state и sensitive material

Уже из прошлых проходов было ясно:

- secrets should be wrapped
- credentials should not live in topology blobs
- remote routes need explicit authority boundaries

Но этого недостаточно.

🔥 Для world-class embeddable package нужно ещё отдельно решить:

- что является `SecretValue`
- что является `CredentialReference`
- что является `AgentCapability`
- что можно forward-ить, а что нельзя by default

Именно тут архитектура часто ломается:

- keychain API тащится в core
- ssh-agent socket path живёт как просто `String`
- forwarded agent becomes ambient authority
- known hosts and keys mix with generic config

## Primary Sources

### Secret/keychain crates

- [`keyring` crate](https://crates.io/crates/keyring)
- [`open-source-cooperative/keyring-rs` repo](https://github.com/open-source-cooperative/keyring-rs)
- [`oo7` crate](https://crates.io/crates/oo7)
- [`linux-credentials/oo7` repo](https://github.com/linux-credentials/oo7)
- [`secrecy` crate](https://crates.io/crates/secrecy)
- [`iqlusioninc/crates` repo](https://github.com/iqlusioninc/crates)
- [`zeroize` crate](https://crates.io/crates/zeroize)

### SSH/agent/key material crates

- [`russh` crate](https://crates.io/crates/russh)
- [`openssh` crate](https://crates.io/crates/openssh)
- [`openssh-mux-client` crate](https://crates.io/crates/openssh-mux-client)
- [`ssh-key` crate](https://crates.io/crates/ssh-key)
- [`RustCrypto/SSH` repo](https://github.com/RustCrypto/SSH)
- [`ssh-agent-client-rs` crate](https://crates.io/crates/ssh-agent-client-rs)
- [`nresare/ssh-agent-client-rs` repo](https://github.com/nresare/ssh-agent-client-rs)

## Freshness signals

- `keyring 4.0.0-rc.3` - repo `open-source-cooperative/keyring-rs`, `720` stars, pushed `2026-04-14`
- `oo7 0.6.0-alpha` - repo `linux-credentials/oo7`, `230` stars, pushed `2026-04-15`
- `secrecy 0.10.3` - repo `iqlusioninc/crates`, `565` stars, pushed `2026-04-14`
- `zeroize 1.8.2`
- `russh 0.60.0` - repo `Eugeny/russh`, `1673` stars, pushed `2026-04-13`
- `openssh 0.11.6` - repo `openssh-rust/openssh`, `267` stars, pushed `2026-04-09`
- `openssh-mux-client 0.17.9`
- `ssh-key 0.7.0-rc.9` - repo `RustCrypto/SSH`, `217` stars, pushed `2026-04-14`
- `ssh-agent-client-rs 1.1.2` - repo `nresare/ssh-agent-client-rs`, `9` stars, pushed `2025-10-14`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**credential storage, agent usage and forwarding should be modeled as separate capability layers, not as one "SSH auth" blob**

Healthiest shape сейчас выглядит так:

1. secret values stay wrapped and short-lived
2. persistent credential storage is a leaf adapter
3. agent access is a separate capability from stored secrets
4. forwarding is an explicit route/policy decision
5. remote transport adapters consume typed credential references, not raw strings

То есть не:

- "у нас есть SSH credentials"

а:

- `CredentialReference`
- `SecretMaterial`
- `AgentAccessPolicy`
- `ForwardingPolicy`
- `KnownHostPolicy`

## Top 3 directions for credential/agent architecture

### 1. `Typed CredentialProviderPort + AgentProviderPort + secret wrappers + explicit forwarding policy`

`🎯 10   🛡️ 9   🧠 8`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- runtime core knows only typed credential references and secret wrappers
- keychain/keyring integration stays in host/infrastructure leaves
- ssh-agent access is modeled separately from static key material
- forwarding is opt-in per route/session policy

Почему это лучший путь:

- clean separation of authority
- embedders can pick their own storage backend
- forwarding becomes auditable policy instead of accidental inheritance
- remote adapters stop depending on ambient env and socket assumptions

### 2. `Runtime-owned OS keyring integration as primary secret store`

`🎯 7   🛡️ 7   🧠 6`
Примерно `5000-11000` строк.

Это workable, but with caveats.

Идея:

- runtime directly integrates with system keychain/keyring
- most credential references resolve locally inside Rust package

Почему это интересно:

- simpler story for standalone app
- strong local UX if package ships its own desktop host

Почему не strongest universal default:

- embedders may already have a credential policy
- current best crates are still `rc/alpha` on the most interesting lines
- cross-host neutrality weakens

### 3. `Ad hoc env/file/socket based credential handling`

`🎯 3   🛡️ 4   🧠 4`
Примерно `3000-7000` строк.

Это плохой путь.

Симптомы:

- credentials leak into general config
- agent socket paths become ambient assumptions
- forwarding happens because the environment happened to contain it

## 1. `keyring` looks like the strongest OS keychain leaf right now, but it is still RC

`keyring 4.0.0-rc.3` has the best ecosystem gravity in this layer today.

Why it matters:

- explicit OS keyring/keychain orientation
- strong repo signal
- plausible default leaf for standalone desktop hosts

But the architectural takeaway is not "put keyring in core".

It is:

- keyring integration should stay an infrastructure leaf
- embedders may choose to provide their own credential store instead

⚠️ Also important:

- current line is still `rc`
- for a universal package that matters

So:

- strong leaf candidate
- not a reason to hardwire runtime core to OS keychains

## 2. `oo7` is interesting, but not a universal default

`oo7 0.6.0-alpha` is very interesting because it leans into Secret Service / keychain style flows.

It is useful as a signal that:

- there is real demand for richer Linux credentials integration
- secret-service style runtime leaves can be decent product differentiators

But it is even less conservative than `keyring` right now:

- alpha line
- stronger platform shaping

So healthiest interpretation now:

- watchlist / specialized Linux leaf
- not default universal package dependency

## 3. `secrecy + zeroize` still form the right baseline under all of this

This pass only reinforced the earlier conclusion.

Why:

- stored credential reference and in-memory secret are different things
- forwarded credential and persisted credential are different things
- any actual secret material should stay wrapped and minimized in lifetime

🔥 Strong rule:

**keychain integration is not a substitute for in-memory secret discipline**

So even if a host uses `keyring` or `oo7`:

- resolved secrets should still enter runtime as wrapped secret types
- zeroization policy still matters

## 4. `ssh-agent-client-rs` shows that agent access deserves its own seam

`ssh-agent-client-rs 1.1.2` is not a huge ecosystem pillar, but it is a useful architectural signal.

It proves there is a distinct client-side protocol seam for SSH agent usage.

That matters because:

- agent-backed auth is different from stored key material
- forwarded agent is different from local agent access
- runtime should not pretend all auth material is "just a key"

So the healthiest abstraction is:

- `AgentProviderPort`
- maybe `AgentHandle` or `AgentReference`

not:

- stuffing agent socket path into generic config and hoping adapters sort it out

## 5. `ssh-key` is powerful, but the current freshest line is RC

`ssh-key 0.7.0-rc.9` is interesting because it covers:

- key formats
- `authorized_keys`
- `known_hosts`
- certificates
- signing formats

That makes it a very strong candidate for:

- parsing known-host policies
- handling key material formats
- keeping SSH-specific data typed

But again, important caution:

- the freshest line visible now is RC

So current best reading:

- strong typed-format helper
- very relevant for `KnownHostPolicy` and key parsing
- conservative default may require isolating it behind an internal adapter until line stabilizes

## 6. `openssh` and `russh` should consume credentials, not define credential truth

This follows naturally from the remote-runtime pass, but this pass sharpens it.

### `openssh`

Strong as:

- pragmatic remote adapter
- reuse of existing user SSH config and agent behavior

Weak as:

- source of truth about credential architecture

### `russh`

Strong as:

- full protocol ownership path
- richer agent-forwarding and auth modeling possibilities

But still:

- transport adapter should consume credential/agent capabilities
- not invent storage or secret policy on its own

🔥 Strong rule:

**transport adapters authenticate using typed credential capabilities provided by the runtime or host, they should not own credential truth themselves**

## 7. Forwarding should be opt-in and explicit

This is probably the most important product/security conclusion of the pass.

Agent forwarding is not just "another auth convenience".

It is:

- delegation of local authority into another route
- potentially high-risk ambient capability

So good architecture should make forwarding:

- explicit
- visible
- route-scoped
- deny-by-default unless product chooses otherwise

Not:

- "if an agent socket exists, let remote route use it"

## 8. Known-hosts policy also belongs here, not in random transport glue

Because `ssh-key` and related SSH format support live near this space, another boundary becomes clearer:

- host verification policy
- credential lookup policy
- agent forwarding policy

should live near each other conceptually, even if implemented by different adapters.

That suggests a healthy bounded context with concepts like:

- `KnownHostPolicy`
- `CredentialReference`
- `ForwardingPolicy`
- `AgentAccessPolicy`

## Practical verdict

If I were designing this layer right now:

### V1

- `secrecy + zeroize` remain mandatory baseline
- typed `CredentialProviderPort`
- typed `AgentProviderPort`
- forwarding explicit and deny-by-default
- `keyring` only as optional standalone-host leaf
- `oo7` only as watchlist/specialized Linux leaf
- `ssh-key` only behind internal typed-format helpers until stable line is more comfortable

### V2

- richer keychain/provider ecosystem
- known-host policy adapters
- agent forwarding audit trail
- route-specific credential capability negotiation

## Чего я бы избегал

- ❌ Mixing credentials into topology or generic config blobs
- ❌ Treating agent forwarding as ambient default behavior
- ❌ Letting transport adapters define credential storage policy
- ❌ Using OS keychain crates as core runtime truth
- ❌ Passing raw socket paths and secret strings through public APIs

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- credential architecture deserves its own bounded context
- secret material, credential references and agent capabilities must be separate concepts
- forwarding should be an explicit policy decision
- keychain/keyring support belongs in infrastructure leaves
- SSH transport crates should consume typed credential capabilities, not define them

## Sources

- [keyring crate](https://crates.io/crates/keyring)
- [open-source-cooperative/keyring-rs](https://github.com/open-source-cooperative/keyring-rs)
- [oo7 crate](https://crates.io/crates/oo7)
- [linux-credentials/oo7](https://github.com/linux-credentials/oo7)
- [secrecy crate](https://crates.io/crates/secrecy)
- [iqlusioninc/crates](https://github.com/iqlusioninc/crates)
- [zeroize crate](https://crates.io/crates/zeroize)
- [russh crate](https://crates.io/crates/russh)
- [openssh crate](https://crates.io/crates/openssh)
- [openssh-mux-client crate](https://crates.io/crates/openssh-mux-client)
- [ssh-key crate](https://crates.io/crates/ssh-key)
- [RustCrypto/SSH](https://github.com/RustCrypto/SSH)
- [ssh-agent-client-rs crate](https://crates.io/crates/ssh-agent-client-rs)
- [nresare/ssh-agent-client-rs](https://github.com/nresare/ssh-agent-client-rs)
