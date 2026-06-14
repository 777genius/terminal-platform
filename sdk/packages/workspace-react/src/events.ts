import type { EventName } from "@lit/react";

import {
  TERMINAL_COMMAND_COMPOSER_EVENTS,
  TERMINAL_SCREEN_EVENTS,
  type TerminalCommandComposerEventMap,
  type TerminalScreenEventMap,
} from "@terminal-platform/workspace-elements";

export type TerminalReactEventHandler<EventType extends Event> = (
  event: EventType,
) => void;

export interface TerminalCommandComposerReactEventHandlers {
  onCommandAutocompleteAccept?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteAccept]
  >;
  onCommandAutocompleteDismiss?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteDismiss]
  >;
  onCommandDraftChange?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange]
  >;
  onCommandHistoryNavigate?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate]
  >;
  onCommandPaste?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.paste]
  >;
  onCommandShortcut?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut]
  >;
  onCommandSubmit?: TerminalReactEventHandler<
    TerminalCommandComposerEventMap[typeof TERMINAL_COMMAND_COMPOSER_EVENTS.submit]
  >;
}

export interface TerminalScreenReactEventHandlers {
  onScreenCopied?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.copied]
  >;
  onScreenCopyFailed?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.copyFailed]
  >;
  onScreenInputSubmitted?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.inputSubmitted]
  >;
  onScreenInputFailed?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.inputFailed]
  >;
  onScreenPasteSubmitted?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.pasteSubmitted]
  >;
  onScreenPasteFailed?: TerminalReactEventHandler<
    TerminalScreenEventMap[typeof TERMINAL_SCREEN_EVENTS.pasteFailed]
  >;
}

type TerminalCommandComposerReactEventNames = {
  [EventProp in keyof Required<TerminalCommandComposerReactEventHandlers>]: EventName<
    Parameters<
      Required<TerminalCommandComposerReactEventHandlers>[EventProp]
    >[0]
  >;
};

type TerminalScreenReactEventNames = {
  [EventProp in keyof Required<TerminalScreenReactEventHandlers>]: EventName<
    Parameters<Required<TerminalScreenReactEventHandlers>[EventProp]>[0]
  >;
};

export const terminalCommandComposerReactEvents = {
  onCommandAutocompleteAccept:
    TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteAccept,
  onCommandAutocompleteDismiss:
    TERMINAL_COMMAND_COMPOSER_EVENTS.autocompleteDismiss,
  onCommandDraftChange: TERMINAL_COMMAND_COMPOSER_EVENTS.draftChange,
  onCommandHistoryNavigate: TERMINAL_COMMAND_COMPOSER_EVENTS.historyNavigate,
  onCommandPaste: TERMINAL_COMMAND_COMPOSER_EVENTS.paste,
  onCommandShortcut: TERMINAL_COMMAND_COMPOSER_EVENTS.shortcut,
  onCommandSubmit: TERMINAL_COMMAND_COMPOSER_EVENTS.submit,
} as TerminalCommandComposerReactEventNames;

export const terminalScreenReactEvents = {
  onScreenCopied: TERMINAL_SCREEN_EVENTS.copied,
  onScreenCopyFailed: TERMINAL_SCREEN_EVENTS.copyFailed,
  onScreenInputSubmitted: TERMINAL_SCREEN_EVENTS.inputSubmitted,
  onScreenInputFailed: TERMINAL_SCREEN_EVENTS.inputFailed,
  onScreenPasteSubmitted: TERMINAL_SCREEN_EVENTS.pasteSubmitted,
  onScreenPasteFailed: TERMINAL_SCREEN_EVENTS.pasteFailed,
} as TerminalScreenReactEventNames;
