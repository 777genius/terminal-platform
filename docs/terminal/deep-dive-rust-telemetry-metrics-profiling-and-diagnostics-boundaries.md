# Deep Dive - Rust Telemetry, Metrics, Profiling, and Diagnostics Boundaries

**Проверено**: 2026-04-19

## Зачем этот слой смотреть отдельно

Для terminal runtime мирового уровня мало сказать:

- "core emits tracing"
- "host can plug OTEL later"

Этого недостаточно, потому что в реальном продукте быстро появляются разные observational surfaces:

- semantic spans and events
- numeric counters, gauges and histograms
- profiling and flamegraphs
- async runtime introspection
- pretty diagnostics for standalone apps
- crash and panic context for leaf executables

🔥 Главная ошибка здесь - смешать всё это в один "observability stack" и дать ему диктовать architecture core runtime.

Для reusable embeddable package правильный вопрос не только "какие crates взять", а:

- что является native semantic event surface
- что является optional numeric surface
- что является debug/profiling leaf
- что разрешено только leaf app, а не core library

## Primary Sources

### Tracing backbone

- [`tracing` crate](https://crates.io/crates/tracing)
- [`tracing` repo](https://github.com/tokio-rs/tracing)
- [`tracing-subscriber` crate](https://crates.io/crates/tracing-subscriber)
- [`tracing-appender` crate](https://crates.io/crates/tracing-appender)
- [`tracing-error` crate](https://crates.io/crates/tracing-error)

### OpenTelemetry bridge

- [`opentelemetry` crate](https://crates.io/crates/opentelemetry)
- [`opentelemetry_sdk` crate](https://crates.io/crates/opentelemetry_sdk)
- [`OpenTelemetry Rust` repo](https://github.com/open-telemetry/opentelemetry-rust)

### Metrics ecosystem

- [`metrics` crate](https://crates.io/crates/metrics)
- [`metrics-util` crate](https://crates.io/crates/metrics-util)
- [`metrics-rs/metrics` repo](https://github.com/metrics-rs/metrics)

### Profiling and async inspection

- [`pprof` crate](https://crates.io/crates/pprof)
- [`pprof-rs` repo](https://github.com/tikv/pprof-rs)
- [`console-subscriber` crate](https://crates.io/crates/console-subscriber)
- [`tokio-rs/console` repo](https://github.com/tokio-rs/console)

### Pretty diagnostics at app leaves

- [`color-eyre` crate](https://crates.io/crates/color-eyre)
- [`eyre-rs/eyre` repo](https://github.com/eyre-rs/eyre)

## Freshness signals

- `tracing 0.1.44` - repo `tokio-rs/tracing`, `6640` stars, pushed `2026-04-17`
- `tracing-subscriber 0.3.23`
- `tracing-appender 0.2.5`
- `tracing-error 0.2.1`
- `opentelemetry 0.31.0` - repo `open-telemetry/opentelemetry-rust`, `2548` stars, pushed `2026-04-18`
- `opentelemetry_sdk 0.31.0`
- `metrics 0.24.3` - repo `metrics-rs/metrics`, `1446` stars, pushed `2026-04-14`
- `metrics-util 0.20.1`
- `pprof 0.15.0` - repo `tikv/pprof-rs`, `1612` stars, pushed `2026-04-14`
- `console-subscriber 0.5.0` - repo `tokio-rs/console`, `4497` stars, pushed `2026-04-09`
- `color-eyre 0.6.5` - repo `eyre-rs/eyre`, `1732` stars, pushed `2026-04-10`

## Короткий вывод

🔥 Самый важный вывод этого прохода:

**`tracing`, `metrics`, profiling and pretty diagnostics must be separate layers with different authority**

Healthiest shape сейчас выглядит так:

1. core runtime emits `tracing` as native semantic event surface
2. numeric metrics are optional derived surface
3. profiling tools are explicitly opt-in leaves
4. pretty diagnostics belong to standalone executables and test harnesses, not runtime core
5. OTEL is a bridge/export concern, not the architectural center

То есть не:

- one big observability framework everywhere

а:

- `TracingSurface`
- optional `MetricsSurface`
- optional `ProfilingSurface`
- leaf-only `HumanDiagnosticsSurface`

## Top 3 directions for production diagnostics stack

### 1. `Tracing-first core + optional metrics + opt-in profiling leaves + leaf-only pretty diagnostics`

`🎯 10   🛡️ 9   🧠 7`
Примерно `6000-13000` строк.

Это strongest default.

Идея:

- all runtime crates emit spans/events through `tracing`
- metrics stay optional and derived
- `pprof` and `console-subscriber` are only enabled in dedicated debug/profiling modes
- `color-eyre` and similar crates stay in CLI/desktop/test leaves
- host decides subscribers/exporters/retention

Почему это лучший путь:

- reusable core stays library-first
- diagnostic richness can evolve without changing public protocol
- profiling and human-friendly formatting do not leak into runtime truth
- Electron, CLI and foreign hosts can choose different operational surfaces

### 2. `Tracing + metrics as parallel core surfaces, profiling mostly external`

`🎯 8   🛡️ 8   🧠 6`
Примерно `5000-11000` строк.

Это рабочий path, если product really wants first-class counters early.

Почему он интересен:

- counters and histograms can be useful for session fleets or remote deployments
- `metrics` crate gives a clean facade

Почему не мой default:

- teams often over-instrument numbers before they know which ones matter
- counters start shaping architecture too early
- event semantics usually age better than metric guesses

### 3. `OTEL-first and diagnostics-heavy core`

`🎯 4   🛡️ 6   🧠 8`
Примерно `7000-15000` строк.

Это path where:

- OTEL SDK is everywhere
- pretty error handlers leak into library layers
- profiling hooks stay always-on

Почему это плохой default:

- host neutrality gets worse
- standalone operational assumptions leak into SDK core
- harder to embed in products that already have their own telemetry policy

## 1. `tracing` remains the only credible native semantic event surface

`tracing 0.1.44` and `tracing-subscriber 0.3.23` still look like the healthiest base.

Most important rule from this ecosystem remains exactly right for our package:

🔥 **libraries emit tracing, hosts install subscribers**

Why this is such a good fit:

- terminal runtime has rich lifecycle events and state transitions
- spans are better than logs for operations like attach, replay, restore, search, export
- cross-host embedding requires semantic instrumentation without hardcoded collectors

Practical implication:

- runtime crates depend on `tracing`
- app/daemon/debug harness crates decide how spans are collected
- no global subscriber install in core

## 2. `metrics` is useful, but should stay derived

`metrics 0.24.3` and `metrics-util 0.20.1` form a serious ecosystem, but the architectural role matters more than the crates themselves.

Where metrics shine:

- queue depth
- replay lag
- attach count
- session counts by route
- spill volume
- search latency histograms
- projection rebuild timings

Where metrics become dangerous:

- when they replace semantic events
- when counters start defining lifecycle truth
- when "to_metrics()" clones too much state too often

🔥 Strong rule:

**numeric metrics should be derived from runtime truth, not become runtime truth**

This fits prior findings well:

- cheap phase projections separate from heavy metrics snapshots
- bounded analyzer/runtime should not clone giant state every tick

## 3. `OpenTelemetry` is still a bridge, not the center

`opentelemetry 0.31.0` and `opentelemetry_sdk 0.31.0` are healthy and active.

They matter because:

- some embedders will want OTLP and standard observability backends
- remote/runtime fleet modes may later need standardized export

But architectural role is still clear:

- core emits `tracing`
- hosts may bridge that into OTEL
- OTEL SDK should not define what core runtime means by lifecycle events

So the right position is:

- OTEL optional bridge layer
- not mandatory dependency of minimal core crates

## 4. `pprof` is an excellent profiling leaf

`pprof 0.15.0` looks very strong for:

- on-demand flamegraphs
- CI or developer performance harnesses
- targeted profiling of hot lanes like replay, parsing or search

Especially useful for this package because we have multiple suspected hot paths:

- VT parsing
- diff/projection generation
- transcript scanning and links
- replay catch-up
- spill compression

But the key boundary is:

🔥 profiling is not always-on telemetry

It should stay:

- debug/profile mode
- benchmark harness
- explicit admin/debug action

not:

- always-on core dependency shaping runtime design

## 5. `console-subscriber` is great for debugging Tokio runtime behavior, but not product observability

`console-subscriber 0.5.0` is very useful because this package will be heavily async and owner-task based.

What it is good for:

- finding stuck async tasks
- understanding wakeups and scheduling
- debugging task leaks or shutdown ordering

What it is **not**:

- end-user observability story
- public metrics contract
- replacement for domain spans/events

Practical implication:

- excellent dev/debug leaf
- not something to build protocol or public dashboards around

## 6. `tracing-error` and `color-eyre` should stay in diagnostic leaves

These crates are useful, but their boundary matters a lot.

### `tracing-error`

Good for:

- enriching errors with span context
- test harnesses
- daemon or standalone app diagnostics

### `color-eyre`

Good for:

- CLI tools
- standalone desktop debug builds
- integration test binaries

But both should stay out of core runtime contracts.

🔥 Strong rule:

**pretty error rendering is a leaf concern**

Core runtime should expose:

- typed domain/infrastructure errors
- explicit fatal/poison categories where needed

Leaf apps may then decorate them with:

- colorful reports
- span traces
- human-friendly panic messages

## 7. Observability should respect package layering

For this package, a healthy layering rule now looks like:

### Core crates

- emit `tracing`
- optionally increment lightweight metrics hooks if explicitly abstracted
- no subscriber install
- no pretty panic handlers
- no exporter side effects

### Daemon/app leaves

- install subscriber stack
- choose JSON/text/file appenders
- maybe enable OTEL bridge
- maybe expose metrics endpoint
- maybe enable profiling/admin routes

### Dev/test/profile leaves

- `console-subscriber`
- `pprof`
- `color-eyre`
- richer debug filters and local appenders

## Practical verdict

If I were designing this layer right now:

### V1

- `tracing` in all runtime crates
- host-selected `tracing-subscriber` setup
- `tracing-appender` only in daemon/CLI/debug leaves
- optional `metrics` facade only where a real numeric surface is justified
- `tracing-error` and `color-eyre` only in leaf executables and harnesses
- `pprof` only in explicit profiling leaves
- `console-subscriber` only in debug/developer runtime modes

### V2

- optional OTEL bridge for daemon/fleet mode
- carefully chosen metrics registry and export surface
- profiling/admin routes or sidecar hooks
- explicit sampling policy for high-volume spans

## Чего я бы избегал

- ❌ Making OTEL SDK a mandatory core dependency
- ❌ Installing global subscribers in library crates
- ❌ Treating counters as runtime truth
- ❌ Keeping profiling hooks always-on in core
- ❌ Returning pretty diagnostics types from core APIs

## Architecture implications

Для нашего будущего Rust package отсюда следует:

- `tracing` should remain the native semantic diagnostics surface
- metrics should be explicit and derived
- profiling should be opt-in and leaf-scoped
- human diagnostics should stay out of core contracts
- observability setup belongs to composition roots, not domain/application services

## Sources

- [tracing crate](https://crates.io/crates/tracing)
- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- [tracing-subscriber crate](https://crates.io/crates/tracing-subscriber)
- [tracing-appender crate](https://crates.io/crates/tracing-appender)
- [tracing-error crate](https://crates.io/crates/tracing-error)
- [opentelemetry crate](https://crates.io/crates/opentelemetry)
- [opentelemetry_sdk crate](https://crates.io/crates/opentelemetry_sdk)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [metrics crate](https://crates.io/crates/metrics)
- [metrics-util crate](https://crates.io/crates/metrics-util)
- [metrics-rs/metrics](https://github.com/metrics-rs/metrics)
- [pprof crate](https://crates.io/crates/pprof)
- [tikv/pprof-rs](https://github.com/tikv/pprof-rs)
- [console-subscriber crate](https://crates.io/crates/console-subscriber)
- [tokio-rs/console](https://github.com/tokio-rs/console)
- [color-eyre crate](https://crates.io/crates/color-eyre)
- [eyre-rs/eyre](https://github.com/eyre-rs/eyre)
