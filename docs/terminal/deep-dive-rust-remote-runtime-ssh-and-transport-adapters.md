# Deep Dive - Rust Remote Runtime, SSH, and Transport Adapters

**Проверено**: 2026-04-19

## Зачем смотреть этот слой отдельно

Для universal terminal package remote story нельзя прикручивать потом как пару `ssh` вызовов.

Если сделать это неаккуратно, весь runtime быстро протечёт transport-деталями:

- локальная session truth смешается с remote transport state
- host API начнёт знать про `ssh2::Channel` или другой backend-specific handle
- restore, attach/detach и replay станут зависеть от конкретного SSH crate
- local и remote session paths начнут расходиться по semantics

Правильный вопрос здесь не "какой SSH crate круче", а:

- где вообще должен жить remote layer
- какой crate даёт правильные seams
- что можно брать как dependency, а что только как donor architecture

## Топ 3

### 1. `RemoteRoutePort` с `wezterm-ssh`-style adapter island
`🎯 9   🛡️ 8   🧠 8`  
Примерно `6000-12000` строк.

Что это значит:

- remote остаётся отдельным route в runtime model
- adapter внутри Rust скрывает `libssh-rs` / `ssh2`
- наружу пакет всё равно отдаёт тот же `MasterPty/Child`-like seam или тот же protocol projection, что и для local runtime

Почему это лучший shape:

- `wezterm-ssh 0.4.0` уже показывает очень полезный pattern
- crate реэкспортит `portable_pty::{Child, ChildKiller, MasterPty, PtySize}`
- внутри у него уже есть `request_pty`, `resize`, `exec`, `shell`, `signal`, `sftp`
- backend выбирается конфигом, а не меняет внешний surface

Почему я бы был осторожен с прямой зависимостью:

- опубликованный crate староват по dependency baseline
- в его `Cargo.toml` ещё видны `portable-pty 0.7`, `libssh-rs 0.1.4`, `ssh2 0.9.3`, `smol 1.2`
- как donor и reference это очень сильная штука
- как жёсткий foundation dependency для нового world-class package я бы его сначала очень внимательно изолировал или форкнул

### 2. `openssh` как pragmatic system adapter
`🎯 8   🛡️ 9   🧠 4`  
Примерно `3000-7000` строк.

Что это значит:

- remote terminal runtime не тащит свою SSH protocol implementation
- вы используете системный `ssh` как внешний adapter
- runtime sees only process/stream/control semantics

Почему это сильно:

- `openssh 0.11.6` очень зрелый по идее API
- reuse existing `.ssh/config`, `ControlMaster`, agent, host config
- `native-mux` и `process-mux` дают понятную connection model
- repo активен, обновлялся 2026-04-05

Почему это не лучший universal core:

- crate прямо пишет, что работает только на Unix
- поддерживает только password-less auth
- remote child по факту это локальный `ssh` process handle, а не полноценная terminal runtime truth
- error fidelity хуже, чем у native SSH implementation
- для interactive PTY terminal platform это скорее outer adapter для `exec/subsystem`, чем настоящий foundation

### 3. `russh` как pure-Rust protocol ownership path
`🎯 8   🛡️ 8   🧠 9`  
Примерно `8000-16000` строк.

Что это значит:

- весь SSH protocol stack у вас в Rust
- можно одинаково моделировать client, server, forwarding, PTY channels и bastion/proxy cases
- transport глубже интегрируется с runtime

Почему это сильно:

- `russh 0.60.0` свежий и активно развивается
- repo активен, обновлялся 2026-04-17
- есть client/server, PTY example, port forwarding, agent forwarding, SFTP ecosystem
- каналы умеют `AsyncRead`/`AsyncWrite`

Почему это дороже:

- event loop и channel orchestration становятся вашей ответственностью
- interactive example уже показывает более ручной lifecycle
- SSH protocol не даёт argv semantics, так что quoting/escaping приходится решать отдельно
- это очень сильный путь, но больше похож на deliberate product bet, чем на cheap v1

## Самый важный вывод

🔥 Для такого пакета **remote не должен быть отдельным специальным терминалом**.  
Он должен быть просто ещё одним `runtime route`:

- `local`
- `remote-ssh`
- позже, возможно, `remote-daemon`
- позже, возможно, `sandbox`

Но сверху host и UI всё ещё должны видеть один и тот же stable contract.

## Что показал `wezterm-ssh`

### Почему проект очень полезен

`wezterm-ssh 0.4.0` оказался самым интересным donor не потому, что он "лучшая SSH библиотека", а потому что он уже показывает хороший architecture shape:

- backend abstraction через `SessionWrap`
- channel abstraction через `ChannelWrap`
- remote PTY завернут в `portable-pty`-совместимую модель
- `Session::request_pty(...)` возвращает `(SshPty, SshChildProcess)`
- resize, exec, signal, sftp проходят через одинаковый request loop

Особенно полезно:

- backend выбирается значением `wezterm_ssh_backend`
- внутри уже разведены `libssh-rs` и `ssh2`
- наружу не вытекают их raw channel refs

