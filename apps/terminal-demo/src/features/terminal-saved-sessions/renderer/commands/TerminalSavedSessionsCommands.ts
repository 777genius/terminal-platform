export interface TerminalSavedSessionsCommands {
  restore(sessionId: string): Promise<void>;
  delete(sessionId: string): Promise<void>;
  toggleVisibility(): void;
}
