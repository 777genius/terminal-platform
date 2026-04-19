# Deep Dive: Control Surface And Runtime Topology

**Проверено**: 2026-04-19

Этот файл про слой, который особенно легко недооценить:

- кто владеет session records
- как выглядит PTY stream protocol
- как local/remote runtime abstractions соединяются с UI
- где появляется controller/viewer semantics
- как topology, mounts и credentials живут рядом с terminal workflows

Это уже не просто terminal engine research.  
Это почти blueprint для terminal feature, если она должна жить как нормальная subsystem.

## 1. `OpenCove` - control surface is already a platform, not a helper

### Какие исходники особенно полезны

- `src/app/main/controlSurface/handlers/sessionRecords.ts`
- `src/app/main/controlSurface/topology/topologyStore.ts`
- `src/app/main/controlSurface/ptyStream/ptyStreamState.ts`
- `src/app/main/controlSurface/ptyStream/ptyStreamHub.ts`
- `src/app/main/controlSurface/remote/remotePtyRuntime.ts`
- `src/app/main/controlSurface/http/webSessionManager.ts`

### Что стало понятнее

#### A. Session record stores route, not only ID

`sessionRecords.ts` intentionally stores:

- normal session data
- `startedAtMs`
- `route`
  - `local`
  - `remote { endpointId, remoteSessionId }`

🔥 Это очень сильная idea.

If a session can move across local/remote boundaries, the route is part of its durable runtime identity.

#### B. Topology store separates durable topology from secrets

`topologyStore.ts` keeps:

- topology file with endpoints/mounts
- secrets file with tokens
- queued persistence writes
- endpoint registration and mount promotion/removal

Это хороший pattern:

- remote endpoint metadata and filesystem mounts are durable facts
- credentials should not live mixed into the same general-purpose topology blob

#### C. PTY stream hub has explicit replay window and roles

`ptyStreamHub.ts` shows a very useful shape:

- session state tracks `seq`, `chunks`, `totalBytes`, `truncated`
- pending PTY chunks are batched and flushed on timer/size threshold
- clients subscribe per session
- one subscriber can be `controller`
- others are `viewer`

This is stronger than "broadcast terminal output to everyone".

Useful lessons:

- replay should be sequence-based
- scrollback replay window should be bounded
- multi-client attach needs explicit role semantics

#### D. Remote PTY runtime auto-reattaches by intent, not by accident

`remotePtyRuntime.ts` keeps:

- subscribers by session and webContents
- `attachedSessions` with `lastSeq`
- `rolePreferenceBySessionId`
- WebSocket handshake with protocol version + auth bearer token
- reattach on reconnect with `afterSeq`

🔥 Very strong lesson:

remote terminal reconnect should be modeled as:

- handshake
- attach intent
- replay from last acknowledged sequence
- role reassertion

not as "reopen socket and hope state lines up".

#### E. Web session management is separate and explicit

`webSessionManager.ts` has:

- short-lived auth tickets
- longer-lived cookie sessions
- redirect sanitization
- explicit cleanup

This is not directly terminal logic, but it reinforces a bigger OpenCove pattern:

control surface is treated as a real entry platform with auth/session semantics, not just app internals.

### Что утащить как идею

- session route as part of session identity
- topology store + separate credential store
- seq-based PTY replay protocol
- controller/viewer attach roles
- reconnect via explicit attach intent and replay offset

---

## 2. `Mux` - runtime abstraction stays minimal and low-level

### Какие исходники особенно полезны

- `src/node/runtime/Runtime.ts`
- `src/node/runtime/runtimeFactory.ts`
- `src/node/services/desktop/DesktopSessionManager.ts`
- `src/node/services/desktop/PortableDesktopSession.ts`

### Что стало понятнее

#### A. Runtime interface is intentionally primitive

`Runtime.ts` is a strong example of a good boundary:

- streaming exec primitives
- background process handle
- workspace create/init/fork operations
- file stat / filesystem helpers
- runtime availability checks

⚠️ Important point:

it does **not** try to expose a magical high-level API for every workflow.  
It stays low-level enough that shared helpers can live above it.

#### B. Workspace creation and workspace init are separate phases

`runtimeFactory.ts` and related runtime interfaces keep separate concepts:

- create workspace
- init workspace
- optional `postCreateSetup`
- run background init with standardized logging

This is a useful pattern for terminal-adjacent workstreams too:

- create durable workspace/session identity
- then provision/runtime-init
- then attach UI

#### C. Runtime mode mapping is centralized

`runtimeFactory.ts` centralizes:

- runtime type selection
- backward compatibility mapping
- availability checks
- coder/docker/devcontainer/ssh special cases

This reduces runtime-specific chaos leaking into higher layers.

#### D. Portable desktop sessions are supervised separately

`DesktopSessionManager.ts` and `PortableDesktopSession.ts` show:

- per-workspace desktop session startup promises
- capability gating by runtime mode
- separate external binary lifecycle
- state file protocol for actions/screenshots

Even though this is not terminal-only, it reinforces an important subsystem rule:

external long-lived helper runtimes should be supervised by dedicated managers, not ad-hoc from UI flows.

### Что утащить как идею

- keep runtime interface small and streaming-first
- separate create vs init phases
- centralize runtime selection/compatibility
- supervise helper runtimes with explicit managers

---

## 3. `cmux` - remote daemon should stay out of the keystroke hot path

### Какие материалы особенно полезны

- `daemon/remote/README.md`
- `tests_v2/test_ssh_remote_resize_scrollback_regression.py`

### Что стало понятнее

#### A. Remote daemon is not the keystroke hot path

`cmuxd-remote` README explicitly says:

- bootstrap
- capability negotiation
- proxy RPC
- session attach/resize/status
- CLI relay

but **not** terminal keystroke hot path.

🔥 This is a very important architecture constraint.

Remote/session daemons should coordinate and broker, but hot terminal interaction paths should stay lean.

#### B. Reverse relay + authenticated local bridge

The relay model includes:

- reverse SSH forward
- local authenticated relay
- HMAC challenge-response
- session-specific relay metadata files

This is a good reference for secure remote control surfaces.

#### C. Resize churn regression is treated as a real contract

`test_ssh_remote_resize_scrollback_regression.py` is especially instructive:

- large remote scrollback is generated
- panes are resized repeatedly
- test asserts that scrollback content survives the churn

This is a strong practical reminder:

resize semantics are not cosmetic.  
Bad resize handling can destroy real user history.

### Что утащить как идею

- remote daemon coordinates, not owns hot path rendering/input
- secure relay/bridge pattern for remote control
- treat resize + scrollback preservation as a regression contract

---

## Synthesis

После этого deep dive runtime topology looks like this:

### 1. Session identity should include route and role

Not just `sessionId`, but potentially:

- route kind
- remote endpoint binding
- attach role
- replay position

### 2. PTY streaming should be protocolized

Prefer:

- seq numbers
- bounded replay windows
- truncation flags
- explicit controller/viewer roles

### 3. Runtime abstraction should stay small

Do not pack all product workflows into the runtime interface.  
Keep runtime low-level and let higher layers compose use cases.

### 4. Remote coordination should stay off the hot path

Brokering, auth, attach/reconnect and proxying are one thing.  
Per-keystroke fast paths are another.

### 5. Topology and credentials should not be mixed carelessly

Remote endpoint metadata, mounts and secrets are related, but they are not the same type of truth.

## Sources

- [OpenCove](https://github.com/DeadWaveWave/opencove)
- [Mux](https://github.com/coder/mux)
- [cmuxd-remote](https://github.com/manaflow-ai/cmux/tree/main/daemon/remote)
