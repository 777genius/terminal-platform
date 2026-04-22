export interface TerminalInputComposerCommands {
  setDraft(value: string): void;
  submit(): Promise<void>;
  sendInterrupt(): Promise<void>;
  recallHistory(): Promise<void>;
  sendEnter(): Promise<void>;
}
