# Workspace Command History Model

**Checked**: 2026-04-25  
**Status**: implemented SDK behavior

## Goal

Command history is shared workspace state, not private element state. A host can observe it through the headless kernel, while SDK elements can render ergonomic recall controls without inventing their own storage.

## Ownership

- `workspace-core` owns the history read model and mutation rules
- `workspace-elements` renders history affordances from `snapshot.commandHistory`
- adapters do not send command history over transport
- runtime DTOs and backend-native refs never appear in history entries

## Public Core API

`WorkspaceSnapshot.commandHistory` exposes:

- `entries`: normalized command strings, oldest to newest
- `limit`: active maximum entry count

`WorkspaceSelectors.commandHistory()` returns the same read model.

`WorkspaceCommands` exposes:

- `recordCommandHistory(value)`
- `clearCommandHistory()`

`CreateWorkspaceKernelOptions.commandHistoryLimit` can override the default in-memory limit. Invalid, zero, negative, or non-finite values fall back to `DEFAULT_COMMAND_HISTORY_LIMIT`.

## Normalization

Recording a command:

- rejects whitespace-only input
- trims trailing whitespace
- keeps leading whitespace intact
- de-duplicates exact entries
- keeps the newest entries within `limit`

## Element Behavior

`tp-terminal-command-dock` uses the core history model for:

- `ArrowUp` and `ArrowDown` draft navigation
- visible recent-command recall buttons, newest first
- a history count badge
- a `Clear history` action in Session tools

The clear action dispatches `tp-terminal-command-history-cleared` as a bubbling, composed `CustomEvent` with no transport payload.

## Persistence Policy

History is in-memory by default. Persistent history is host-owned because shell commands can contain secrets, local paths, tokens, and customer data. A host that persists history should do so through an explicit product policy and should keep the persisted format separate from backend-native runtime state.

## Verification

Required coverage:

- core unit tests for normalization, de-duplication, limits, and clearing
- browser smoke coverage for command submission, recent recall, keyboard recall, badge sync, and clearing
- visual browser check for layout and overflow
