# V1 Verification And Acceptance Plan

**Проверено**: 2026-04-19
**Статус**: verification source of truth

## Why this exists

Для terminal platform нельзя ограничиться фразой "потом потестим".

Нужен отдельный план того:

- что валидируется автоматически
- что валидируется руками
- какие regression classes must never go unnoticed

## Verification layers

### 1. Unit and domain tests

Проверяют:

- DTO invariants
- mapping rules
- degraded-mode reasoning
- route binding rules
- capability merge rules

### 2. Property tests

Проверяют:

- topology invariants
- split tree validity
- ID mapping consistency
- protocol envelope roundtrip
- projection source invariants

### 3. Snapshot tests

Проверяют:

- `TopologySnapshot`
- `ScreenSnapshot`
- protocol envelopes
- host-facing error payloads

### 4. Integration tests

Проверяют:

- daemon handshake
- session lifecycle
- attach/detach
- subscription open/close
- native command dispatch

### 5. PTY interaction tests

Проверяют:

- shell start
- write input
- resize
- snapshot after interaction
- basic TUI handling

### 6. Adapter smoke suites

Проверяют:

- tmux import
- zellij import
- route-local mappings
- degraded responses on unsupported actions

### 7. Fuzzing

Проверяют:

- parser edges
- protocol decoder
- replay/snapshot merge code
- input encoding boundaries

## Recommended tools

- `proptest`
- `insta`
- `cargo-fuzz`
- `nextest`
- `expectrl`
- optional later `loom`

## Proposed suite layout from day one

Чтобы verification не превратилась в хаос, layout стоит зафиксировать сразу:

```text
terminal-platform/
  crates/
    terminal-protocol/tests/
    terminal-domain/tests/
    terminal-projection/tests/
    terminal-backend-native/tests/
    terminal-backend-tmux/tests/
    terminal-backend-zellij/tests/
    terminal-testing/
      fixtures/
      golden/
      smoke/
      manual/
  fuzz/
    fuzz_targets/
```

### Purpose of each area

- `terminal-protocol/tests` - handshake, envelope, version skew fixtures
- `terminal-domain/tests` - invariants, ID and route rules
- `terminal-projection/tests` - snapshot and delta golden tests
- backend crate tests - backend-local smoke and mapping behavior
- `terminal-testing/fixtures` - shared input corpora
- `terminal-testing/golden` - shared canonical expected outputs
- `terminal-testing/smoke` - end-to-end helper harnesses
- `terminal-testing/manual` - captured manual QA scripts and checklists
- `fuzz/fuzz_targets` - decoder, parser, replay and merge hot paths

## What must be fully automated

- daemon handshake correctness
- protocol version checks
- canonical DTO snapshots
- native session lifecycle
- route mapping persistence
- backend capability negotiation
- degraded-mode envelopes
- adapter smoke create/list/attach flows

## What should be partly automated, partly manual

- resize semantics
- alt-screen transitions
- multi-client focus behavior
- rendered snapshot correctness on real TUIs

## What must stay manual in v1

- feel of split/resize/focus UX
- behavior of real TUIs:
  - `vim`
  - `less`
  - `htop`
  - `fzf`
  - `lazygit`
- Electron embed behavior
- clipboard/open-link/notification integration feel
- platform-specific weirdness

## Test matrix

### Native backend

- shell start
- send text
- send paste
- resize
- split
- new tab
- focus pane
- topology snapshot
- screen snapshot
- shutdown

### `tmux` backend

- list sessions
- import session
- observe raw output
- capture screen snapshot
- route-local window binding
- degraded action on unsupported semantics
- resize-authority policy behavior

### `Zellij` backend

- list sessions
- import session
- JSON topology snapshot
- subscribe rendered viewport
- dump screen
- preserve typed pane refs
- preserve `tab_id` and `position`
- ordered mutation lane behavior

## Acceptance checklist for each milestone

### After Milestone 1

- contracts compile
- no backend refs leaked publicly
- naming is stable enough to write fixtures

### After Milestone 2

- handshake stable
- daemon connect/list path stable
- failure envelopes readable

### After Milestone 4

- native shell launches reliably
- screen snapshot works after input
- resize does not corrupt session

### After Milestone 8

- tmux import works on at least one real session
- raw output and snapshot semantics are both understood
- no false promise of independent focus parity

### After Milestone 9

- Zellij import works on at least one real session
- viewport subscribe and dump-screen semantics are both understood
- plugin panes are not mistaken for native panes

## Manual QA checklist

### Native

- open shell
- split horizontally
- split vertically
- open new tab
- run `vim`
- run `less`
- run `htop`
- resize host window several times
- detach and reattach

### `tmux`

- import attached session
- observe output while shell runs
- switch tmux window externally and confirm imported topology updates as expected
- verify resize policy does not accidentally hijack size authority

### `Zellij`

- import session with multiple tabs
- confirm `tab_id` and position are handled correctly
- observe rendered subscribe stream
- confirm plugin pane does not masquerade as native terminal pane

## Release gates before calling v1 usable

- snapshot suite green
- integration suite green
- fuzz baseline green
- adapter smoke suites green
- one manual QA pass on actual Electron host

## Merge gates by phase

### During bootstrap and contract freeze

- fmt green
- clippy green
- nextest green
- protocol fixtures updated if envelopes changed

### During native runtime milestones

- all above
- native snapshot golden suite green
- native PTY smoke suite green

### During `tmux` and `Zellij` milestones

- all above
- backend-specific smoke suite green
- explicit degraded-mode assertions green
- at least one recorded manual QA pass for imported route

## Bug taxonomy to use from day one

- `protocol`
- `native-runtime`
- `projection`
- `daemon`
- `tmux-adapter`
- `zellij-adapter`
- `host-node`
- `host-electron`
- `manual-qa`

## Manual findings policy

Каждый manual bug должен получить один из трёх исходов:

1. fixed and covered by automated regression
2. accepted as explicit degraded behavior and documented
3. postponed with named risk and milestone tag

Не должно быть четвёртого исхода в стиле "запомним потом".

## Main rule

🔥 **Every weird manual finding should become a new automated regression artifact when possible.**
