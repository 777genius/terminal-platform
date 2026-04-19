# Deep Dive - Rust Transcript, Search, Indexing, and Link Surfaces

**Проверено**: 2026-04-19

## Зачем смотреть этот слой отдельно

У terminal package мирового уровня почти неизбежно появляются сразу несколько поисковых и текстовых поверхностей:

- live scrollback find
- durable transcript history
- command/output search
- link detection
- export/debug snapshots
- возможно позже global workspace search

Если всё это сложить в один "умный текстовый буфер", архитектура быстро разваливается:

- hot PTY path начнёт зависеть от тяжёлой индексации
- live transcript truth перемешается с derived search data
- локальный find и глобальный history search начнут тянуть разные компромиссы в одном storage
- host projection начнёт зависеть от слишком тяжёлого внутреннего text model

Здесь правильный вопрос не "какую search library взять", а:

- что является transcript truth
- где нужен rope/read model
- где нужен lightweight scan
- где уже оправдан полноценный индекс

## Топ 3

### 1. `Append-first transcript + read-model rope + lightweight scanners`
`🎯 10   🛡️ 9   🧠 7`  
Примерно `6000-12000` строк.

Это strongest default.

Идея:

- source of truth остаётся append-first transcript/event stream
- поверх него строится отдельный text read model
- для live find использовать `memchr`, `bstr`, `aho-corasick`, `regex`
- для link detection использовать `linkify`
- heavy indexing не сидит на PTY hot path

Почему это лучше всего:

- не смешивает hot path и durable index
- даёт нормальную архитектуру для local find vs global history
- хорошо ложится на already established truth/read-model separation

Что я бы взял здесь:

- `crop 0.4.3` как лучший stable rope candidate
- `memchr 2.8.0` для very cheap scans
- `linkify 0.11.0` для link surface
- `fst 0.4.7` только если нужен compact prefix-like dictionary/index later

### 2. `Rope-centered transcript model` через `crop` или `ropey`
`🎯 8   🛡️ 7   🧠 6`  
Примерно `5000-10000` строк.

Это рабочий путь, если transcript сразу нужен как searchable text surface.

Почему он интересен:

- ropes naturally дают line/offset operations
- cheap clones/snapshots полезны для background export/search
- удобнее строить line-oriented projections и partial search windows

Но тут есть важные нюансы:

- `ropey` сейчас на `2.0.0-beta.1`, то есть latest release уже beta
- `crop` стабильнее как package choice
- rope не должен становиться единственной truth model runtime

То есть rope-centered path возможен, но лучше всё равно держать transcript truth и rope projection раздельно.

### 3. `Full-text engine` через `tantivy` как separate durable history/search surface
`🎯 7   🛡️ 8   🧠 8`  
Примерно `7000-14000` строк.

Это сильный путь, когда появится:

- search across many sessions
- persistent command history search
- rich filtering/facets/tags
- global workspace search over terminal artifacts

Почему это уже серьёзно:

- `tantivy 0.26.0` очень зрелый и активно поддерживается
- repo обновлялся 2026-04-19
- даёт полноценный index/query model

Почему это не v1 transcript truth:

- documents immutable, update model через delete + reindex
- reader/searcher reload semantics уже тяжелее, чем нужно live runtime
- для live scrollback/find это overkill

## Самый важный вывод

🔥 У reusable terminal package должно быть как минимум **3 разных текстовых слоя**:

1. `Transcript truth`
2. `Read/search projection`
3. `Durable history index`

Их нельзя склеивать в одну "универсальную text engine" без большого архитектурного долга.

## Что показали `crop` и `ropey`

### `crop`

- Latest checked: `0.4.3`
- Repo: `noib3/crop`
- Updated: `2026-03-25`

Почему `crop` сейчас выглядит самым здоровым default rope:

- stable release line
- B-tree rope
- emphasis on performance
- explicitly parallel-friendly snapshots
- cheap clone story за счёт shared ownership
- good line and byte slicing APIs

Очень важный сигнал из README:

- `Rope` можно дёшево клонировать и отправлять в background thread для IO/CPU-heavy tasks

Это очень хорошо ложится на terminal architecture:

- live runtime остаётся responsive
- search/export/indexing может работать по snapshot

### `ropey`

- Latest checked: `2.0.0-beta.1`
- Repo: `cessen/ropey`
- Updated: `2026-04-16`

Почему `ropey` всё ещё очень интересен:

- сильная Unicode story
- line-aware APIs
- char-based indexing
- cheap clones
- good low-level chunk access

