export { createWorkspaceKernel } from "./kernel/create-workspace-kernel.js";

export type {
  CreateWorkspaceKernelOptions,
  WorkspaceCommands,
  WorkspaceDiagnostics,
  WorkspaceKernel,
  WorkspaceSelectors,
} from "./kernel/types.js";

export {
  DEFAULT_TERMINAL_FONT_SCALE,
  DEFAULT_WORKSPACE_THEME_ID,
  createInitialWorkspaceSnapshot,
  terminalPlatformTerminalFontScales,
  terminalPlatformWorkspaceThemeIds,
  type CreateInitialWorkspaceSnapshotOptions,
  type TerminalPlatformTerminalFontScale,
  type TerminalPlatformWorkspaceThemeId,
  type WorkspaceCatalogSnapshot,
  type WorkspaceConnectionSnapshot,
  type WorkspaceConnectionState,
  type WorkspaceDiagnosticRecord,
  type WorkspaceDiagnosticSeverity,
  type WorkspaceSelectionSnapshot,
  type WorkspaceSnapshot,
  type WorkspaceTerminalDisplaySnapshot,
  type WorkspaceThemeSnapshot,
} from "./read-models/workspace-snapshot.js";

export {
  countTerminalOutputSearchMatches,
  countTerminalOutputSearchMatchesInLine,
  createTerminalOutputSearchResult,
  formatTerminalOutputSearchCount,
  normalizeTerminalOutputSearchQuery,
  resolveTerminalOutputSearchMatchIndex,
  serializeTerminalOutputLines,
  type TerminalOutputSearchLine,
  type TerminalOutputSearchMatchSegment,
  type TerminalOutputSearchOptions,
  type TerminalOutputSearchResult,
  type TerminalOutputSearchSegment,
  type TerminalOutputSearchTextSegment,
} from "./selectors/terminal-output-search.js";
