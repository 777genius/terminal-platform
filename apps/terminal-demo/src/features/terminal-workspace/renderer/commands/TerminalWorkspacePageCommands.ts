export interface TerminalWorkspaceCreateSessionCommands {
  setTitle(value: string): void;
  setProgram(value: string): void;
  setArgs(value: string): void;
  setCwd(value: string): void;
  submit(): Promise<void>;
}

export interface TerminalWorkspaceSessionCommands {
  select(sessionId: string): Promise<void>;
  refreshCatalog(): Promise<void>;
}

export interface TerminalWorkspaceDiscoveredSessionCommands {
  importSession(importHandle: string): Promise<void>;
}

export interface TerminalWorkspaceSavedSessionCommands {
  restore(sessionId: string): Promise<void>;
  delete(sessionId: string): Promise<void>;
  toggleVisibility(): void;
}

export interface TerminalWorkspaceTopologyCommands {
  newTab(): Promise<void>;
  splitHorizontal(): Promise<void>;
  splitVertical(): Promise<void>;
  saveSession(): Promise<void>;
  focusPane(paneId: string): Promise<void>;
  focusTab(tabId: string): Promise<void>;
}

export interface TerminalWorkspaceInputCommands {
  setDraft(value: string): void;
  submit(): Promise<void>;
  sendInterrupt(): Promise<void>;
  recallHistory(): Promise<void>;
  sendEnter(): Promise<void>;
}

export interface TerminalWorkspacePageCommands {
  createSession: TerminalWorkspaceCreateSessionCommands;
  sessions: TerminalWorkspaceSessionCommands;
  discoveredSessions: TerminalWorkspaceDiscoveredSessionCommands;
  savedSessions: TerminalWorkspaceSavedSessionCommands;
  topology: TerminalWorkspaceTopologyCommands;
  input: TerminalWorkspaceInputCommands;
}
