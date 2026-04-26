# Keyboard And Focus Behavior

**Checked**: 2026-04-27  
**Status**: implemented baseline

This document freezes the keyboard and focus contract for the public UI SDK elements.

Keyboard behavior is public API. Host apps can restyle elements, but they should not need to rediscover basic terminal navigation, focus recovery, or command submission rules from implementation details.

## Global Rules

- Native controls stay native where possible.
- Focus-visible affordances must remain visible in every theme.
- Terminal focus changes flow through `WorkspaceKernel` commands, not private DOM state.
- Web components may adapt DOM events, but command decisions stay in pure resolver functions when the behavior can be tested without a browser.
- Disabled or degraded capabilities must remove actions from the tab order.

## Terminal Workspace

`tp-terminal-workspace` owns the main layout composition:

- session list and saved layouts remain outside the terminal column
- terminal tab strip renders above terminal output
- command dock renders directly below terminal output
- `auto-focus-command-input` delegates focus to the command dock after workspace updates
- `inspector-mode="inline"` keeps topology tools beside the terminal
- `inspector-mode="collapsed"` keeps topology tools available behind a drawer for terminal-first views
- `inspector-mode="hidden"` lets host apps own topology controls elsewhere

The workspace element must not define canonical DTOs or backend-native references. It only composes SDK elements around `WorkspaceKernel`.

## Terminal Tab Strip

`tp-terminal-tab-strip` is the keyboard contract for session tabs:

- the tab list uses `role="tablist"`
- tab buttons use `role="tab"` and expose `aria-selected`
- active/focusable tabs use roving `tabindex="0"`
- inactive tabs stay programmatically reachable but are removed from sequential tab order
- disabled tab focus capability removes tab buttons from the tab order
- active close controls are focusable only when closing is supported

Supported keys on a focused tab or active close control:

| Key | Behavior |
| --- | --- |
| `ArrowLeft` / `ArrowUp` | Focus previous focusable tab, wrapping at the start |
| `ArrowRight` / `ArrowDown` | Focus next focusable tab, wrapping at the end |
| `Home` | Focus first focusable tab |
| `End` | Focus last focusable tab |
| `Delete` / `Backspace` | Start or confirm close for the focused tab when close is supported |
| `Enter` / `Space` on close button | Use the native button click path |

Close remains a two-step destructive action. Keyboard close uses the same confirmation state as pointer close.

After a keyboard focus command, DOM focus is restored to the newly active tab button after the snapshot update. This keeps screen-reader and keyboard users anchored while topology state remains owned by the runtime.

The pure contract lives in `resolveTerminalTabStripKeyboardIntent`.

## Terminal Screen

`tp-terminal-screen` owns output, output search, and direct focused-pane input:

- the viewport is focusable only when direct focused-pane input is available
- `Control+F` and `Meta+F` move focus to output search
- printable keys and terminal navigation keys are translated to terminal input only while the viewport is focused
- browser/system shortcuts using `Meta` or `Alt` are not stolen
- search `Enter` moves to the next match
- search `Shift+Enter` moves to the previous match
- search `Escape` clears search and restores focus to the viewport

The screen element must keep input capability status visible and announced with polite live updates.

## Command Dock And Composer

`tp-terminal-command-dock` adapts command composer events to `WorkspaceKernel` input commands.

`tp-terminal-command-composer` owns text entry:

- `Enter` submits the current draft when `Shift` is not held
- `Shift+Enter` remains textarea newline behavior
- `ArrowUp` and `ArrowDown` request command history navigation only when the text cursor is on the first or last logical line
- quick command and history actions refocus the input after they update the draft
- action buttons expose only real UI shortcuts through `aria-keyshortcuts`

The composer emits typed custom events. React wrappers map those events without duplicating business logic.

## Verification

Current automated coverage:

- unit tests for tab strip presentation state
- unit tests for tab strip keyboard intent resolution
- unit tests for command composer action and layout contracts
- public API tests for exported keyboard resolver types
- static renderer contract checks for terminal layout and tab strip keyboard markers
- browser smoke checks for tab strip pointer close and keyboard left/right navigation

Before release, this baseline should be extended with packed-package consumer smoke and a cross-browser keyboard matrix.