Но есть важная practical проблема:

- latest release line сейчас beta

Для пакета мирового уровня это не автоматически stop sign, но для strongest default я бы сейчас скорее доверял `crop`, а `ropey` держал как сильный alternative/donor.

## Что показал `jumprope`

- Latest checked: `1.1.2`
- Repo: `josephg/jumprope-rs`

Почему интересно:

- очень быстрый editing trace performance
- buffered edit mode
- wchar conversion feature

Почему не лучший default:

- акцент больше на editing traces, чем на rich line/read-model ergonomics
- `crop` README прямо подчёркивает, что cheap clone story там хуже
- для terminal transcript нам важны не только inserts, но и snapshot/export/search ownership

Я бы держал `jumprope` как performance donor, не как default transcript truth.

## Что показал `xi-rope`

- Latest checked: `0.3.0`
- Repo lineage: `xi-editor`

Почему это важно:

- сильный historical donor для rope science и metrics ideas

Почему не default:

- слишком больше похож на architectural ancestor/reference, чем на strongest modern package choice

## Что показал `tantivy`

### Когда он реально уместен

`tantivy 0.26.0` очень силён, если вам нужен уже настоящий search product layer:

- persisted session artifacts
- query language
- tags/facets
- search across many sessions/workspaces
- history that survives runtime restarts

### Когда он неуместен

Но из README и API shape видно, что это именно search engine library, не runtime text buffer:

- documents immutable
- visibility через commit/reload
- separate searcher/index lifecycle

То есть `tantivy` очень уместен как:

- separate durable history/search surface

И очень неуместен как:

- live scrollback store
- single source of truth for transcript

## Что показал `fst`

- Latest checked: `0.4.7`
- Repo: `BurntSushi/fst`
- Updated: `2026-04-16`

Почему это интересный building block:

- compact set/map over strings
- fast prefix/range-like lookups
- integrates conceptually well with `regex-automata`

Где он уместен:

- prefix dictionaries
- command name caches
- compact lookup side-indices

Где он не уместен:

- full transcript truth
- arbitrary free-form live text search engine

## Что показал `linkify`

- Latest checked: `0.11.0`
- Repo: `robinst/linkify`
- Updated: `2026-04-12`

Это сильный practical finding:

- link detection не надо решать regex-хаком
- crate отдельно показывает, почему boundary logic around punctuation tricky
- linear scan runtime

Для terminal package это хороший default для:

- durable transcript link extraction
- visible link overlays
- export link metadata

И плохой кандидат для:

- URL validation as policy truth

То есть `linkify` should detect, а не adjudicate trust.

## Practical verdict

Если выбирать прямо сейчас, я бы делал так:

### V1

- transcript truth: append-first runtime records
- search/read model: snapshot projection into `crop`
- live find: `memchr` + `bstr` + `aho-corasick` + bounded regex where needed
- links: `linkify`

### V2

- derived compact side-index for commands/tokens: optional `fst`
- durable global terminal history search: optional `tantivy`

## Чего я бы избегал

- ❌ Делать rope единственной runtime truth model
- ❌ Тащить `tantivy` в hot path live transcript
- ❌ Смешивать local find и global history search в один storage layer
- ❌ Держать link extraction на regex-наборе без нормальной boundary semantics
- ❌ Считать cached/full-text index authoritative instead of transcript truth

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- transcript truth должен быть append-first и host-neutral
- rope/read model должен быть derived snapshot or mirror
- local find и durable history search должны быть separate surfaces
- compact side-indices допустимы, но только как derived data
- host projection API должен возвращать explicit search/link/snapshot surfaces, не raw rope internals

## Sources

- [ropey crate](https://crates.io/crates/ropey)
- [ropey repo](https://github.com/cessen/ropey)
- [crop crate](https://crates.io/crates/crop)
- [crop repo](https://github.com/noib3/crop)
- [tantivy crate](https://crates.io/crates/tantivy)
- [tantivy repo](https://github.com/quickwit-oss/tantivy)
- [fst crate](https://crates.io/crates/fst)
- [fst repo](https://github.com/BurntSushi/fst)
- [linkify crate](https://crates.io/crates/linkify)
- [linkify repo](https://github.com/robinst/linkify)
- [xi-rope crate](https://crates.io/crates/xi-rope)
- [xi-editor repo](https://github.com/xi-editor/xi-editor)
- [jumprope crate](https://crates.io/crates/jumprope)
- [jumprope repo](https://github.com/josephg/jumprope-rs)
