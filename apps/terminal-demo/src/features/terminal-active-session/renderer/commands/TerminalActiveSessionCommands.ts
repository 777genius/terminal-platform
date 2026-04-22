export interface TerminalActiveSessionCommands {
  refreshCatalog(): Promise<void>;
  newTab(): Promise<void>;
  splitHorizontal(): Promise<void>;
  splitVertical(): Promise<void>;
  saveSession(): Promise<void>;
  focusPane(paneId: string): Promise<void>;
  focusTab(tabId: string): Promise<void>;
}
