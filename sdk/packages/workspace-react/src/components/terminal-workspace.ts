import * as React from "react";
import { createComponent, type EventName } from "@lit/react";

import {
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TerminalCommandComposerElement,
  TerminalCommandDockElement,
  TerminalPaneTreeElement,
  TerminalSavedSessionsElement,
  TerminalScreenElement,
  TerminalSessionListElement,
  TerminalStatusBarElement,
  TerminalToolbarElement,
  TerminalWorkspaceElement,
  defineTerminalPlatformElements,
  type TerminalCommandComposerDraftChangeDetail,
  type TerminalCommandComposerHistoryNavigateDetail,
  type TerminalCommandComposerShortcutDetail,
} from "@terminal-platform/workspace-elements";

defineTerminalPlatformElements();

export const TerminalWorkspace = createComponent({
  react: React,
  tagName: "tp-terminal-workspace",
  elementClass: TerminalWorkspaceElement,
  displayName: "TerminalWorkspace",
});

export const TerminalStatusBar = createComponent({
  react: React,
  tagName: "tp-terminal-status-bar",
  elementClass: TerminalStatusBarElement,
  displayName: "TerminalStatusBar",
});

export const TerminalCommandDock = createComponent({
  react: React,
  tagName: "tp-terminal-command-dock",
  elementClass: TerminalCommandDockElement,
  displayName: "TerminalCommandDock",
});

export const TerminalCommandComposer = createComponent({
  react: React,
  tagName: "tp-terminal-command-composer",
  elementClass: TerminalCommandComposerElement,
  events: {
    onCommandDraftChange: TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange as EventName<
      CustomEvent<TerminalCommandComposerDraftChangeDetail>
    >,
    onCommandHistoryNavigate: TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate as EventName<
      CustomEvent<TerminalCommandComposerHistoryNavigateDetail>
    >,
    onCommandPaste: TERMINAL_COMMAND_COMPOSER_EVENTS.paste as EventName<CustomEvent<void>>,
    onCommandShortcut: TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut as EventName<
      CustomEvent<TerminalCommandComposerShortcutDetail>
    >,
    onCommandSubmit: TERMINAL_COMMAND_COMPOSER_EVENTS.submit as EventName<CustomEvent<void>>,
  },
  displayName: "TerminalCommandComposer",
});

export const TerminalSessionList = createComponent({
  react: React,
  tagName: "tp-terminal-session-list",
  elementClass: TerminalSessionListElement,
  displayName: "TerminalSessionList",
});

export const TerminalSavedSessions = createComponent({
  react: React,
  tagName: "tp-terminal-saved-sessions",
  elementClass: TerminalSavedSessionsElement,
  displayName: "TerminalSavedSessions",
});

export const TerminalScreen = createComponent({
  react: React,
  tagName: "tp-terminal-screen",
  elementClass: TerminalScreenElement,
  displayName: "TerminalScreen",
});

export const TerminalPaneTree = createComponent({
  react: React,
  tagName: "tp-terminal-pane-tree",
  elementClass: TerminalPaneTreeElement,
  displayName: "TerminalPaneTree",
});

export const TerminalToolbar = createComponent({
  react: React,
  tagName: "tp-terminal-toolbar",
  elementClass: TerminalToolbarElement,
  displayName: "TerminalToolbar",
});
