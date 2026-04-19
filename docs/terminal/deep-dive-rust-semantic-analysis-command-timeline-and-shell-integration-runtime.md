# Deep Dive - Rust Semantic Analysis, Command Timeline, And Shell Integration Runtime

**Проверено**: 2026-04-19

## Зачем этот слой важен

Для terminal package мирового уровня мало иметь:

- PTY
- emulator
- snapshots

Нужен ещё слой, который умеет строить:

- bounded semantic hints
- command timeline
- shell-visible context
- tool/file/action summaries

Именно здесь чаще всего всё ломают архитектурно:

- либо делают giant regex pile без границ
- либо тащат parser combinator в hot path на каждый chunk
- либо смешивают raw transcript, semantic cache и durable truth

Для reusable Rust runtime это особенно опасно, потому что потом host UI, automation и recovery начинают спорить, где находится "истина".

## Primary Sources

### Product/runtime references

- [`Hermes` analyzer.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/analyzer.rs)
- [`Hermes` adapters.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/adapters.rs)
- [`Hermes` patterns.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/patterns.rs)

### Rust matcher and parsing crates

- [`regex` README](https://github.com/rust-lang/regex/blob/master/README.md)
- [`regex-automata` README](https://github.com/rust-lang/regex/blob/master/regex-automata/README.md)
- [`aho-corasick` README](https://github.com/BurntSushi/aho-corasick/blob/master/README.md)
- [`bstr` README](https://github.com/BurntSushi/bstr/blob/master/README.md)
- [`memchr` README](https://github.com/BurntSushi/memchr/blob/master/README.md)
- [`winnow` README](https://github.com/winnow-rs/winnow/blob/main/README.md)
- [`nom` README](https://github.com/rust-bakery/nom/blob/main/README.md)
- [`chumsky` README](https://github.com/zesterer/chumsky/blob/main/README.md)
- [`logos` README](https://github.com/maciejhirsz/logos/blob/master/README.md)

### ANSI normalization references

- [`strip-ansi-escapes` README](https://github.com/luser/strip-ansi-escapes/blob/master/README.md)
- [`vte` README](https://github.com/alacritty/vte/blob/master/README.md)
- [`anstyle-parse` README](https://github.com/rust-cli/anstyle/blob/main/crates/anstyle-parse/README.md)

## Freshness signals

- `regex 1.12.3` - repo `rust-lang/regex`, `3950` stars, pushed `2026-02-24`
- `regex-automata 0.4.14`
- `aho-corasick 1.1.4` - repo `BurntSushi/aho-corasick`, `1231` stars, pushed `2026-02-27`
- `bstr 1.12.1` - repo `BurntSushi/bstr`, `1061` stars, pushed `2026-02-10`
- `memchr 2.8.0` - repo `BurntSushi/memchr`, `1420` stars, pushed `2026-02-12`
- `winnow 1.0.1` - repo `winnow-rs/winnow`, `880` stars, pushed `2026-04-17`
- `nom 8.0.0` - repo `rust-bakery/nom`, `10.4k` stars, pushed `2025-08-26`
- `chumsky 1.0.0-alpha.8` - repo `zesterer/chumsky`, `4536` stars, pushed `2026-03-27`
- `logos 0.16.1` - repo `maciejhirsz/logos`, `3478` stars, pushed `2026-04-16`
- `strip-ansi-escapes 0.2.1`
- `vte 0.15.0` - repo `alacritty/vte`, `311` stars, pushed `2026-02-28`
- `anstyle-parse 1.0.0` - repo `rust-cli/anstyle`, `160` stars, pushed `2026-04-16`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**semantic analyzer в reusable terminal runtime должен быть layered pipeline, а не одним parser crate**

На сейчас strongest shape выглядит так:

1. raw PTY bytes stay raw
2. ANSI normalization is streaming or derived, not regex replace
3. cheap byte-first prefilters cut the search space
4. stable line patterns use compiled regexes
5. narrow structured payloads use a small parser seam
6. results land in a bounded semantic cache
7. command timeline stays a derived projection above that cache

## Top 3 directions for Rust semantic runtime

### 1. `bstr + memchr + aho-corasick + regex`

`🎯 10   🛡️ 9   🧠 7`
Примерно `5000-10000` строк.

Это мой текущий **лучший default**.

Почему:

- `bstr` keeps the analyzer honest about non-UTF-8 PTY reality
- `memchr` is the cheapest first filter for newline and sentinel bytes
- `aho-corasick` is excellent for multi-literal detectors
- `regex` is ideal for bounded line-local capture extraction

Это самый здоровый stack для:

- provider detection
- slash-command spotting
- tool/file/action markers
- token/cost lines
- cwd/prompt hints

### 2. `regex-automata + bstr + memchr`

`🎯 8   🛡️ 8   🧠 8`
Примерно `6000-12000` строк.

Это stronger expert path, если analyzer станет реально hot-path critical.

Почему:

- `regex-automata` даёт более низкоуровневый контроль над engines и pattern sets
- лучше подходит, если нужна более явная multi-pattern story
- useful if you later want tighter automata control than `regex` facade gives

Почему не default:

- API заметно экспертнее
- easier to over-engineer too early
- v1 reusable package probably does not need this immediately

### 3. `winnow` for narrow grammars only

`🎯 7   🛡️ 8   🧠 7`
Примерно `3000-7000` строк.

Это хороший path для:

- OSC payload parsing after prefilter
- structured shell markers
- narrow command-summary grammars
- future protocol-ish line payloads

Почему именно `winnow`, а не parser-combinators everywhere:

- он выглядит аккуратнее для narrow byte/text parsers
- его проще держать как local seam, не превращая runtime в compiler project

## 1. `Hermes` confirms the right product boundary

`Hermes` полезен не потому, что у них "идеальный parser stack", а потому что они хорошо показывают shape semantic runtime.

Из `analyzer.rs` и `adapters.rs` видно:

- `OutputAnalyzer` - это bounded cache, не transcript DB
- provider specifics live behind `ProviderAdapter`
- `NodeBuilder` builds command/AI timeline entries
- `CompletedNode` is already a derived summary, not raw truth

Очень важные детали:

- `output_lines` в node builder capped at `50`
- summary is truncated to around `500` chars
- completed nodes capped at `20`
- many other stores are `VecDeque`-bounded

🔥 Это очень сильный architectural rule:

**semantic analyzer должен быть bounded cache, а не бесконечной историей**

## 2. `patterns.rs` is useful precisely because it is not trying to parse everything

`Hermes` uses compiled `regex::Regex` for:

- `OSC7_RE`
- token usage patterns
- cost lines
- tool call lines
- slash commands
- file paths
- provider/model/version hints

Это важный lesson:

- many valuable terminal semantics are line-local and stable
- they do not require a full grammar engine

Но ещё важнее то, чего там **нет**:

- no attempt to make regex the durable truth model
- no attempt to parse the entire PTY stream with one grammar

That is the correct boundary.

## 3. `bstr` should be the default mindset for analyzer input

`bstr` turned out to be more important than it looks.

Its core value here is not "fancy string API", but:

- byte strings not required to be valid UTF-8
- fast line-oriented processing on bytes
- Unicode-aware helpers when needed, but not as a mandatory assumption

For terminal runtime this matters because:

- PTY output is not guaranteed to be valid UTF-8 at every boundary
- replay windows and partial chunks can split code points
- shell and TUI output may include arbitrary bytes and control sequences

🔥 Practical rule:

**semantic runtime should start from bytes, not from `String`**

## 4. `memchr` is the cheapest hot-path primitive in this whole layer

`memchr` looks small, but for terminal analyzer design it is foundational.

It is ideal for:

- finding line breaks
- finding `ESC`
- finding sentinel bytes before expensive parsing
- cheap substring prechecks through `memmem`

Because it operates on raw bytes with SIMD-backed search paths, it is a perfect first stage before any heavier matcher.

This suggests a strong pipeline shape:

1. `memchr` finds candidate boundaries
2. `bstr` gives line slices
3. only then do richer matchers run

## 5. `aho-corasick` is better than giant alternation regex for many detectors

`aho-corasick` is especially strong when the analyzer wants to detect many literal signatures at once:

- slash command families
- tool names
- provider names
- fixed approval/error phrases
- shell integration markers

Why it is stronger than a giant regex in these cases:

- linear multi-pattern search
- natural mapping to pattern IDs
- cleaner false-positive policy when combined with boundary checks
- easier staged architecture: "did any candidate literal appear?" before capture parsing

🔥 Strong recommendation:

**use `aho-corasick` for multi-literal candidate detection, not `regex` for everything**

## 6. `regex` is still the best default for bounded capture extraction

The `regex` crate remains the best pragmatic default for:

- stable line-local capture groups
- bounded token and cost formats
- file path suffix extraction
- simple prompt and approval patterns

The primary source reminder matters here:

- worst case `O(m * n)`
- no backreferences or look-around
- compile once, do not compile in loops

That is actually a feature for a reusable runtime:

- more predictable
- less accidental ReDoS shape
- simpler to audit

## 7. `regex-automata` is the expert step-up, not the starting point

`regex-automata` is powerful, but its README correctly signals an "expert" API.

Where it becomes interesting:

- large multi-pattern bundles
- explicit engine selection
- more deliberate automata control
- situations where the higher-level `regex` facade becomes a limiting abstraction

Where it should not be used by default:

- ordinary line capture extraction
- early v1 analyzers
- places where team readability matters more than squeezing the last percent

Practical takeaway:

- start with `regex`
- keep `regex-automata` as a future seam for the hottest pattern sets

## 8. Parser combinators should be narrow, not universal

After comparing `winnow`, `nom` and `chumsky`, the conclusion is pretty clear.

### `winnow`

Best fit here when you need:

- a small structured marker grammar
- byte-oriented, zero-copy-ish local parsing
- parser combinators without turning the project into a language frontend

### `nom`

Still strong and battle-tested, but for this use case it feels more like:

- a general parser toolkit
- better when the structured grammar is central to the subsystem

not when it is just a seam inside a much larger runtime.

### `chumsky`

Very interesting, but it is still `1.0.0-alpha.8` and much more compiler-ish in ergonomics.

I would not make it the default inside a terminal runtime whose primary hot path is PTY chunks.

### `logos`

Excellent for lexers, but usually the wrong center for terminal semantic analysis unless you are truly building a token language over a structured textual protocol.

🔥 Practical rule:

**parser combinators belong on narrow structured payloads, not on the raw PTY stream**

## 9. ANSI normalization must not be "remove escapes with regex"

This part is easy to get wrong.

### `strip-ansi-escapes`

Good for:

- log export
- quick plain-text mirrors
- leaf tooling

But weak as analyzer truth because it is fundamentally "strip and forget".

### `vte`

Good reminder that terminal control bytes should ideally be handled by a streaming parser with explicit state machine semantics.

### `anstyle-parse`

Interesting for style-escape parsing, but narrower than full terminal semantics.

The healthier runtime rule is:

- if emulator/runtime already has a normalized text projection, analyze that
- otherwise use a streaming normalization seam
- do not regex-strip ANSI on the hot path and call that truth

## 10. Recommended layered analyzer shape

For this package I would now recommend:

1. `RawChunkIn`
   - bytes from PTY
2. `NormalizationStage`
   - escape-aware plain-text or structured line projection
3. `BytePrefilterStage`
   - `memchr` and `aho-corasick`
4. `CaptureStage`
   - compiled `regex`
5. `NarrowGrammarStage`
   - optional `winnow` on structured payloads only
6. `SemanticCache`
   - bounded provider/tool/file/action/token/cwd state
7. `TimelineProjection`
   - command summaries, completed nodes, attention events

This separates:

- throughput concerns
- parsing concerns
- semantic ownership
- durable persistence

## 11. What I would explicitly avoid

- ❌ `String` as the mandatory first representation of PTY output
- ❌ regexes compiled in loops
- ❌ one mega-regex for all provider/tool/slash command detection
- ❌ parser combinators over every line or chunk by default
- ❌ regex-based ANSI stripping as canonical analyzer input
- ❌ storing semantic cache as durable transcript truth
- ❌ letting command timeline mutate the raw output model

## Final recommendation

If building the Rust semantic runtime today, I would choose:

- bytes in: `bstr`
- cheap scanning: `memchr`
- multi-literal detectors: `aho-corasick`
- capture extraction: `regex`
- structured payload seam: `winnow`, but only where clearly justified
- ANSI normalization: derived projection or streaming parser, not regex stripping

And I would treat:

- `regex-automata` as future hot-path upgrade
- `nom` as strong but heavier alternative for dedicated grammar islands
- `chumsky` and `logos` as interesting, but not default fits for this runtime layer

## Sources

- [Hermes analyzer.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/analyzer.rs)
- [Hermes adapters.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/adapters.rs)
- [Hermes patterns.rs](https://github.com/hermes-hq/hermes-ide/blob/main/src-tauri/src/pty/patterns.rs)
- [regex](https://github.com/rust-lang/regex)
- [regex-automata](https://github.com/rust-lang/regex/tree/master/regex-automata)
- [aho-corasick](https://github.com/BurntSushi/aho-corasick)
- [bstr](https://github.com/BurntSushi/bstr)
- [memchr](https://github.com/BurntSushi/memchr)
- [winnow](https://github.com/winnow-rs/winnow)
- [nom](https://github.com/rust-bakery/nom)
- [chumsky](https://github.com/zesterer/chumsky)
- [logos](https://github.com/maciejhirsz/logos)
- [strip-ansi-escapes](https://github.com/luser/strip-ansi-escapes)
- [vte](https://github.com/alacritty/vte)
- [anstyle-parse](https://github.com/rust-cli/anstyle/tree/main/crates/anstyle-parse)
