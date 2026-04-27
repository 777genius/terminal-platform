import type {
  TerminalCommandHistoryInputState,
  TerminalCommandHistoryNavigationDirection,
} from "./terminal-command-history-navigation.js";
import type { TerminalCommandComposerShortcut } from "./terminal-command-composer-actions.js";

export type { TerminalCommandComposerShortcut } from "./terminal-command-composer-actions.js";

export const TERMINAL_COMMAND_COMPOSER_EVENTS = {
  draftChange: "tp-terminal-command-draft-change",
  historyNavigate: "tp-terminal-command-history-navigate",
  paste: "tp-terminal-command-paste",
  shortcut: "tp-terminal-command-shortcut",
  submit: "tp-terminal-command-submit",
} as const;

export type TerminalCommandComposerDraftChangeDetail = {
  value: string;
};

export type TerminalCommandComposerHistoryNavigateDetail = {
  direction: TerminalCommandHistoryNavigationDirection;
  input: TerminalCommandHistoryInputState;
};

export type TerminalCommandComposerShortcutDetail = {
  data: TerminalCommandComposerShortcut;
};

export type TerminalCommandComposerEventType =
  (typeof TERMINAL_COMMAND_COMPOSER_EVENTS)[keyof typeof TERMINAL_COMMAND_COMPOSER_EVENTS];

export type TerminalCommandComposerEventMap = {
  [TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange]: CustomEvent<TerminalCommandComposerDraftChangeDetail>;
  [TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate]: CustomEvent<TerminalCommandComposerHistoryNavigateDetail>;
  [TERMINAL_COMMAND_COMPOSER_EVENTS.paste]: CustomEvent<void>;
  [TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut]: CustomEvent<TerminalCommandComposerShortcutDetail>;
  [TERMINAL_COMMAND_COMPOSER_EVENTS.submit]: CustomEvent<void>;
};
