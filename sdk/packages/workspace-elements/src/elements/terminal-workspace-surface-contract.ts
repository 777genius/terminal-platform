export const TERMINAL_WORKSPACE_SLOTS = {
  commandDock: "command-dock",
  inspector: "inspector",
  navigation: "navigation",
  screen: "screen",
  statusBar: "status-bar",
  tabStrip: "tab-strip",
} as const;

export const TERMINAL_WORKSPACE_PARTS = {
  body: "body",
  commandRegion: "command-region",
  content: "content",
  diagnostics: "diagnostics",
  diagnosticsStack: "diagnostics-stack",
  inspectorColumn: "inspector-column",
  inspectorDrawer: "inspector-drawer",
  navigationDrawer: "navigation-drawer",
  operationsDeck: "operations-deck",
  secondarySummary: "secondary-summary",
  sidebar: "sidebar",
  terminalColumn: "terminal-column",
  workspace: "workspace",
} as const;

export type TerminalWorkspaceSlotName =
  (typeof TERMINAL_WORKSPACE_SLOTS)[keyof typeof TERMINAL_WORKSPACE_SLOTS];

export type TerminalWorkspacePartName =
  (typeof TERMINAL_WORKSPACE_PARTS)[keyof typeof TERMINAL_WORKSPACE_PARTS];