### Что в нём настораживает

- published dependency baseline заметно старее сегодняшнего Rust terminal stack
- внутри много `smol`-specific и thread-based решений
- это хороший donor для adapter island
- это не готовый источник истины для нового standalone runtime package

## Что показал `openssh`

### Сильные стороны

`openssh 0.11.6` очень убедительно показывает, как хорошо выглядит **outer adapter**:

- reuse user SSH config
- reuse ControlMaster multiplexing
- explicit `native-mux` vs `process-mux`
- API shape около `std::process::Command`

И это правда очень удобно для:

- remote exec
- remote subsystem
- scripted operations
- diagnostics tooling

### Ограничения

Но у crate есть жёсткие границы, которые для terminal platform важно признать заранее:

- Unix only
- password-less auth only
- max multiplexed sessions by default tied to sshd `MaxSessions`
- remote child lifecycle завязан на локальный `ssh` process

Поэтому `openssh` хорошо годится как:

- outer admin adapter
- migration bridge
- pragmatic remote exec path

И плохо годится как:

- primary remote terminal truth
- unified session-runtime core

## Что показал `russh`

### Почему он реально силён

`russh 0.60.0` после исходников и README выглядит не как niche crate, а как серьёзный SSH stack:

- client + server
- PTY example
- port forwarding
- OpenSSH certs
- agent forwarding
- SFTP ecosystem
- crypto backend selection

Это уже достаточно, чтобы строить:

- remote terminal route
- SSH proxy/bastion
- terminal-aware gateway
- server-side terminal hosts

### Почему это expensive path

Его interactive example очень хорошо показывает цену:

- raw mode
- manual stdin/stdout wiring
- channel wait loop
- explicit PTY request
- explicit EOF lifecycle

Это не недостаток crate. Это честный signal, что pure-Rust protocol ownership даёт силу, но требует больше runtime engineering.

## Проверенные дополнительные crates

### `ssh2`

- Latest checked: `0.9.5`
- Repo: `alexcrichton/ssh2-rs`
- Updated: `2026-04-15`

Полезные факты:

- low-level binding to `libssh2`
- умеет `request_pty`, `request_pty_size`, `exec`, `shell`
- хороший backend brick
- плохой public architecture boundary

### `libssh-rs`

- Latest checked: `0.3.6`
- Repo: `wez/libssh-rs`
- Updated: `2025-12-29`

Полезные факты:

- safe bindings around `libssh`
- умеет vendored build
- README отдельно поясняет LGPL nuance для vendored `libssh`
- хороший backend option внутри adapter island

### `async-ssh2-lite`

- Latest checked: `0.5.0`
- Repo: `bk-rs/ssh-rs`
- Updated: `2025-07-16`

Полезные факты:

- даёт async wrapper around `ssh2`
- полезен как helper crate
- недостаточно силён как main architectural foundation для universal terminal package

## Practical verdict

Если выбирать прямо сейчас, я бы делал так:

### V1

- local daemon/runtime остаётся primary truth
- remote route делается optional feature
- shape route adapter брать по мотивам `wezterm-ssh`
- для fastest pragmatic path можно иметь отдельный `openssh` adapter для exec/subsystem flows

### V2

- если remote становится first-class product feature, тогда либо:
  - усиливать adapter island вокруг `libssh-rs/ssh2`
  - либо делать осознанную ставку на `russh`

## Чего я бы избегал

- ❌ Делать SSH backend частью domain truth
- ❌ Светить host-ам `ssh2::Channel` или `libssh_rs::Channel`
- ❌ Склеивать local and remote session semantics только потому, что обе "терминалы"
- ❌ Выбирать `openssh` как единственный universal remote answer
- ❌ Сразу тащить pure-Rust SSH stack в центр v1, если remote ещё не доказал свою продуктовую ценность

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- remote route должен быть explicit property session/workstream identity
- local daemon protocol остаётся primary contract
- SSH adapters должны жить на outer infrastructure ring
- host projections, replay, restore и screen deltas не должны знать, local это или remote
- backend-specific channels, sessions и sockets должны быть private

## Sources

- [openssh crate](https://crates.io/crates/openssh)
- [openssh repo](https://github.com/openssh-rust/openssh)
- [russh crate](https://crates.io/crates/russh)
- [russh repo](https://github.com/Eugeny/russh)
- [ssh2 crate](https://crates.io/crates/ssh2)
- [ssh2-rs repo](https://github.com/alexcrichton/ssh2-rs)
- [wezterm-ssh crate](https://crates.io/crates/wezterm-ssh)
- [wezterm repo](https://github.com/wez/wezterm)
- [libssh-rs crate](https://crates.io/crates/libssh-rs)
- [libssh-rs repo](https://github.com/wez/libssh-rs)
- [async-ssh2-lite crate](https://crates.io/crates/async-ssh2-lite)
- [ssh-rs repo](https://github.com/bk-rs/ssh-rs)
