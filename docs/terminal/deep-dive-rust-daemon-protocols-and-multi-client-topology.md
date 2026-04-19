# Deep Dive - Rust Daemon Protocols And Multi-Client Topology

**Проверено**: 2026-04-19

## Зачем этот слой важен

Если terminal package должен быть:

- embeddable into Electron
- usable from standalone app
- reusable from other languages
- capable of persistent sessions and attach/detach

тогда runtime boundary уже нельзя мыслить как:

- direct in-process Rust calls only
- one host binding as the source of truth
- one undifferentiated byte stream

Нужен отдельный daemon/control-surface слой, который чётко разделяет:

- lifecycle and control commands
- PTY byte/data plane
- session discovery and attach semantics
- optional remote or web-facing APIs

🔥 Именно тут определяется, будет ли пакет реально универсальным, или останется "Node binding с terminal логикой внутри".

## Primary Sources

### Core transport and framing layers

- [`interprocess` README](https://github.com/kotauskas/interprocess/blob/master/README.md)
- [`interprocess::local_socket::tokio`](https://docs.rs/interprocess/latest/interprocess/local_socket/tokio/index.html)
- [`bytes` README](https://github.com/tokio-rs/bytes/blob/master/README.md)
- [`tokio-util` docs](https://docs.rs/tokio-util/latest/tokio_util/)
- [`prost` README](https://github.com/tokio-rs/prost/blob/master/README.md)

### RPC frameworks

- [`jsonrpsee` README](https://github.com/paritytech/jsonrpsee/blob/master/README.md)
- [`tarpc` README](https://github.com/google/tarpc/blob/master/README.md)
- [`tonic` README](https://github.com/hyperium/tonic/blob/master/README.md)

### Donor products with daemon/session topology

- [`zinc` README](https://github.com/ComeBertrand/zinc/blob/main/README.md)
- [`gritty` README](https://github.com/chipturner/gritty/blob/main/README.md)
- [`missiond` README](https://github.com/rickyjim626/missiond/blob/main/README.md)

## Freshness signals

- `interprocess 2.4.1` - repo `kotauskas/interprocess`, `551` stars, pushed `2026-04-18`
- `bytes 1.11.1` - repo `tokio-rs/bytes`, `2205` stars, pushed `2026-02-04`
- `tokio-util 0.7.18`
- `serde_json 1.0.149`
- `prost 0.14.3` - repo `tokio-rs/prost`, `4665` stars, pushed `2026-03-02`
- `prost-reflect 0.16.3`
- `jsonrpsee 0.26.0` - repo `paritytech/jsonrpsee`, `830` stars, pushed `2026-04-18`
- `tarpc 0.37.0` - repo `google/tarpc`, `3685` stars, pushed `2026-03-25`
- `tonic 0.14.5` - repo `hyperium/tonic`, `11.9k` stars, pushed `2026-04-17`
- `zinc-proto 0.3.4`
- `gritty-cli 0.11.0` latest, repo `chipturner/gritty`, pushed `2026-04-18`
- `missiond-attach 0.1.0`, repo `rickyjim626/missiond`, pushed `2026-04-14`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**для universal terminal package primary contract должен быть local-first framed daemon protocol, а не RPC framework**

На сейчас healthiest shape выглядит так:

1. local daemon over socket-like IPC
2. framed control protocol
3. separate byte/data plane for PTY/replay/snapshots
4. explicit multi-client attach roles and session routing
5. optional outer facades like JSON-RPC, WebSocket or gRPC only above that

## Top 3 directions for daemon/control topology

### 1. `interprocess + tokio-util::codec + bytes + explicit protocol envelope`

`🎯 10   🛡️ 9   🧠 8`
Примерно `7000-14000` строк.

Это мой текущий **лучший default**.

Почему:

- `interprocess` gives a serious cross-platform local IPC base
- local sockets map naturally to Unix sockets and Windows named pipes
- `tokio-util::codec` gives framed transport shape
- `bytes` is the right hot-path buffer currency
- explicit envelope protocol keeps the package host-neutral

Это strongest path для:

- Electron host
- standalone desktop app
- daemon that survives UI restarts
- future adapters for other languages

### 2. `same local-first stack + prost schema hardening`

`🎯 8   🛡️ 9   🧠 8`
Примерно `9000-16000` строк.

Это сильный next step, когда protocol already stabilizes.

Почему:

- `prost` and optionally `prost-reflect` can harden a public schema story
- useful for generated bindings and long-term compatibility tooling
- better than inventing a bespoke binary schema too early

Почему не default v1:

- more build and schema discipline upfront
- `protoc` and codegen story adds product friction
- for early iterations explicit envelopes with `serde_json` or similar are easier to evolve

### 3. `jsonrpsee` or `tarpc` as outer service facade, not runtime core`

`🎯 6   🛡️ 7   🧠 7`
Примерно `5000-11000` строк.

Это useful, but not as the main truth.

Почему:

- `jsonrpsee` is good for external/web or remote-facing APIs
- `tarpc` is good for Rust-to-Rust service ergonomics
- `tonic` is strong for networked service products

Но для local terminal runtime core они слабее, потому что:

- they bias the model toward RPC rather than stream/session topology
- they tend to blur control plane and data plane unless you resist that explicitly
- they pull in assumptions that are heavier than a local daemon needs

## 1. `interprocess` looks even better after a deeper read

`interprocess` is one of the strongest fresh signals in this space.

Important qualities:

- explicit cross-platform local sockets
- Tokio integration behind a feature flag
- honest platform support policy
- namespaced/file-path socket abstraction
- direct fit for "host process talks to local runtime daemon"

Its README is especially valuable because it does **not** pretend all IPC is one thing.

That is a healthy sign for our architecture too.

🔥 Practical takeaway:

**local daemon transport should be local-socket-first, not HTTP-first**

## 2. `tokio-util::codec` and `bytes` remain the correct framing baseline

This pass reinforced an earlier conclusion.

For the core local daemon boundary, we want:

- `bytes` for owned/shared byte buffers
- `BytesMut` for framing and incremental assembly
- `tokio-util::codec` for deliberate framed transport

Why this matters:

- PTY control traffic is message-oriented
- PTY output chunks are stream-oriented
- replay and snapshot payloads can get large
- attach/detach and capabilities negotiation need explicit envelopes

So the correct architecture is not:

- `BufReader::read_line` protocol everywhere

but:

- framed messages for control
- dedicated chunk streaming for data

## 3. `serde_json` is still a strong early protocol format

`serde_json 1.0.149` still looks like the best early envelope format for the local control plane.

Why:

- easy to debug with host teams
- simple for cross-language adapters
- good enough for lifecycle/query/topology traffic

But only if used in the right place:

- control messages
- capabilities
- state snapshots of metadata
- attach negotiation

Not for:

- raw PTY bytes
- giant replay buffers
- heavy durable snapshot blobs

🔥 Practical rule:

**JSON is fine for control plane, not for the byte plane**

## 4. `prost` is useful when the protocol hardens, not before

This pass made the `prost` boundary even clearer.

Good uses:

- stable public contracts
- multi-language tooling
- stronger schema governance
- future external integrations

Why it is not the default first move:

- `prost` is passively maintained but still strong and widely used
- codegen and `protoc` setup add weight
- early daemon protocols usually change too often for proto-first ergonomics to pay off

So the healthier path is:

- explicit local envelope protocol first
- `prost` when the protocol is genuinely stabilizing

## 5. `jsonrpsee`, `tarpc`, and `tonic` each have a place, but not at the center

### `jsonrpsee`

Strong for:

- external APIs
- WebSocket subscriptions
- browser/devtool access
- optional management/control facade

Weak as the runtime center because:

- terminal sessions are not just RPC methods
- attach/replay/overflow semantics are more streaming than request/response

### `tarpc`

Strong for:

- internal Rust service ergonomics
- generated service traits
- cancellation and deadlines

Weak as the universal package boundary because:

- it is primarily Rust-to-Rust shaped
- schema-in-code is less friendly for truly polyglot reuse

### `tonic`

Strong for:

- distributed/networked service products
- remote fleet or cloud control plane
- mature service interoperability

Weak as the local daemon default because:

- HTTP/2 and gRPC are heavier than needed for local IPC
- terminal runtime still needs a separate byte/data plane anyway

🔥 Practical rule:

**RPC frameworks are outer facades, not inner runtime truth**

## 6. `zinc`, `gritty`, and `missiond` confirm the session-topology lessons

### `zinc`

Useful because it clearly models:

- background daemon independent of terminals
- attach/detach semantics
- state tracking per agent/session
- TUI as supervisor, not as owner of runtime truth

That is very close to what a modern embeddable terminal runtime needs.

### `gritty`

Very useful because it shows:

- Unix domain socket local-first topology
- reconnect and self-healing sessions
- no custom network protocol by default
- session names and reattach behavior as first-class UX

🔥 `gritty` is especially valuable as proof that **persistent shell UX can be built around local sockets without dragging in a giant RPC stack**.

### `missiond`

Valuable because it explicitly splits:

- daemon
- MCP layer
- WebSocket API
- PTY attach tool

That is a strong proof of the architectural rule:

- one runtime
- many outer surfaces

## 7. Recommended protocol shape now

At this point, the healthiest shape looks like:

### Control plane

- framed request/event envelopes
- lifecycle commands
- queries
- topology and capability negotiation
- attach/detach
- state watches

### Data plane

- PTY byte chunks
- replay windows
- overflow markers
- snapshot chunks
- optional binary payloads

### Optional facades

- JSON-RPC for dashboards/tools
- WebSocket for live board UI
- gRPC only if later building a real networked service product

## 8. Multi-client attach should be explicit, not accidental

This pass reinforced a key rule from earlier product research.

The daemon should explicitly model:

- controller vs viewer roles
- attach ownership
- session route
- replay starting point
- reconnect and overflow behavior

That should not be inferred ad hoc from "who connected first".

Useful donor signals:

- `zinc` style attach/detach
- `gritty` style reconnect/reattach
- `missiond` style dedicated attach tool plus separate API surfaces

## 9. What I would explicitly avoid

- ❌ HTTP-first daemon protocol for a local terminal runtime
- ❌ using one generic RPC framework as the only public contract
- ❌ mixing PTY bytes and control commands in one ad hoc message format
- ❌ making Electron binding the canonical API
- ❌ assuming local daemon and remote/web API have the same transport needs
- ❌ hiding attach roles and replay semantics behind implicit behavior

## Final recommendation

If building the daemon/control surface today, I would choose:

- transport: `interprocess`
- async framing: `tokio-util::codec`
- buffers: `bytes`
- early control encoding: explicit envelope types with `serde_json`
- future schema hardening: `prost` and maybe `prost-reflect`
- outer facades only when needed:
  - `jsonrpsee` for external/web management
  - `tarpc` for internal Rust service experiments
  - `tonic` only for later remote/network products

🔥 Most important practical takeaway:

**the runtime should own one local-first protocol, and every host or API surface should adapt to it**

That is the healthiest path if this package must work:

- inside Electron
- in a standalone app
- from other languages
- and eventually in remote or board-style environments

## Sources

- [interprocess](https://github.com/kotauskas/interprocess)
- [interprocess tokio local sockets](https://docs.rs/interprocess/latest/interprocess/local_socket/tokio/index.html)
- [bytes](https://github.com/tokio-rs/bytes)
- [tokio-util](https://docs.rs/tokio-util/latest/tokio_util/)
- [prost](https://github.com/tokio-rs/prost)
- [jsonrpsee](https://github.com/paritytech/jsonrpsee)
- [tarpc](https://github.com/google/tarpc)
- [tonic](https://github.com/hyperium/tonic)
- [zinc](https://github.com/ComeBertrand/zinc)
- [gritty](https://github.com/chipturner/gritty)
- [missiond](https://github.com/rickyjim626/missiond)
