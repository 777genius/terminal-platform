export const TERMINAL_SCREEN_EVENTS = {
  copied: "tp-terminal-screen-copied",
  copyFailed: "tp-terminal-screen-copy-failed",
  inputSubmitted: "tp-terminal-screen-input-submitted",
  inputFailed: "tp-terminal-screen-input-failed",
  pasteSubmitted: "tp-terminal-screen-paste-submitted",
  pasteFailed: "tp-terminal-screen-paste-failed",
} as const;

export type TerminalScreenCopiedDetail = {
  paneId: string;
  lineCount: number;
};

export type TerminalScreenCopyFailedDetail = {
  paneId: string;
  error: unknown;
};

export type TerminalScreenInputSubmittedDetail = {
  sessionId: string;
  paneId: string;
  inputLength: number;
};

export type TerminalScreenInputFailedDetail = {
  sessionId: string;
  paneId: string;
  error: unknown;
};

export type TerminalScreenPasteSubmittedDetail = {
  sessionId: string;
  paneId: string;
  inputLength: number;
};

export type TerminalScreenPasteFailedDetail = {
  sessionId: string;
  paneId: string;
  error: unknown;
};

export type TerminalScreenEventType =
  (typeof TERMINAL_SCREEN_EVENTS)[keyof typeof TERMINAL_SCREEN_EVENTS];

export type TerminalScreenEventMap = {
  [TERMINAL_SCREEN_EVENTS.copied]: CustomEvent<TerminalScreenCopiedDetail>;
  [TERMINAL_SCREEN_EVENTS.copyFailed]: CustomEvent<TerminalScreenCopyFailedDetail>;
  [TERMINAL_SCREEN_EVENTS.inputSubmitted]: CustomEvent<TerminalScreenInputSubmittedDetail>;
  [TERMINAL_SCREEN_EVENTS.inputFailed]: CustomEvent<TerminalScreenInputFailedDetail>;
  [TERMINAL_SCREEN_EVENTS.pasteSubmitted]: CustomEvent<TerminalScreenPasteSubmittedDetail>;
  [TERMINAL_SCREEN_EVENTS.pasteFailed]: CustomEvent<TerminalScreenPasteFailedDetail>;
};
