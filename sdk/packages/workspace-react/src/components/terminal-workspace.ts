import * as React from "react";
import { createComponent } from "@lit/react";

import {
  TerminalCommandDockElement,
  TerminalPaneTreeElement,
  TerminalSavedSessionsElement,
  TerminalScreenElement,
  TerminalSessionListElement,
  TerminalStatusBarElement,
  TerminalToolbarElement,
  TerminalWorkspaceElement,
  defineTerminalPlatformElements,
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
