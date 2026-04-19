# Deep Dive - Streaming Protocol And Timeline Persistence

**Проверено**: 2026-04-19

## Зачем этот deep dive

Два проекта особенно полезны для понимания terminal feature не как "виджет с PTY", а как полноценной runtime-системы:

- `OpenCove` - потому что у них PTY streaming уже оформлен как нормальный протокол с handshake, ролями, replay и recovery semantics
- `Hermes IDE` - потому что у них execution intelligence и timeline уже живут отдельно от raw terminal log

Вместе они хорошо показывают, что для сильной terminal feature мало просто уметь:

- запустить PTY
- показать вывод
- сохранить scrollback

Нужны ещё:

- явный session streaming contract
- explicit recovery path при replay overflow
- separate read models for raw log, structured execution timeline и context snapshots

## Primary Sources

### OpenCove

- [`controlSurfaceHttpServer.sessionStreaming.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/contract/controlSurface/controlSurfaceHttpServer.sessionStreaming.spec.ts)
- [`controlSurfaceHttpServer.multiEndpoint.ptyProxy.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/contract/controlSurface/controlSurfaceHttpServer.multiEndpoint.ptyProxy.spec.ts)
- [`controlSurfaceHttpServer.syncWriteState.spec.ts`](https://github.com/DeadWaveWave/opencove/blob/main/tests/contract/controlSurface/controlSurfaceHttpServer.syncWriteState.spec.ts)
- [`ptyStreamHub.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/ptyStream/ptyStreamHub.ts)
- [`ptyStreamWire.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/ptyStream/ptyStreamWire.ts)
- [`remotePtyStreamMessageHandler.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/remote/remotePtyStreamMessageHandler.ts)
- [`ptyStreamTypes.ts`](https://github.com/DeadWaveWave/opencove/blob/main/src/app/main/controlSurface/ptyStream/ptyStreamTypes.ts)

### Hermes IDE

- [`src-tauri/src/db/mod.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/db/mod.rs)
- [`src-tauri/src/pty/commands.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/commands.rs)
- [`src-tauri/src/pty/models.rs`](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/models.rs)
- [`src/components/TerminalPane.tsx`](https://github.com/hermes-hq/hermes-ide/blob/main/src/components/TerminalPane.tsx)

## OpenCove - Streaming Protocol Is A Product Contract

### Handshake is mandatory, not optional sugar

В `OpenCove` WebSocket stream требует:

- subprotocol `opencove-pty.v1`
- initial `hello`
- protocol version match

Контракт тестами подтверждён явно:

- без subprotocol соединение отвергается
- `attach` до `hello` даёт `protocol.expected_hello`
- mismatch версии даёт `protocol.version_mismatch`

Это сильный signal: terminal streaming protocol стоит проектировать как отдельный contract с versioning, а не как "просто websocket с JSON".

### Roles are first-class product semantics

`ptyStreamTypes.ts` вводит:

- `PtyStreamClientKind = 'web' | 'desktop' | 'cli' | 'unknown'`
- `PtyStreamRole = 'viewer' | 'controller'`

А `sessionStreaming.spec.ts` показывает, что эти роли реально используются как business semantics:

- первый attach с `role: controller` получает control
- второй attach, даже если просит `controller`, может быть downgraded до `viewer`
- `request_control` переводит роли через явный `control_changed`

🔥 Это важный product pattern.  
Attach role - это не транспортная мелочь, а часть UX и прав записи.

### Replay window and overflow are explicit

`PtyStreamHub` держит per-session state:

- `seq`
- replay `chunks`
- `totalBytes`
- `truncated`
- `subscribers`
- `controllerClientId`

Полезные детали:

- `replayWindowMaxBytes` имеет floor `64_000`
- batching вывода идёт с `PTY_DATA_FLUSH_DELAY_MS = 32`
- upper batch size `PTY_DATA_MAX_BATCH_CHARS = 256_000`

Когда replay window переполнен:

- старые chunks выбрасываются
- `truncated = true`
- клиенту шлётся `overflow`
- `overflow` уже содержит `reason: 'replay_window_exceeded'`
- и главное - `recovery: 'snapshot'`

Это отличный pattern: overflow не должен оставаться "неопределённой деградацией".  
Он должен явно переводить клиента в другой recovery path.

### Snapshot resync is part of the protocol

`remotePtyStreamMessageHandler.ts` показывает очень зрелую реакцию на overflow:

- runtime remembers `lastSeq` per attached session
- при `overflow` он не пытается гадать, что потерялось
- он вызывает `snapshot(sessionId)`
- и публикует recovered snapshot дальше как terminal data

То есть recovery уже встроен в message semantics:

- normal path - incremental replay by `seq`
- degraded path - full snapshot resync

⚠️ Это сильно лучше, чем молча дропать scrollback или пытаться "доиграть" неизвестную дыру.

### Slow clients can be kicked out

`ptyStreamWire.ts` вводит `WS_BACKPRESSURE_CLOSE_THRESHOLD_BYTES = 8_000_000`.

Если `ws.bufferedAmount` превышает порог:

- сервер закрывает клиент с кодом `1013`
- reason - `backpressure`

Это полезный reminder: backpressure - это не только batching.  
Иногда нормальная защита системы - явно выгнать unhealthy subscriber, а не позволять ему бесконечно раздувать память.

### Local and remote identities must stay separate

`controlSurfaceHttpServer.multiEndpoint.ptyProxy.spec.ts` показывает важный runtime shape:

- home worker регистрирует remote endpoint
- потом создаёт mount
- потом делает `pty.spawnInMount`
- home session id отличается от remote session id
- streaming, writes и exits проксируются между ними

🔥 Значит session identity нельзя сводить к одному голому `sessionId`.  
Нужны как минимум:

- local/home session identity
- remote/runtime binding identity
- route/topology metadata

### Sync writes need revision preconditions

`controlSurfaceHttpServer.syncWriteState.spec.ts` показывает ещё один зрелый pattern:

- первый `sync.writeState` проходит
- потом `sync.state` возвращает revision
- следующий write без `baseRevision` уже отвергается как `persistence.invalid_state`

Это очень полезная защита против тихого last-write-wins хаоса.  
Если terminal workspace будет иметь durable layout/session truth, там почти наверняка понадобятся revision-based writes или snapshot-aware merge.

## Hermes IDE - Timeline Must Be Separate From Raw Log

### There are already multiple persistence shapes

В `db/mod.rs` у Hermes есть не один execution store, а несколько:

- `execution_log`
- `execution_nodes`
- `context_snapshots`
- `command_patterns`

Это уже хороший architectural signal: один store не должен пытаться быть одновременно:

- raw transcript
- human-readable timeline
- context memory
- prediction memory

### Raw execution log is append-oriented and cheap

`execution_log` хранит:

- `session_id`
- `event_type`
- `content`
- `exit_code`
- `working_directory`
- `timestamp`

Это ближе к audit/event stream слою.  
Он дешёвый, простой и пригоден для generic inspection.

### Execution nodes are structured semantic timeline entries

`execution_nodes` уже ближе к product timeline:

- `session_id`
- `timestamp`
- `kind`
- `input`
- `output_summary`
- `exit_code`
- `working_dir`
- `duration_ms`
- `metadata`

А в `commands.rs` видно, как они появляются:

- analyzer завершает команду
- completed nodes забираются после release analyzer lock
- DB пишет `insert_execution_node(...)`
- затем runtime emits `execution-node-{sessionId}`

То есть structured timeline:

- строится из analyzer/runtime semantics
- пишется отдельно в DB
- и отдельно отправляется живым event-потоком

🔥 Это очень сильный rule: raw PTY output и semantic execution timeline нельзя смешивать в одну сущность.

### Prediction memory learns from completed commands, not raw keystrokes

После insert execution node Hermes:

- нормализует command input
- пушит его в recent command deque
- пишет bigram/trigram-like sequence в `command_patterns`
- делает `predict_next_command(...)`
- и эмитит `command-prediction-{sessionId}`

Это очень хороший pattern:

- prediction строится на completed execution semantics
- а не на сырых клавишах или transient prompt echo

Такой подход обычно стабильнее и меньше зависит от shell noise.

### Context snapshots are versioned and bounded

`context_snapshots` у Hermes хранят:

- `session_id`
- `version`
- `context_json`

Поведение:

- `INSERT OR REPLACE`
- на session хранится только последние `5` snapshots
- snapshot можно читать списком или по конкретной версии

⚠️ Это полезный pattern не только для AI context.  
Вообще любой semantic session cache лучше делать:

- versioned
- bounded
- читаемым отдельно от raw log

### Session phase still matters, but it is not the whole story

В `models.rs` session phase machine даёт:

- `ShellReady`
- `Idle`
- `Busy`
- `NeedsInput`
- и другие состояния

Но именно persistence-тракт у Hermes показывает важную границу:

- phase state отдельно
- raw log отдельно
- execution timeline отдельно
- context snapshots отдельно
- prediction memory отдельно

То есть phase - это runtime coordination truth, а не универсальный контейнер для всех user-facing данных сессии.

## Cross-Project Synthesis

### 1. Streaming protocol is not persistence

`OpenCove` показывает, как делать live stream:

- handshake
- roles
- seq
- replay
- overflow
- snapshot resync

Но это не заменяет durable timeline или semantic memory.

### 2. Raw log is not the same as execution timeline

`Hermes` прямо подтверждает, что полезно держать отдельно:

- raw append log
- structured execution summaries
- prediction memory
- context snapshots

### 3. Recovery should advertise its fallback explicitly

Очень сильный pattern из `OpenCove`:

- client не должен сам догадываться, что replay сломан
- server должен явно сказать, что дальше нужен `snapshot` recovery path

### 4. Controller/viewer is a product concept

Если terminal будет attachable из разных surfaces:

- main UI
- remote shell
- automation
- read-only preview

то role semantics лучше закладывать рано, а не пытаться прикрутить потом поверх raw socket attach.

### 5. Revision guards beat accidental last-write-wins

Как только появляется:

- workspace state
- layout state
- session metadata
- control-center mutations

стоит заранее думать про:

- `baseRevision`
- snapshot-aware merge
- explicit conflict semantics

### 6. Bounded semantic caches are healthier than giant truth blobs

`Hermes` с `context_snapshots` и `OpenCove` с bounded replay window оба подталкивают к одному и тому же выводу:

- хранить всё подряд "навсегда" в одном truth blob - плохая стратегия
- лучше иметь bounded, purpose-built stores с явной semantics

## Что это значит для нашей terminal feature

Если переносить эти идеи в нашу архитектуру, то minimum shape выглядит так:

1. `session streaming contract`
   - versioned handshake
   - attach roles
   - seq-based replay
   - explicit overflow + snapshot recovery

2. `durable runtime stores`
   - raw output mirror
   - semantic execution timeline
   - bounded context/session snapshots

3. `sync truth contract`
   - revisioned layout/session writes
   - conflict-safe restore/update semantics

4. `session identity model`
   - local session id
   - route/runtime binding
   - controller/viewer ownership

## Short Architectural Verdict

🔥 Самый ценный вывод этого deep dive:

Сильная terminal feature - это не "renderer + PTY + scrollback".  
Это как минимум три разных класса контрактов:

- live streaming contract
- durable persistence contract
- semantic timeline/intelligence contract

Если их смешать рано, дальше почти неизбежно получится хрупкий terminal subsystem.
