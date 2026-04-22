export interface TerminalWorkspaceCatalogCommands {
  setTitle(value: string): void;
  setProgram(value: string): void;
  setArgs(value: string): void;
  setCwd(value: string): void;
  submitCreate(): Promise<void>;
  selectSession(sessionId: string): Promise<void>;
  importSession(importHandle: string): Promise<void>;
}
