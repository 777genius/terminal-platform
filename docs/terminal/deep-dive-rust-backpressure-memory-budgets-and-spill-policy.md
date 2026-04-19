# Deep Dive - Rust Backpressure, Memory Budgets, and Spill Policy

**Проверено**: 2026-04-19

## Зачем смотреть этот слой отдельно

Если terminal package должен быть мирового уровня, он обязан переживать:

- bursty PTY output
- runaway log streams
- giant test/build output
- long-lived sessions with large scrollback
- reattach/replay after UI disconnect

Именно тут чаще всего ломаются "обычные" встроенные терминалы:

- UI лагает или замерзает
- scrollback бесконтрольно жрёт память
- replay становится слишком дорогим
- resize и rehydrate теряют консистентность

Поэтому backpressure и memory policy должны быть не "оптимизациями потом", а отдельным архитектурным слоем.

## Топ 3

### 1. `Owner-task bounded lanes + explicit budgets + spill-to-disk`
`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Это strongest default.

Идея:

- каждая session/workstream имеет свой owner task
- все input/output/replay lanes bounded
- у каждой surface есть explicit memory budget
- hot replay queue маленькая и дешёвая
- всё, что должно жить долго, уходит в durable mirror/spill layer

Что сюда хорошо ложится:

- `tokio::sync::mpsc` и `Semaphore`
- `bytes 1.11.1`
- `ringbuf 0.4.8` для bounded hot replay
- `rusqlite`/`redb` для durable mirror
- `zstd`/`blake3` для compressed deduped blobs

Почему это лучший путь:

- backpressure живёт в ownership model, а не в случайных throttles
- memory growth becomes policy-driven
- replay/restore/separate UI attach can stay deterministic

### 2. `thingbuf` for allocation-reusing bounded hot lanes
`🎯 8   🛡️ 8   🧠 7`  
Примерно `5000-10000` строк.

`thingbuf 0.1.6` оказался очень интересным не как "магическая замена Tokio", а как strong donor для узких hot lanes:

- bounded MPSC
- allocation reuse
- static/no_std story
- async and blocking variants

Где он особенно интересен:

- log/event formatting lanes
- preallocated chunk pools
- internal worker handoff where allocation churn matters

Почему это не strongest universal default:

- весь runtime вокруг него строить не стоит
- owner model и session truth всё равно лучше держать на Tokio-first foundation

### 3. `Low-level bounded queue islands` через `ringbuf` и `crossbeam-queue`
`🎯 7   🛡️ 8   🧠 6`  
Примерно `4000-9000` строк.

Это хороший путь для very specific islands:

- `ringbuf` - SPSC hot replay buffer
- `crossbeam-queue::ArrayQueue` - bounded MPMC side lanes

Почему это полезно:

- fixed-capacity thinking
- no accidental unbounded growth
- explicit pressure points

Почему это не должно становиться центром:

- queue primitive не заменяет runtime policy
- без owner-task model легко получить много локально правильных, но глобально хрупких решений

## Самый важный вывод

🔥 Terminal runtime не спасается rate-limiter-ами.  
Он спасается:

- explicit ownership
- bounded queues
- memory budgets
- spill policy
- clear distinction between hot replay and durable history

## Что показал `alacritty_terminal`

После просмотра `alacritty_terminal 0.26.0` особенно важно следующее:

- grid уже мыслится как structure with visible lines, history, display offset and max scroll limit
- `update_history()` явно режет history size и clamp-ит display offset
- resize/reflow logic deeply tied to history semantics
- `grow_lines` и `shrink_lines` реально тащат содержимое из history и обратно

Это очень сильный сигнал:

⚠️ memory/scrollback policy нельзя держать полностью "снаружи" emulator core.  
Она влияет на сам смысл restore, resize и what the user sees.

## Что показал `thingbuf`

- Latest checked: `0.1.6`
- Repo: `hawkw/thingbuf`
- Updated: `2026-04-15`

Почему интересно:

- lock-free array-based bounded MPSC
- access to slots by reference
- allocation-reusing queue/channel design
- explicit bounded-by-construction mindset

Очень полезный практический вывод из README:

- thingbuf отлично подходит там, где capacity fixed and known
- но сам README честно предупреждает, что huge rarely-filled bounds - плохая идея

Это идеально совпадает с terminal runtime reality:

- hot lanes should be small and intentionally bounded
- giant "just in case" channel capacities are usually a smell

## Что показали `crossbeam-queue` и `ringbuf`

### `crossbeam-queue`

- Latest checked: `0.3.12`
- Repo: `crossbeam-rs/crossbeam`
- Updated: `2026-04-19`

Полезные сигналы:

- `ArrayQueue` gives bounded MPMC
- `SegQueue` gives unbounded segmented allocation

Практический вывод:

- `ArrayQueue` интересен как bounded side primitive
- `SegQueue` almost never should be terminal hot-path default for this package

### `ringbuf`

- Latest checked: `0.4.8`
- Repo: `agerasev/ringbuf`
- Updated: `2026-04-11`

Почему особенно полезен:

- explicit SPSC identity
- direct access to inner data
- very natural fit for hot replay tail or serialization staging

Это хороший строительный блок для:

- last-N replay bytes/chunks
- small attached-viewer catchup buffers
- local staging before durable mirror flush

## Что показал `governor`

- Latest checked: `0.10.4`
- Repo: `boinkor-net/governor`
- Updated: `2026-04-13`

Это полезный crate, но очень важно не переоценить его роль.

`governor` хорош для:

- API rate limits
- fairness around externally-triggered actions
- maybe host-driven side operations

Но это **не** главный ответ для terminal backpressure.

Почему:

- terminal output pressure usually comes from ownership and buffer growth
- проблема не в том, что bytes arrive "too fast" в abstract sense
- проблема в том, что runtime must decide what to keep, what to drop, what to spill, and who is allowed to lag

## Что показали `lru` и `mini-moka`

- `lru 0.17.0`
- `mini-moka 0.10.3`

Они полезны только как:

- derived caches
- memoized projections
- maybe parsed metadata or link/search caches

Их нельзя делать authoritative state for:

- transcript truth
- replay correctness
- scrollback retention

То есть cache is not policy truth.

## Practical verdict

Если выбирать прямо сейчас, я бы делал так:

### V1

- Tokio owner-task runtime
- bounded `mpsc` lanes everywhere
- explicit per-session memory budget
- small hot replay via `ringbuf`
- durable mirror/spill in store layer
- UI lag or disconnect must never force unbounded buffering

### V2

- introduce `thingbuf` only in proven hot lanes with allocation churn
- optionally use `ArrayQueue` for narrow bounded MPMC islands

## Чего я бы избегал

- ❌ Unbounded channels for PTY output hot path
- ❌ Giant queue capacities "for safety"
- ❌ Treating cache crates as state authority
- ❌ Solving runtime pressure primarily with rate limiting
- ❌ Keeping all scrollback only in RAM for long-lived sessions

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- session owner must own memory budget enforcement
- hot replay and durable history must be separate layers
- backpressure decisions must be explicit and testable
- emulator-visible history policy and runtime spill policy must cooperate
- host disconnect/reconnect semantics should not require unbounded retention

## Sources

- [thingbuf crate](https://crates.io/crates/thingbuf)
- [thingbuf repo](https://github.com/hawkw/thingbuf)
- [crossbeam-queue crate](https://crates.io/crates/crossbeam-queue)
- [crossbeam repo](https://github.com/crossbeam-rs/crossbeam)
- [governor crate](https://crates.io/crates/governor)
- [governor repo](https://github.com/boinkor-net/governor)
- [ringbuf crate](https://crates.io/crates/ringbuf)
- [ringbuf repo](https://github.com/agerasev/ringbuf)
- [memmap2 crate](https://crates.io/crates/memmap2)
- [memmap2 repo](https://github.com/RazrFalcon/memmap2-rs)
- [alacritty_terminal crate](https://crates.io/crates/alacritty_terminal)
- [Alacritty repo](https://github.com/alacritty/alacritty)
- [lru crate](https://crates.io/crates/lru)
- [mini-moka crate](https://crates.io/crates/mini-moka)
