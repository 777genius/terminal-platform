# Deep Dive - Rust Panic, Unwind, Error Envelopes And Fault Containment

**Проверено**: 2026-04-19

## Зачем этот deep dive

После предыдущих deep dive уже стало видно, что reusable terminal package должен иметь:

- protocol-first public contract
- explicit handles
- explicit async semantics
- clear ownership over memory and buffers

Но есть ещё один слой, без которого всё это остаётся хрупким:

🔥 **что именно происходит при panic, unwind, bad callback, poisoned session state и boundary-level faults**

Для universal terminal package это особенно важно, потому что у нас есть сразу несколько boundary classes:

- in-process C ABI
- Node/Electron adapter
- daemon protocol
- foreign callbacks / callback interfaces
- background session runtime

Если fault model здесь расплывчатая, итог обычно плохой:

- panic accidentally crosses FFI boundary
- callback error silently becomes process corruption
- one broken session poisons the whole runtime
- host SDKs receive panic text instead of typed error semantics
- crash behavior differs between Node, C and daemon clients

## Primary Sources

- [Rust Reference - behavior considered undefined](https://doc.rust-lang.org/beta/reference/behavior-considered-undefined.html)
- [Rustonomicon - Unwinding](https://doc.rust-lang.org/nomicon/unwinding.html)
- [`ffi-support` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/)
- [`ffi-support::abort_on_panic` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/abort_on_panic/index.html)
- [`interoptopus::patterns::result::FFIError` docs.rs](https://docs.rs/interoptopus/latest/interoptopus/patterns/result/trait.FFIError.html)
- [`interoptopus` `#[ffi_function]` docs.rs](https://docs.rs/interoptopus/latest/interoptopus/attr.ffi_function.html)
- [`UniFFI` foreign traits guide](https://mozilla.github.io/uniffi-rs/latest/foreign_traits.html)
- [`async-ffi` docs.rs](https://docs.rs/async-ffi/latest/async_ffi/)
- [`unwind_aborts` docs.rs](https://docs.rs/unwind_aborts/latest/unwind_aborts/)
- [`nounwind` docs.rs](https://docs.rs/nounwind/latest/nounwind/)
- [`tarnish` docs.rs](https://docs.rs/tarnish/0.0.2/tarnish/)

## Короткий вывод

🔥 Для universal terminal package правильная fault model должна жёстко разделять:

1. **domain errors**
2. **boundary faults**
3. **runtime poison / session invalidation**
4. **process-fatal crashes**

И главное правило:

🔥 **panic must never become implicit cross-language control flow**

## Top 3 Fault-Policy Strategies

### 1. `Typed domain errors + guarded boundaries + explicit poison/fatal semantics`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Что это значит:

- normal business failures return typed error envelopes
- every FFI/protocol boundary has a panic guard
- panic never escapes the boundary
- if a panic or severe callback fault happens, runtime emits explicit fatal state:
  - operation failed
  - session poisoned
  - subscription terminated
  - runtime fatal

Почему это strongest path:

- keeps product semantics stable across hosts
- lets daemon clients, Node clients and C clients observe the same class of failures
- supports crash isolation and future remote runtimes better

Практический вывод:

✅ Это мой лучший default.

### 2. `Abort-on-panic at strict C ABI leaves + typed protocol errors elsewhere`

`🎯 8   🛡️ 9   🧠 7`  
Примерно `5000-11000` строк.

Что это значит:

- low-level C ABI leaves may choose to abort instead of catching/encoding panic
- protocol/daemon surfaces still expose typed error/fatal envelopes
- host bindings with stronger ergonomics can sit above a safer process boundary

Почему это интересно:

- simple and honest for thin C edges
- avoids pretending a panic is recoverable when the boundary cannot safely continue

Где риск:

- inconsistent semantics if overused
- easy to make the public SDK story harsher than necessary

Практический вывод:

✅ Good as a narrow leaf policy.  
⚠️ Not enough as the whole product fault model.

### 3. `Process isolation for dangerous or untrusted islands`

`🎯 7   🛡️ 8   🧠 9`  
Примерно `9000-18000` строк.

Что это значит:

- especially risky subsystems can run out of process
- crash or abort becomes worker death, not host death
- main runtime or host can restart/recover

Почему это интересно:

- strongest containment
- useful for future untrusted plugins, experimental analyzers or foreign adapters

Где риск:

- complexity and latency
- not justified for every boundary

Практический вывод:

✅ Strong future seam.  
⚠️ Too heavy as the universal v1 default.

## Tool-by-tool findings

## 1. Rust itself sets the hard rule - unwinding across the wrong FFI boundary is UB

The Rust Reference and Rustonomicon are explicit:

- wrong call ABI is UB
- unwinding into foreign code or out of foreign code is UB

This means:

🔥 **“we will just let panic propagate and hope the binding catches it” is not a real architecture**

For the package:

- every FFI leaf must choose a deliberate policy
- either catch and translate
- or abort

## 2. `ffi-support` - strongest practical donor for boundary guards

- Crate: [`ffi-support`](https://crates.io/crates/ffi-support)
- Latest: `0.4.4`

What matters most:

- `call_with_result` and `call_with_output` explicitly catch panics
- normal Rust errors convert into `ExternError`
- `abort_on_panic` module exists for edges that should abort instead
- crate docs explicitly say panicking across the FFI boundary is undefined and weird things happen

This is an extremely useful practical pattern:

- boundary wrapper owns panic policy
- domain code returns normal results
- leaf adapter decides whether panic becomes:
  - encoded fault
  - process abort

## 3. `interoptopus` - useful proof that panic deserves its own FFI-level category

- Crate: [`interoptopus`](https://crates.io/crates/interoptopus)
- Latest observed: `0.16.0-alpha.20`

What matters most:

- `FFIError` trait explicitly requires `SUCCESS`, `NULL`, `PANIC`
- docs for `#[ffi_function]` explicitly warn that exported functions must never panic

This is a strong signal:

🔥 **panic is not just another app error string**

It deserves its own boundary category.

## 4. `UniFFI` - foreign callbacks are a fault seam, not just an extension seam

The `foreign_traits` docs are especially important:

- foreign traits can create cycles and leaks
- unexpected callback errors should be converted explicitly
- if you don’t implement the relevant conversion path, generated code may panic

This is directly relevant for our future host adapters:

- foreign callbacks should be treated as dangerous boundaries
- callback failures need typed conversion
- callback panic/error cannot be allowed to silently poison runtime truth

## 5. `async-ffi` - useful reminder that async panic handling is still edge work

`async-ffi` explicitly states:

- FFI-safe futures are wrappers around async work
- panic during `poll` is caught and represented
- panic in drop or waker-related machinery is much harder and may abort

This is very valuable because it shows:

- even when async is made FFI-safe, not every failure becomes cleanly recoverable
- low-level future/waker mechanics are not a good public product contract

## 6. `unwind_aborts` and `nounwind` - useful leaf tools, not the whole model

- `unwind_aborts 0.1.1`
- `nounwind 0.1.5`

Both crates are useful because they make leaf functions abort if they unwind.

Why they matter:

- useful for very thin exported FFI leaves
- good when the safest response is “never continue after this boundary panic”

Why they are not enough:

- they do not define typed error envelopes
- they do not define poison semantics
- they do not define how the rest of the runtime should react

## 7. `tarnish` - interesting donor for process-level fault containment

- Crate: [`tarnish`](https://crates.io/crates/tarnish)
- Latest: `0.0.2`

What matters most from docs:

- explicitly contrasts process isolation with `catch_unwind`
- process isolation survives segfaults, aborts and FFI crashes

This is relevant because:

- `catch_unwind` is not magic crash containment
- some failure classes are only containable with process boundaries

For our package this suggests a strong future seam:

- high-risk or untrusted islands may eventually need worker-process containment

## The key architectural rules

## 1. Domain errors and boundary faults must be different types

Examples of domain errors:

- invalid profile
- session not found
- unsupported shell integration capability
- search query invalid

Examples of boundary faults:

- panic caught at FFI boundary
- unexpected foreign callback failure
- invalid handle map state
- data-plane decode corruption

These should not share one generic error enum just because both are “errors”.

## 2. Panic text is not a public API

Bad design:

- pass raw panic message to the host and call it the error contract

Better design:

- public SDK sees typed fatal category
- internal telemetry/logging records panic message and backtrace where allowed

This keeps public semantics stable and telemetry richer than API.

## 3. Session poison is not the same as process fatal

For terminal runtime a useful split is:

- operation failed
- subscription closed
- session poisoned and must be recreated
- runtime fatal and host should reconnect or restart

This gives much better containment than:

- everything is either okay or crash the whole host

## 4. Foreign callbacks need explicit fault policy

Whenever Rust calls back into host code:

- title callbacks
- auth prompts
- file open actions
- future plugin hooks

there should be a policy:

- callback error maps to typed host-fault result
- callback timeout or disconnect maps to cancellation/failure
- callback panic/unexpected state does not corrupt the core runtime silently

## 5. Boundary wrappers should be tiny

The safest pattern is:

- leaf boundary wrapper catches/aborts
- converts to boundary error carrier
- delegates immediately to runtime/application services

Do not keep business logic inside the boundary wrapper.

## 6. Logging and fault semantics should be separate

Useful pattern:

- public boundary returns typed envelope
- internal telemetry emits:
  - panic origin
  - callback source
  - session id
  - operation id
  - fatal category

That means observability can evolve without changing public API.

## Recommended fault model for this project

```text
Recoverable domain errors
  -> typed result/error envelope

Boundary panic or unexpected callback failure
  -> typed fatal/boundary-fault category
  -> optionally poison only the affected session/subscription

Allocator corruption / invalid ABI / impossible runtime invariant
  -> runtime fatal or process abort

Untrusted or highly unsafe islands
  -> future process isolation seam
```

## What I would do in v1

1. Define separate categories for:
   - domain error
   - boundary fault
   - session poisoned
   - runtime fatal
2. Guard every FFI leaf from panic crossing.
3. Keep panic payloads in logs/telemetry, not as public API strings.
4. Treat foreign callbacks as explicit fault seams.
5. Allow a session to become invalid without requiring full-process death.
6. Reserve abort for truly unsafe/no-recovery boundaries.

## What I would avoid in v1

- ❌ One giant error enum for every possible failure
- ❌ Treating panic message as the stable error contract
- ❌ Letting panic unwind across any foreign boundary
- ❌ Assuming `catch_unwind` is enough for segfault/abort/allocator corruption
- ❌ Making Node/C/daemon hosts observe different failure classes for the same runtime event

## Final take

Если сжать всё в одну фразу:

🔥 **a world-class embeddable terminal runtime must treat panic and boundary faults as explicit architectural concepts, not incidental exceptions**

То есть:

- normal errors are typed results
- panic never crosses the boundary
- fatal categories are explicit
- session poison is contained
- process isolation remains an available future seam for truly unsafe islands
