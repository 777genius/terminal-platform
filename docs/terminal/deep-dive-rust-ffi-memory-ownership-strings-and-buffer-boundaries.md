# Deep Dive - Rust FFI Memory Ownership, Strings And Buffer Boundaries

**Проверено**: 2026-04-19

## Зачем этот deep dive

После предыдущих deep dive уже стало ясно, что reusable terminal package должен быть:

- protocol-first
- handle-first
- runtime-first
- adapter-leaf oriented

Но остаётся ещё один слой, на котором embeddable packages очень часто ломаются уже в production:

🔥 **кто владеет памятью на FFI boundary, как передавать строки и байты, и кто именно имеет право их освобождать**

Для terminal package это не мелочь. Здесь у нас постоянно ходят:

- screen snapshots
- deltas
- transcript chunks
- search results
- config strings
- command arguments
- protocol payloads

Если ownership contract мутный, то дальше почти неизбежно появляются:

- leaks
- double free
- allocator mismatch between shared libraries
- host-specific hacks for strings and buffers
- impossible-to-stabilize public APIs

## Primary Sources

- [`ffi-support` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/)
- [`ffi-support::ByteBuffer` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/struct.ByteBuffer.html)
- [`ffi-support::HandleMap` docs.rs](https://docs.rs/ffi-support/latest/ffi_support/handle_map/struct.HandleMap.html)
- [`UniFFI` RustBuffer docs](https://mozilla.github.io/uniffi-rs/0.27/internals/api/uniffi/struct.RustBuffer.html)
- [`UniFFI` lifting/lowering docs](https://mozilla.github.io/uniffi-rs/next/internals/lifting_and_lowering.html)
- [`Diplomat runtime` docs.rs](https://docs.rs/diplomat-runtime/latest/diplomat_runtime/)
- [`safer_ffi` user guide](https://getditto.github.io/safer_ffi/)
- [`safer_ffi` rustdoc](https://getditto.github.io/safer_ffi/rustdoc/safer_ffi/)

## Короткий вывод

🔥 Для universal terminal package правильный memory contract выглядит так:

1. **public FFI types must encode ownership deliberately**
2. **outbound Rust-owned data and inbound foreign-owned views must be different shapes**
3. **allocator responsibility must stay with the allocating side**
4. **strings should not all be modelled as `char *`**
5. **opaque handles are for objects, explicit blobs/views are for data**

## Top 3 Public Data-Boundary Strategies

### 1. `Opaque handles + borrowed views in + Rust-owned blobs out`

`🎯 10   🛡️ 9   🧠 8`  
Примерно `7000-14000` строк.

Что это значит:

- long-lived things use handles:
  - sessions
  - subscriptions
  - searches
  - exports
- inbound data from host comes as borrowed view:
  - pointer + length
  - validated string view
  - typed small value structs
- outbound bulk data from Rust comes as Rust-owned blob with explicit destructor

Почему это strongest path:

- ownership is obvious
- allocator responsibility stays clear
- works across C ABI, Node adapter and generated SDKs
- fits terminal workloads where many large payloads are one-way snapshots/deltas

Практический вывод:

✅ Это лучший default.

### 2. `Generated SDK carrier types` like `RustBuffer` / `DiplomatSlice` / `OwnedSlice`

`🎯 8   🛡️ 8   🧠 7`  
Примерно `5000-10000` строк.

Что это значит:

- generator/runtime crate provides carrier types
- bindings know how to lift/lower them
- host often sees a higher-level language-native value

Почему это хорошо:

- reduces manual glue
- can be ergonomic for Swift/Python/Kotlin/C++

Где риск:

- carrier semantics become generator-specific
- if this layer becomes the main truth, your package gets coupled to one FFI framework

Практический вывод:

✅ Хороший adapter-layer pattern.  
⚠️ Не должен заменять deliberate core ownership rules.

### 3. `Raw pointers and C strings everywhere`

`🎯 3   🛡️ 4   🧠 4`  
Примерно `2000-6000` строк at the start and then expensive cleanup.

Что это значит:

- strings everywhere as `char *`
- buffers as `void * + len`
- ownership mainly in docs and human discipline

Почему это плохо:

- impossible to express nuanced ownership and mutability
- bad fit for UTF-8 and structured payloads
- easy allocator mismatch
- hard to auto-generate safe SDKs above it

Практический вывод:

❌ Для reusable terminal platform я бы это не брал.

## Tool-by-tool findings

## 1. `ffi-support::ByteBuffer` - strongest warning about allocator boundaries

- Crate: [`ffi-support`](https://crates.io/crates/ffi-support)
- Latest: `0.4.4`

Что особенно важно из docs:

- `ByteBuffer` does **not** implement `Drop`
- docs explicitly say you should expose a destructor via `define_bytebuffer_destructor!`
- docs explicitly warn that destroying a buffer allocated by another allocator or another Rust shared object is fundamentally broken

Это очень важный сигнал для нашей архитектуры:

🔥 **one Rust shared library should only free buffers it allocated itself**

Практический вывод:

- every outbound Rust-owned byte blob needs a destructor from the same library
- host code should never "guess" it can free Rust memory directly

## 2. `ffi-support::HandleMap` - a good donor for object identity, not bulk data

Что особенно важно:

- stable handles convertible to/from 64-bit integers
- good for long-lived objects across FFI

Это усиливает правильный shape:

- objects via handles
- blobs/views via explicit buffer types

Не надо пихать snapshots or transcripts into fake object handles just because handle maps exist.

## 3. `UniFFI::RustBuffer` - useful proof for Rust-owned outbound blobs

Что особенно важно из docs:

- `RustBuffer` is for passing Rust-allocated bytes over FFI
- docs explicitly say it must only be constructed from a Rust `Vec<u8>`
- docs explicitly say foreign code must not invent its own RustBuffer pointing at foreign memory
- docs mention `ForeignBytes` for foreign-owned memory views

Это очень сильный architectural signal:

🔥 **foreign-owned and Rust-owned bytes must not pretend to be the same type**

Практический вывод:

- we should likely mirror this rule in our own public contract
- outbound snapshots/deltas/transcript chunks can be Rust-owned blob types
- inbound payloads should come as foreign-owned views, not fake Rust-owned buffers

## 4. UniFFI lifting/lowering - strong proof that rich values usually collapse to byte blobs

Docs explicitly note that many non-trivial types like strings, optionals and records are lowered to `RustBuffer`.

Что это значит для нас:

- even mature binding generators often fall back to byte buffers for rich payload transport
- this is another reason to keep binary/control/data lanes explicit

Практический вывод:

- for large structured terminal payloads, a stable blob/projection format is healthy
- but do not let generator-specific blobs become the only truth of your public API

## 5. `Diplomat runtime` - good donor for safe slices and string slices

- Crate: [`diplomat-runtime`](https://crates.io/crates/diplomat-runtime)
- Latest: `0.15.1`

Что особенно важно:

- provides FFI-safe versions of slices and strings:
  - `DiplomatSlice`
  - `DiplomatOwnedSlice`
  - `DiplomatUtf8StrSlice`
  - `DiplomatOwnedUtf8StrSlice`
- also exposes FFI-safe result/option-like carrier types

Почему это полезно:

- good proof that pointer+len string and slice carriers are often healthier than universal `char *`
- very relevant for host-neutral config fields, titles, cwd, search matches and transcript fragments

Практический вывод:

✅ Very strong donor for public data carrier design.

## 6. `safer_ffi` - strongest donor for explicit pointer semantics

Что особенно важно из guide/rustdoc:

- framework distinguishes owned pointer, borrowed pointer and string shapes explicitly
- has `char_p` for C-compatible strings
- has `repr_c::String` and `repr_c::Vec<T>` for repr(C) Rust-owned carriers
- guide shows that an owned pointer and a borrowed pointer should not be the same type

Это очень полезно для нашей package architecture:

- API surface can encode meaning at the type layer
- not every string has to be `char *`
- not every buffer has to be an untyped raw pointer

Практический вывод:

✅ One of the best design donors for explicit ownership modelling.

## The key architectural rules

## 1. Inbound and outbound bytes must be different types

Bad shape:

- one `Buffer` type for everything

Better shape:

- `ForeignBytesView` for inbound borrowed host memory
- `RustOwnedBlob` for outbound bytes allocated by Rust

This avoids the most dangerous ownership ambiguity immediately.

## 2. Destructors belong to the allocating library

If a Rust dynamic library allocates:

- string
- byte buffer
- snapshot blob
- transcript export

then that same dynamic library must expose the destructor for it.

Do not assume:

- libc free is okay
- another Rust cdylib can free it
- the host runtime can guess the allocator

## 3. Strings need more than one shape

There is no single best universal string carrier.

Useful split:

- `char *` / null-terminated UTF-8 only where C ergonomics matters and interior NUL is unacceptable
- `ptr + len` UTF-8 slices for general host-neutral string views
- Rust-owned repr(C) string/blob types for outbound returned values

For terminal package this matters because:

- file paths
- cwd
- titles
- shell integration payloads
- search hits
- diagnostics

do not all have the same lifetime or encoding needs.

## 4. Bulk data should be flat and self-contained

For projections like:

- screen snapshot
- search result block
- transcript export chunk

prefer:

- flat blob
- flat slice
- flat struct-of-slices

Avoid:

- nested pointer graphs
- borrowing internal runtime memory into the host
- host retaining references to emulator-owned cells

## 5. Opaque handles are for identity, not payload

Use handles for:

- sessions
- panes
- subscriptions
- operations

Do not use handles just to avoid designing buffer semantics.

Payloads and identities are different things.

## 6. Public SDKs should hide low-level carriers where possible

At the lowest layer:

- C ABI may see handles, slices and blobs

At higher adapters:

- Node adapter may convert to `Buffer`, `Uint8Array`, `string`
- Swift adapter may convert to `Data` or `String`
- Python adapter may convert to `bytes` or `str`

But these are adapter presentations, not core truth.

## Recommended shape for this project

```text
terminal-core
  owns sessions, transcript, replay, snapshots

terminal-protocol
  owns handles, view/blob carriers, projections

terminal-capi
  exposes:
    - opaque handles
    - foreign byte/string views for inbound data
    - Rust-owned blob/string destructors for outbound data

terminal-node
  maps:
    - Rust blobs -> Buffer / Uint8Array
    - UTF-8 views -> string
    - handles -> JS object wrappers or opaque IDs
```

## What I would do in v1

1. Define explicit `ForeignBytesView` and `ForeignUtf8View`.
2. Define explicit `RustOwnedBlob` and `RustOwnedUtf8`.
3. Expose destructors only from the allocating library.
4. Keep snapshots/deltas as flat blobs or flat projections.
5. Keep object identity strictly handle-based.
6. Let adapters lift carriers into host-native types.

## What I would avoid in v1

- ❌ One universal buffer type for inbound and outbound data
- ❌ Assuming a host can free Rust memory with its own allocator
- ❌ Returning borrowed pointers into emulator/runtime internals
- ❌ Modelling every string as `char *`
- ❌ Hiding ownership rules only in prose without type distinctions

## Final take

Если сжать всё в одно правило:

🔥 **a world-class embeddable terminal package should make ownership visible in its types, not only in its documentation**

А значит:

- Rust-owned outbound data needs Rust-owned carriers and destructors
- foreign-owned inbound data needs borrowed view types
- strings need multiple deliberate carrier shapes
- opaque handles should represent identity, not leaked memory layout
