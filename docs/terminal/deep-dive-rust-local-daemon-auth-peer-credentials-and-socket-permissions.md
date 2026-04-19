# Deep Dive - Rust Local Daemon Auth, Peer Credentials, and Socket Permission Boundaries

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для reusable terminal runtime local daemon almost always sounds harmless:

- "это же только локальный сокет"
- "это же не network service"
- "UI и daemon всё равно на одной машине"

🔥 На практике это один из самых опасных self-deception layers.

Если этот пакет должен встраиваться:

- в Electron app
- в standalone terminal app
- в другие host apps
- с attach/detach, multi-client и long-lived sessions

то local daemon boundary быстро становится security boundary:

- кто может attach к session
- кто может прочитать snapshot или transcript
- кто может послать control command
- можно ли доверять path/name alone
- как жить с Unix peer creds, Windows named pipe ACLs и session scoping

Это уже не просто transport convenience. Это explicit authority seam.

## Primary Sources

### Cross-platform local transport baseline

- [`interprocess` crate](https://crates.io/crates/interprocess)
- [`interprocess` repo](https://github.com/kotauskas/interprocess)

### Unix and low-level OS leaves

- [`rustix` crate](https://crates.io/crates/rustix)
- [`rustix` repo](https://github.com/bytecodealliance/rustix)
- [`nix` crate](https://crates.io/crates/nix)
- [`nix` repo](https://github.com/nix-rust/nix)
- [`uds` crate](https://crates.io/crates/uds)
- [`uds` repo](https://github.com/tormol/uds)
- [`unix-cred` crate](https://crates.io/crates/unix-cred)
- [`unix-cred` repo](https://github.com/cptpcrd/unix-cred-rs)

### Donor and adjacent references

- [`gips` crate](https://crates.io/crates/gips)
- [`gips` repo](https://github.com/funelk/gips)

### OS-level reference docs

- [`unix(7)` on man7.org](https://man7.org/linux/man-pages/man7/unix.7.html)
- [`Named Pipe Security and Access Rights` on Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights)
- [`Impersonation Levels` on Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/secauthz/impersonation-levels)

## Freshness signals

- `interprocess 2.4.1` - repo `kotauskas/interprocess`, `551` stars, pushed `2026-04-18`
- `rustix 1.1.4` - repo `bytecodealliance/rustix`, `1966` stars, pushed `2026-04-18`
- `nix 0.31.2` - repo `nix-rust/nix`, `3019` stars, pushed `2026-04-18`
- `uds 0.4.2` - repo `tormol/uds`, `23` stars, pushed `2026-03-27`
- `unix-cred 0.1.1` - repo `cptpcrd/unix-cred-rs`, latest crate published `2021-01-03`, repo updated `2021-10-23`
- `gips 0.2.0` - repo `funelk/gips`, latest crate published `2025-12-06`, repo updated `2026-01-19`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**local daemon auth must be policy-first, not path-first**

Healthiest shape сейчас выглядит так:

1. one host-neutral local transport abstraction
2. per-platform auth leaves
3. explicit peer identity check on accept/attach
4. runtime directory / pipe namespace hygiene as part of the contract
5. authority derived from verified peer identity and role policy, not from endpoint name alone

## Top 3 directions for this layer

### 1. `Per-user runtime endpoint + explicit peer verification + OS-specific auth leaves`

`🎯 10   🛡️ 9   🧠 8`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- public transport abstraction stays simple and cross-platform
- Unix uses pathname sockets inside a controlled runtime directory
- peer identity is explicitly verified on accept
- Windows named pipes get explicit security descriptors and session scoping
- attach rights are checked by runtime policy, not by path guessing

Почему это лучший путь:

- security boundary stays deliberate
- portable story remains sane
- fits protocol-first daemon architecture
- scales to multi-host and multi-surface embedding without ambient trust

### 2. `Same baseline + Unix advanced leaves for seqpacket, fd-passing and richer peer metadata`

`🎯 8   🛡️ 8   🧠 9`
Примерно `8000-16000` строк.

Это strong advanced path if the product later needs:

- fd-passed artifact handles
- seqpacket semantics
- Unix-only richer local control lanes

Здесь полезны:

- `uds`
- `nix`
- `rustix`

Но это должен быть explicit Unix acceleration leaf, not the universal host contract.

### 3. `Ambient trust by endpoint name, abstract sockets or default pipe security`

`🎯 3   🛡️ 3   🧠 4`
Примерно `3000-7000` строк на старт и потом дорого чинить.

Это плохой default.

Типичные smell-ы:

- trust the socket path because "it is local"
- use Linux abstract sockets and assume they are secure
- create Windows named pipes with default security
- skip peer verification because the daemon is "single-user anyway"

Это как раз тот путь, который делает local daemon accidentally permissive.

## 1. `interprocess` is the right transport baseline, but not the whole auth story

`interprocess` remains the healthiest cross-platform baseline for local daemon transport.

Почему:

- cross-platform local sockets model
- Unix and Windows shape under one conceptual seam
- strong fit for local control planes
- explicit platform-specific extensions philosophy

Но очень важно не переоценить его роль.

`interprocess` solves mainly:

- transport abstraction
- platform mapping
- async integration

It does **not** magically define:

- attach authority
- peer identity policy
- session scoping
- per-user isolation policy

🔥 Strong rule:

**transport choice is not authorization policy**

## 2. Unix pathname sockets and abstract sockets have very different security meaning

This pass made one Linux rule much clearer.

From `unix(7)`:

- pathname sockets honor directory permissions on creation
- Linux also checks write permission on the socket object for connect/send
- but portable programs should not rely on socket-file permissions as security
- abstract sockets ignore permissions entirely
- abstract namespace is Linux-only and nonportable

That leads to a very practical verdict:

### Healthy default

- pathname socket in a private runtime directory
- explicit directory ownership and mode
- explicit peer credential verification

### Unhealthy default

- abstract namespace socket as the main auth boundary

🔥 Abstract sockets are a convenience feature, not a reliable security boundary.

## 3. Peer credentials should be first-class on Unix

This layer became much clearer after checking `nix`, `rustix`, `unix-cred` and the Linux man page.

### `rustix`

Very important signal:

- `rustix::net::sockopt::socket_peercred` exists
- `rustix` treats this as a low-level safe syscall leaf

That is exactly the right role:

- modern low-level Unix leaf
- safe typed syscall wrapper
- no fake cross-platform abstraction pretending the feature exists everywhere

### `nix`

Also very strong:

- `PeerCredentials` for Linux/Android `SO_PEERCRED`
- `LocalPeerCred` for Apple targets
- `UnixCredentials` as typed structure

This is useful proof that:

- peer identity must remain platform-shaped
- a runtime-level auth adapter can still expose one internal semantic concept

### `unix-cred`

Useful, but smaller and older.

It is attractive because:

- it focuses exactly on peer IDs
- it covers several Unix families

But it is too small and too stale to be my main foundation recommendation for a package of this ambition.

Healthy role:

- donor
- optional helper
- maybe test/reference utility

## 4. `uds` is a strong Unix-only capability leaf

`uds` turned out to be especially instructive.

It explicitly gives:

- abstract addresses
- fd passing
- `SOCK_SEQPACKET`
- `initial_peer_credentials()`

And it also explicitly says:

- ancillary credentials are not yet supported
- macOS lacks some features
- portability differs across BSD/Linux

🔥 That honesty is valuable.

Architectural verdict:

- `uds` is an excellent Unix specialization leaf
- it is not the universal daemon transport center

This fits the package direction very well:

- universal host-neutral transport contract first
- richer Unix-only leaves behind capability gates

## 5. Windows named pipes need explicit security policy, not default trust

The Microsoft docs are the strongest reminder here.

Two practical findings matter a lot:

### Named pipe security descriptors

Microsoft states that:

- you can specify a security descriptor at `CreateNamedPipe`
- if you pass `NULL`, a default security descriptor is used
- that default grants full control to LocalSystem, administrators and creator owner
- and grants read access to Everyone and the anonymous account

🔥 This means:

**default named pipe security is not a safe architectural assumption for a serious embeddable runtime**

### Impersonation levels

Microsoft also documents that named pipe clients control impersonation level, and the default level for named pipe, RPC and DDE servers is `SecurityImpersonation`.

That implies:

- Windows local daemon attach semantics are not just "connect or not"
- client/server identity and impersonation policy must be explicit
- runtime should not let pipe creation defaults define authority model accidentally

### Practical Windows verdict

- use explicit security descriptors
- scope to the intended user/session
- keep Windows auth and impersonation logic in a dedicated adapter leaf

## 6. Logon/session scoping matters

Another practical Microsoft point:

- to prevent remote users or users in a different terminal services session from accessing a named pipe, use the logon SID on the DACL

This is exactly the kind of detail that should become product policy, not app-specific folklore.

For the terminal package that means:

- "same machine" is not enough
- "same username" may not be enough
- runtime may need an explicit per-logon-session or per-app-instance scoping policy

## 7. What to do in the package architecture

If we compress this whole pass into practical rules:

### Public shape

- one local control-plane transport abstraction
- explicit attach/open/subscribe commands
- no host-visible raw Unix or Win32 auth plumbing

### Unix leaves

- pathname socket in private runtime dir by default
- peer creds checked on accept
- optional Unix-only extras like fd-passing and seqpacket behind leaf adapters

### Windows leaves

- named pipes stay behind the same host-neutral transport seam
- explicit security descriptor policy
- explicit session/logon scoping
- impersonation details hidden in adapter layer

### Runtime policy

- authority comes from verified peer identity plus role policy
- not from endpoint name
- not from "local means trusted"

## Final verdict

🔥 The healthy question is not:

- "which local IPC primitive do we use?"

The healthy question is:

- "how do we prove who is connecting, and what are they allowed to do once connected?"

For your terminal package the strongest answer right now is:

- `interprocess` as the transport baseline
- `rustix`/`nix` as Unix auth leaves
- pathname sockets in controlled runtime dirs, not abstract namespace by default
- explicit Windows pipe security policy, not defaults
- peer verification and attach policy as first-class runtime concerns
