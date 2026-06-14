import { css, html, nothing } from "lit";
import type { PropertyValues, TemplateResult } from "lit";

import {
  createTerminalOutputSearchResult,
  formatTerminalOutputSearchCount,
  resolveTerminalOutputSearchMatchIndex,
  serializeTerminalOutputLines,
  type TerminalOutputSearchResult,
  type TerminalOutputSearchSegment,
} from "@terminal-platform/workspace-core";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { writeClipboardText } from "./terminal-clipboard.js";
import {
  shouldRefreshAfterTerminalDirectInput,
  TerminalDirectInputBuffer,
} from "./terminal-direct-input-buffer.js";
import {
  resolveTerminalScreenInputStatus,
  type TerminalScreenInputActivity,
} from "./terminal-screen-input-status.js";
import { terminalInputForKeyboardEvent } from "./terminal-keyboard-input.js";
import { resolveTerminalScreenControlState } from "./terminal-screen-controls.js";
import { isTerminalScreenSearchShortcut } from "./terminal-screen-shortcuts.js";
import {
  TERMINAL_SCREEN_SEARCH_ACTION_IDS,
  resolveTerminalScreenSearchActions,
  type TerminalScreenSearchActionId,
} from "./terminal-screen-search-actions.js";
import {
  resolveTerminalScreenChromeState,
  TERMINAL_SCREEN_CHROME_MODES,
  type TerminalScreenChromeMetaItem,
  type TerminalScreenChromeState,
} from "./terminal-screen-chrome.js";
import {
  resolveTerminalScreenActions,
  TERMINAL_SCREEN_ACTION_IDS,
  type TerminalScreenActionId,
  type TerminalScreenCopyState,
  type TerminalScreenHistoryLoadState,
} from "./terminal-screen-actions.js";
import {
  TERMINAL_SCREEN_EVENTS,
  type TerminalScreenCopiedDetail,
  type TerminalScreenCopyFailedDetail,
  type TerminalScreenInputFailedDetail,
  type TerminalScreenInputSubmittedDetail,
  type TerminalScreenPasteFailedDetail,
  type TerminalScreenPasteSubmittedDetail,
} from "./terminal-screen-events.js";

type TerminalScreenPlacement = "panel" | "terminal";
export type VisibleOutputLineSource = "history" | "boundary" | "live";

export interface VisibleOutputLine {
  text: string;
  source: VisibleOutputLineSource;
}

export interface VisibleOutputLineOptions {
  hideShellPromptNoise?: boolean;
  preserveShellPromptCommands?: boolean;
  terminalPromptLabel?: string;
}

export interface TerminalHistoryEntryOptions {
  terminalPromptLabel?: string;
}

export type TerminalCommandPresentationStatus =
  | "failed"
  | "running"
  | "succeeded"
  | "unknown";

export interface TerminalCommandPresentationMetadata {
  command: string;
  durationMs?: number | null;
  exitCode?: number | null;
  startedAtMs?: number | null;
  status?: TerminalCommandPresentationStatus | null;
}

export interface TerminalHistoryRenderOptions {
  activeCommandContextLineIndex?: number | null;
  commandMetadata?: readonly TerminalCommandPresentationMetadata[] | null;
  onCommandContextMenu?: (
    event: MouseEvent,
    entry: Extract<TerminalHistoryEntry, { kind: "command" }>,
  ) => void;
}

interface ShellPromptCommandLine {
  prompt: string;
  command: string;
}

interface TerminalCommandContextMenuState {
  blockText: string;
  commandLineIndex: number;
  commandText: string;
  outputText: string;
  x: number;
  y: number;
}

export interface TerminalHistoryCommandOutputLine {
  line: VisibleOutputLine;
  lineIndex: number;
}

export type TerminalHistoryEntry =
  | {
      kind: "line";
      line: VisibleOutputLine;
      lineIndex: number;
    }
  | {
      kind: "command";
      prompt: string;
      commandLine: VisibleOutputLine;
      commandLineIndex: number;
      command: string;
      output: TerminalHistoryCommandOutputLine[];
    };

const TERMINAL_SCREEN_SEARCH_COUNT_ID = "tp-screen-search-count";
const RESTORED_HISTORY_BOUNDARY_TEXT =
  "--- restored history above; live process below ---";
const RESTORED_HISTORY_PARTIAL_TEXT =
  "--- restored history is partial; more persisted output is available ---";
const HISTORY_AUTO_LOAD_TOP_THRESHOLD_PX = 24;

export class TerminalScreenElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    placement: { type: String },
    hideShellPromptNoise: {
      attribute: "hide-shell-prompt-noise",
      type: Boolean,
    },
    terminalPromptLabel: {
      attribute: "terminal-prompt-label",
      type: String,
    },
    commandPresentationMetadata: { attribute: false },
    followOutput: { state: true },
    searchQuery: { state: true },
    activeSearchMatchIndex: { state: true },
    copyState: { state: true },
    directInputActivity: { state: true },
    historyLoadState: { state: true },
    commandContextMenu: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .screen {
        display: grid;
        grid-template-rows: auto auto auto minmax(0, 1fr);
        gap: var(--tp-space-3);
        padding: var(--tp-terminal-screen-panel-padding, var(--tp-space-4));
        padding-bottom: var(
          --tp-terminal-screen-panel-padding-bottom,
          var(--tp-space-4)
        );
        border-top-left-radius: var(
          --tp-terminal-screen-panel-border-top-left-radius,
          var(--tp-radius-md)
        );
        border-top-right-radius: var(
          --tp-terminal-screen-panel-border-top-right-radius,
          var(--tp-radius-md)
        );
        border-bottom-left-radius: var(
          --tp-terminal-screen-panel-border-bottom-left-radius,
          var(--tp-radius-md)
        );
        border-bottom-right-radius: var(
          --tp-terminal-screen-panel-border-bottom-right-radius,
          var(--tp-radius-md)
        );
        box-shadow: var(
          --tp-terminal-screen-panel-shadow,
          var(--tp-shadow-panel)
        );
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-color-bg-inset) 92%, transparent),
            var(--tp-color-bg)
          ),
          var(--tp-color-bg);
        min-height: 18rem;
      }

      .screen[data-placement="terminal"] {
        gap: 0;
        grid-template-rows: auto minmax(0, 1fr);
        height: 100%;
        min-height: 0;
        color: var(--tp-terminal-color-text);
        background:
          linear-gradient(
            180deg,
            color-mix(
              in srgb,
              var(--tp-terminal-color-bg-raised) 92%,
              transparent
            ),
            var(--tp-terminal-color-bg)
          ),
          var(--tp-terminal-color-bg);
      }

      .screen-chrome {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(12rem, 0.48fr) auto;
        gap: 0.45rem;
        align-items: center;
        min-height: 2.45rem;
        min-width: 0;
        border: 1px solid
          color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-bottom-width: 0;
        border-radius: var(--tp-radius-md) var(--tp-radius-md) 0 0;
        background:
          linear-gradient(
            180deg,
            color-mix(
              in srgb,
              var(--tp-terminal-color-bg-raised) 88%,
              transparent
            ),
            color-mix(in srgb, var(--tp-terminal-color-bg) 96%, transparent)
          ),
          var(--tp-terminal-color-bg);
        padding: 0.42rem 0.65rem;
      }

      .screen-chrome[data-search-active="true"] {
        grid-template-columns: minmax(0, 1fr) minmax(10rem, 0.42fr) auto auto;
      }

      .screen-chrome__title {
        display: flex;
        align-items: center;
        gap: 0.48rem;
        min-width: 0;
      }

      .screen-chrome__tools {
        display: contents;
      }

      .screen-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: var(--tp-space-3);
      }

      .screen[data-placement="terminal"] .screen-header {
        align-items: center;
      }

      .screen-header .panel-header {
        margin-bottom: 0;
      }

      .screen[data-placement="terminal"] .panel-header {
        min-width: 0;
      }

      .screen[data-placement="terminal"] .panel-title {
        overflow: hidden;
        color: var(--tp-terminal-color-text);
        font-size: 0.96rem;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .screen-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: var(--tp-space-2);
      }

      .screen[data-placement="terminal"] .screen-actions {
        flex-wrap: nowrap;
        gap: 0.35rem;
      }

      .screen-actions button {
        white-space: nowrap;
      }

      .screen[data-placement="terminal"] .screen-actions button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 78%,
          transparent
        );
        border-radius: 0.45rem;
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 84%,
          transparent
        );
        color: var(--tp-terminal-color-text);
        font-size: 0.82rem;
        padding: 0.32rem 0.55rem;
      }

      .screen[data-placement="terminal"]
        .screen-actions
        button[data-screen-action-label-mode="glyph"] {
        inline-size: 2.25rem;
        min-width: 2.25rem;
        aspect-ratio: 1;
        padding: 0;
        font-family: var(--tp-font-family-mono);
        font-size: 0.9rem;
        line-height: 1;
      }

      .screen[data-placement="terminal"]
        .screen-actions
        button[data-screen-action-tone="primary"] {
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 54%,
          transparent
        );
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 16%,
          var(--tp-terminal-color-bg-raised)
        );
      }

      .screen-tools {
        display: grid;
        grid-template-columns: minmax(12rem, 1fr) auto;
        gap: var(--tp-space-2);
        align-items: center;
      }

      .screen[data-placement="terminal"] .screen-tools {
        grid-template-columns: minmax(11rem, 0.64fr) auto;
        gap: 0.35rem;
      }

      .screen-chrome .search {
        min-width: 0;
      }

      .screen-chrome .search-actions {
        flex-wrap: nowrap;
      }

      .search {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: var(--tp-space-2);
        align-items: center;
        min-width: 0;
      }

      .search input {
        min-width: 0;
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-sm);
        background: color-mix(in srgb, var(--tp-color-bg) 72%, transparent);
        color: var(--tp-color-text);
        font: inherit;
        padding: 0.48rem 0.65rem;
      }

      .screen[data-placement="terminal"] .search input {
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 78%,
          transparent
        );
        border-radius: 0.45rem;
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 84%,
          transparent
        );
        color: var(--tp-terminal-color-text);
        font-size: 0.84rem;
        padding: 0.34rem 0.55rem;
      }

      .screen[data-placement="terminal"] .search input::placeholder {
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text-muted) 72%,
          transparent
        );
      }

      .search input:focus-visible {
        outline: 2px solid
          color-mix(in srgb, var(--tp-color-accent) 62%, transparent);
        outline-offset: 2px;
      }

      .search input:disabled {
        cursor: not-allowed;
        opacity: 0.5;
      }

      .search-count {
        color: var(--tp-color-text-muted);
        font-size: 0.82rem;
        white-space: nowrap;
      }

      .screen[data-placement="terminal"] .search-count {
        color: var(--tp-terminal-color-text-muted);
      }

      .screen[data-placement="terminal"]
        .search-count[data-search-active="false"] {
        display: none;
      }

      .search-actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        justify-content: flex-end;
      }

      .screen[data-placement="terminal"] .search-actions {
        gap: 0.35rem;
      }

      .search-actions button {
        white-space: nowrap;
      }

      .screen[data-placement="terminal"] .search-actions button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 78%,
          transparent
        );
        border-radius: 0.45rem;
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 84%,
          transparent
        );
        color: var(--tp-terminal-color-text);
        font-size: 0.82rem;
        min-width: 2.15rem;
        padding: 0.34rem 0.55rem;
      }

      .screen[data-placement="terminal"]
        .search-actions
        button[data-screen-search-action-label-mode="glyph"] {
        inline-size: 2.2rem;
        min-width: 2.2rem;
        aspect-ratio: 1;
        padding: 0;
        font-family: var(--tp-font-family-mono);
        font-size: 0.92rem;
        line-height: 1;
      }

      .viewport {
        --tp-terminal-history-base-font-size: var(
          --tp-terminal-history-font-size,
          0.9rem
        );
        margin: 0;
        min-height: var(
          --tp-terminal-screen-viewport-min-height,
          clamp(18rem, 42vh, 34rem)
        );
        max-height: var(
          --tp-terminal-screen-viewport-max-height,
          min(58vh, 44rem)
        );
        overflow: auto;
        border: 1px solid
          color-mix(in srgb, var(--tp-color-border) 70%, transparent);
        border-radius: var(--tp-radius-lg);
        border-bottom-left-radius: var(
          --tp-terminal-screen-viewport-border-bottom-left-radius,
          var(--tp-radius-lg)
        );
        border-bottom-right-radius: var(
          --tp-terminal-screen-viewport-border-bottom-right-radius,
          var(--tp-radius-lg)
        );
        background: var(--tp-terminal-color-bg);
        color: var(--tp-terminal-color-text);
        padding: var(--tp-space-3);
        font-family: var(--tp-font-family-mono);
        font-size: var(--tp-terminal-history-font-size, 0.9rem);
        line-height: 1.48;
        scrollbar-gutter: stable;
      }

      .screen[data-placement="terminal"] .viewport {
        grid-row: 2;
        height: 100%;
        min-height: 0;
        max-height: none;
        align-self: stretch;
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 78%,
          transparent
        );
        border-top-width: 0;
        border-radius: 0;
        border-bottom-left-radius: var(
          --tp-terminal-screen-viewport-border-bottom-left-radius,
          0
        );
        border-bottom-right-radius: var(
          --tp-terminal-screen-viewport-border-bottom-right-radius,
          0
        );
        padding: 0;
        box-shadow: inset 0 1px 0
          color-mix(in srgb, var(--tp-terminal-color-accent) 18%, transparent);
      }

      .viewport:focus-visible {
        outline: 2px solid
          color-mix(in srgb, var(--tp-color-accent) 64%, transparent);
        outline-offset: 3px;
      }

      .screen[data-direct-input="true"] .viewport {
        cursor: text;
      }

      .screen[data-font-scale="compact"] .viewport {
        --tp-terminal-history-base-font-size: var(
          --tp-terminal-history-font-size,
          0.82rem
        );
        font-size: var(--tp-terminal-history-font-size, 0.82rem);
        line-height: 1.42;
      }

      .screen[data-font-scale="large"] .viewport {
        --tp-terminal-history-base-font-size: var(
          --tp-terminal-history-font-size,
          1rem
        );
        font-size: var(--tp-terminal-history-font-size, 1rem);
        line-height: 1.56;
      }

      .screen[data-line-wrap="false"] .text {
        white-space: pre;
        overflow-wrap: normal;
      }

      .line {
        display: grid;
        grid-template-columns: 3.25rem minmax(0, 1fr);
        gap: 0.72rem;
        min-height: 1.35rem;
      }

      .line[data-line-source="history"] {
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text) 82%,
          var(--tp-terminal-color-text-muted)
        );
      }

      .line[data-line-source="boundary"] {
        margin: 0.45rem 0;
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 72%,
          var(--tp-terminal-color-text)
        );
        font-size: 0.82em;
        text-transform: uppercase;
      }

      .line[data-line-source="boundary"] .gutter {
        color: transparent;
      }

      .gutter {
        border-right: 1px solid
          color-mix(in srgb, var(--tp-color-border) 42%, transparent);
        color: color-mix(in srgb, var(--tp-color-text-muted) 48%, transparent);
        font-variant-numeric: tabular-nums;
        padding-right: 0.55rem;
        text-align: right;
        user-select: none;
      }

      .screen[data-placement="terminal"] .gutter {
        border-right-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 46%,
          transparent
        );
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text-muted) 58%,
          transparent
        );
      }

      .screen[data-placement="terminal"] .line {
        grid-template-columns: minmax(0, 1fr);
        gap: 0;
        padding: 0.1rem 1.15rem;
      }

      .screen[data-placement="terminal"] .gutter {
        display: none;
      }

      .history-entry {
        display: grid;
        box-sizing: border-box;
        width: 100%;
        justify-items: start;
        gap: 0.18rem;
        border-top: 1px solid
          color-mix(in srgb, var(--tp-terminal-color-border) 46%, transparent);
        padding: 0.72rem 1.15rem 0.78rem;
        text-align: left;
      }

      .history-entry:first-child {
        border-top-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 42%,
          transparent
        );
      }

      .history-entry[data-line-source="history"] {
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text) 82%,
          var(--tp-terminal-color-text-muted)
        );
      }

      .history-entry[data-command-context-menu="true"] {
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 38%,
          transparent
        );
      }

      .history-entry-prompt {
        display: flex;
        flex-wrap: wrap;
        align-items: baseline;
        gap: 0.48rem;
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text-muted) 86%,
          transparent
        );
        font-size: 0.91em;
        line-height: 1.28;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
      }

      .history-entry-meta {
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text-muted) 78%,
          transparent
        );
        font-size: 0.9em;
        white-space: nowrap;
      }

      .history-entry-meta[data-command-status="failed"] {
        color: color-mix(
          in srgb,
          var(--tp-color-danger) 82%,
          var(--tp-terminal-color-text)
        );
      }

      .history-entry-meta[data-command-status="running"] {
        color: color-mix(
          in srgb,
          var(--tp-color-warning) 78%,
          var(--tp-terminal-color-text)
        );
      }

      .history-entry-command {
        --tp-history-entry-text-size: calc(
          var(--tp-terminal-history-base-font-size) * 1.04
        );
        color: var(--tp-terminal-color-text);
        font-size: 1.04em;
        font-weight: 760;
        line-height: 1.28;
      }

      .history-entry-output {
        --tp-history-entry-text-size: var(--tp-terminal-history-base-font-size);
        color: var(--tp-terminal-color-text);
        line-height: 1.34;
        margin-top: 0.06rem;
      }

      .command-context-menu {
        position: fixed;
        z-index: 1000;
        min-width: 13.5rem;
        overflow: hidden;
        border: 1px solid
          color-mix(in srgb, var(--tp-terminal-color-border) 52%, transparent);
        border-radius: 0.52rem;
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 96%,
          black 8%
        );
        box-shadow:
          0 18px 44px rgba(0, 0, 0, 0.42),
          inset 0 1px 0 rgba(255, 255, 255, 0.04);
        padding: 0.32rem;
      }

      .command-context-menu__item {
        display: grid;
        width: 100%;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        gap: 1.5rem;
        border: 0;
        border-radius: 0.36rem;
        background: transparent;
        color: var(--tp-terminal-color-text);
        cursor: default;
        font: inherit;
        font-family: var(--tp-font-family-sans);
        font-size: 0.92rem;
        line-height: 1.2;
        padding: 0.56rem 0.68rem;
        text-align: left;
      }

      .command-context-menu__item:hover,
      .command-context-menu__item:focus-visible {
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 18%,
          transparent
        );
        outline: none;
      }

      .command-context-menu__shortcut {
        color: color-mix(
          in srgb,
          var(--tp-terminal-color-text-muted) 78%,
          transparent
        );
        font-family: var(--tp-font-family-mono);
        font-size: 0.84em;
        white-space: nowrap;
      }

      .history-entry-text {
        display: block;
        font-size: 0;
        white-space: pre-wrap;
        overflow-wrap: anywhere;
      }

      .history-entry-text .terminal-output-segment {
        font-size: var(
          --tp-history-entry-text-size,
          var(--tp-terminal-history-base-font-size)
        );
        line-height: inherit;
      }

      .screen[data-line-wrap="false"] .history-entry-prompt,
      .screen[data-line-wrap="false"] .history-entry-text {
        white-space: pre;
        overflow-wrap: normal;
      }

      .text {
        white-space: pre-wrap;
        overflow-wrap: anywhere;
      }

      mark {
        -webkit-box-decoration-break: clone;
        box-decoration-break: clone;
        border-radius: 0.2rem;
        background: color-mix(
          in srgb,
          var(--tp-color-warning) 36%,
          transparent
        );
        color: var(--tp-color-text);
        line-height: inherit;
        padding: 0 0.08em;
      }

      mark[data-active="true"] {
        outline: 1px solid
          color-mix(in srgb, var(--tp-color-warning) 80%, transparent);
        background: color-mix(
          in srgb,
          var(--tp-color-warning) 58%,
          var(--tp-color-bg)
        );
      }

      .screen[data-placement="terminal"] mark {
        color: var(--tp-terminal-color-text);
      }

      .screen[data-placement="terminal"] mark[data-active="true"] {
        background: color-mix(
          in srgb,
          var(--tp-color-warning) 58%,
          var(--tp-terminal-color-bg)
        );
      }

      .screen-meta {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        color: var(--tp-color-text-muted);
        font-size: 0.82rem;
      }

      .screen-meta span {
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        padding: 0.2rem 0.5rem;
        background: color-mix(
          in srgb,
          var(--tp-color-panel-raised) 60%,
          transparent
        );
      }

      .screen[data-placement="terminal"] .screen-meta {
        color: var(--tp-terminal-color-text-muted);
        flex-wrap: nowrap;
        gap: 0.35rem;
        font-size: 0.78rem;
        min-width: 0;
        overflow: hidden;
      }

      .screen[data-placement="terminal"] .screen-meta span {
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-border) 72%,
          transparent
        );
        border-radius: 0.45rem;
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-bg-raised) 78%,
          transparent
        );
        min-width: 0;
        max-width: 10rem;
        overflow: hidden;
        padding: 0.18rem 0.45rem;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .screen-meta [data-input-tone="ready"] {
        border-color: color-mix(
          in srgb,
          var(--tp-color-success) 52%,
          transparent
        );
        color: var(--tp-color-success);
      }

      .screen-meta [data-input-tone="pending"] {
        border-color: color-mix(
          in srgb,
          var(--tp-color-warning) 56%,
          transparent
        );
        color: var(--tp-color-warning);
      }

      .screen-meta [data-input-tone="failed"] {
        border-color: color-mix(
          in srgb,
          var(--tp-color-danger) 62%,
          transparent
        );
        background: color-mix(
          in srgb,
          var(--tp-color-danger-soft) 70%,
          transparent
        );
        color: var(--tp-color-danger);
      }

      @media (max-width: 960px) {
        .screen[data-placement="terminal"] .screen-chrome,
        .screen[data-placement="terminal"]
          .screen-chrome[data-search-active="true"] {
          grid-template-columns: minmax(0, 1fr) auto auto;
        }

        .screen[data-placement="terminal"] .screen-chrome__title {
          grid-column: 1 / -1;
          overflow: hidden;
        }
      }

      @media (max-width: 720px) {
        .screen {
          gap: var(--tp-space-2);
          padding: var(
            --tp-terminal-screen-mobile-panel-padding,
            var(--tp-space-3)
          );
          padding-bottom: var(
            --tp-terminal-screen-panel-padding-bottom,
            var(--tp-space-3)
          );
        }

        .screen-header {
          display: grid;
        }

        .screen-tools {
          grid-template-columns: 1fr;
        }

        .screen[data-placement="terminal"] .screen-tools {
          grid-template-columns: 1fr;
        }

        .search {
          grid-template-columns: 1fr;
        }

        .search-actions {
          justify-content: flex-start;
        }

        .screen-actions {
          justify-content: flex-start;
        }

        .screen-chrome {
          grid-template-columns: minmax(0, 1fr) auto;
          align-items: center;
          min-height: 0;
        }

        .screen-chrome__title {
          grid-column: 1 / -1;
          overflow: hidden;
        }

        .screen-chrome .screen-actions {
          justify-content: flex-end;
          max-width: 100%;
          overflow-x: auto;
          scrollbar-width: none;
        }

        .viewport {
          min-height: var(
            --tp-terminal-screen-mobile-viewport-min-height,
            clamp(14rem, 38vh, 22rem)
          );
          max-height: var(
            --tp-terminal-screen-mobile-viewport-max-height,
            min(48vh, 26rem)
          );
          padding: var(--tp-space-2);
        }

        .screen[data-placement="terminal"] .viewport {
          min-height: var(--tp-terminal-screen-mobile-viewport-min-height, 0);
          max-height: none;
        }

        .line {
          grid-template-columns: 2.45rem minmax(0, 1fr);
          gap: 0.55rem;
        }

        .screen[data-placement="terminal"] .line {
          grid-template-columns: minmax(0, 1fr);
          gap: 0;
        }

        .gutter {
          padding-right: 0.38rem;
        }
      }
    `,
  ];

  declare placement: TerminalScreenPlacement;
  declare hideShellPromptNoise: boolean;
  declare terminalPromptLabel: string;
  declare commandPresentationMetadata:
    | readonly TerminalCommandPresentationMetadata[]
    | null
    | undefined;
  declare protected followOutput: boolean;
  declare protected searchQuery: string;
  declare protected activeSearchMatchIndex: number | null;
  declare protected copyState: TerminalScreenCopyState;
  declare protected directInputActivity: TerminalScreenInputActivity;
  declare protected historyLoadState: TerminalScreenHistoryLoadState;
  declare protected commandContextMenu: TerminalCommandContextMenuState | null;

  #autoScrolling = false;
  #copyStateResetTimer: ReturnType<typeof setTimeout> | null = null;
  #directInputActivityResetTimer: ReturnType<typeof setTimeout> | null = null;
  #historyLoadStateResetTimer: ReturnType<typeof setTimeout> | null = null;
  #directInputQueue = Promise.resolve();
  #directInputBuffer: TerminalDirectInputBuffer;

  constructor() {
    super();
    this.placement = "panel";
    this.hideShellPromptNoise = false;
    this.terminalPromptLabel = "shell";
    this.commandPresentationMetadata = null;
    this.followOutput = true;
    this.searchQuery = "";
    this.activeSearchMatchIndex = null;
    this.copyState = "idle";
    this.directInputActivity = "idle";
    this.historyLoadState = "idle";
    this.commandContextMenu = null;
    this.#directInputBuffer = new TerminalDirectInputBuffer({
      flush: (input) => this.queueDirectInput(input),
    });
  }

  override disconnectedCallback(): void {
    this.clearCopyStateResetTimer();
    this.clearDirectInputActivityResetTimer();
    this.clearHistoryLoadStateResetTimer();
    this.#directInputBuffer.dispose();
    super.disconnectedCallback();
  }

  scrollToLatestOutput(): void {
    this.scrollLatest();
  }

  protected override willUpdate(changedProperties: PropertyValues): void {
    super.willUpdate(changedProperties);
    if (changedProperties.has("snapshot")) {
      this.syncTerminalDisplayAttributes();
    }
  }

  protected override updated(changedProperties: PropertyValues): void {
    const shouldSyncSearch =
      changedProperties.has("snapshot") ||
      changedProperties.has("searchQuery") ||
      changedProperties.has("activeSearchMatchIndex");
    if (shouldSyncSearch && this.syncActiveSearchMatch()) {
      return;
    }

    if (
      changedProperties.has("snapshot") ||
      changedProperties.has("followOutput")
    ) {
      if (this.followOutput && this.snapshot.attachedSession?.focused_screen) {
        this.scrollViewportToBottom();
      }
    }
  }

  override render() {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    const screen = controls.screen;
    const inputStatus = resolveTerminalScreenInputStatus(
      controls,
      this.directInputActivity,
    );
    const isTerminalPlacement = this.placement === "terminal";
    const terminalPromptLabel = normalizeTerminalPromptLabel(
      this.terminalPromptLabel,
    );
    const outputLines = createVisibleOutputLines(controls.history, screen, {
      hideShellPromptNoise: this.hideShellPromptNoise || isTerminalPlacement,
      preserveShellPromptCommands: isTerminalPlacement,
      terminalPromptLabel,
    });
    const searchResult = this.createSearchResult(
      undefined,
      outputLines.map((line) => line.text),
    );
    const terminalHistoryEntries = isTerminalPlacement
      ? createTerminalHistoryEntries(outputLines, { terminalPromptLabel })
      : [];
    const terminalDisplay = this.snapshot.terminalDisplay;
    const chrome = screen
      ? resolveTerminalScreenChromeState(screen, terminalDisplay, {
          mode: isTerminalPlacement
            ? TERMINAL_SCREEN_CHROME_MODES.compact
            : TERMINAL_SCREEN_CHROME_MODES.full,
        })
      : null;

    return html`
      <div
        class="panel screen"
        part="screen"
        data-testid="tp-terminal-screen"
        data-placement=${this.placement}
        data-chrome-mode=${chrome?.mode ?? TERMINAL_SCREEN_CHROME_MODES.full}
        data-font-scale=${terminalDisplay.fontScale}
        data-line-wrap=${String(terminalDisplay.lineWrap)}
        data-direct-input=${String(controls.canUseDirectInput)}
        data-direct-paste=${String(controls.canUseDirectPaste)}
        data-input-capability=${controls.inputCapabilityStatus}
        data-input-status=${inputStatus.tone}
      >
        ${screen
          ? html`
              ${chrome
                ? isTerminalPlacement
                  ? this.renderCompactChrome(
                      chrome,
                      inputStatus,
                      searchResult,
                      controls,
                    )
                  : this.renderFullChrome(
                      chrome,
                      inputStatus,
                      searchResult,
                      controls,
                    )
                : nothing}
              <div
                class="viewport"
                part="screen-lines"
                data-testid="tp-screen-viewport"
                tabindex=${controls.canUseDirectInput ||
                controls.canUseDirectPaste
                  ? "0"
                  : nothing}
                role="region"
                aria-describedby="tp-screen-input-status"
                aria-keyshortcuts="Control+F Meta+F"
                aria-label=${controls.canUseDirectInput ||
                controls.canUseDirectPaste
                  ? "Terminal output and focused pane input"
                  : "Terminal output"}
                @keydown=${(event: KeyboardEvent) =>
                  this.handleViewportKeydown(event)}
                @paste=${(event: ClipboardEvent) =>
                  this.handleViewportPaste(event)}
                @pointerdown=${() => this.closeCommandContextMenu()}
                @scroll=${(event: Event) => this.handleViewportScroll(event)}
              >
                ${isTerminalPlacement
                  ? renderTerminalHistoryEntries(
                      terminalHistoryEntries,
                      searchResult,
                      {
                        activeCommandContextLineIndex:
                          this.commandContextMenu?.commandLineIndex ?? null,
                        commandMetadata:
                          this.commandPresentationMetadata ?? null,
                        onCommandContextMenu: (event, entry) =>
                          this.openCommandContextMenu(event, entry),
                      },
                    )
                  : searchResult.lines.map((line) =>
                      renderLine(
                        line.lineIndex + 1,
                        line.segments,
                        outputLines[line.lineIndex]?.source ?? "live",
                      ),
                    )}
              </div>
              ${this.renderCommandContextMenu()}
            `
          : isTerminalPlacement
            ? html`<div
                class="viewport"
                part="screen-lines"
                data-testid="tp-screen-viewport"
                role="region"
                aria-label="Terminal output"
              ></div>`
            : html`<div class="empty-state" part="empty">
                No active screen yet. Start or attach a session to see output
                here.
              </div>`}
      </div>
    `;
  }

  private renderFullChrome(
    chrome: TerminalScreenChromeState,
    inputStatus: ReturnType<typeof resolveTerminalScreenInputStatus>,
    searchResult: TerminalOutputSearchResult,
    controls: ReturnType<typeof resolveTerminalScreenControlState>,
  ): TemplateResult {
    return html`
      <div class="screen-header">
        <div class="panel-header">
          <div class="panel-eyebrow">Terminal</div>
          <div class="panel-title">${chrome.title}</div>
          <div class="panel-copy">Focused pane output.</div>
        </div>
        ${this.renderScreenActions(controls)}
      </div>
      ${this.renderScreenMeta(chrome, inputStatus)}
      <div class="screen-tools" part="screen-tools">
        ${this.renderSearch(searchResult)}
        ${this.renderSearchActions(searchResult)}
      </div>
    `;
  }

  private renderCompactChrome(
    chrome: TerminalScreenChromeState,
    inputStatus: ReturnType<typeof resolveTerminalScreenInputStatus>,
    searchResult: TerminalOutputSearchResult,
    controls: ReturnType<typeof resolveTerminalScreenControlState>,
  ): TemplateResult {
    return html`
      <div
        class="screen-chrome"
        part="screen-chrome"
        data-testid="tp-screen-chrome"
        data-chrome-mode=${chrome.mode}
        data-search-active=${String(Boolean(searchResult.query))}
      >
        <div class="screen-chrome__title">
          <span class="panel-title">${chrome.title}</span>
          ${this.renderScreenMeta(chrome, inputStatus)}
        </div>
        <div class="screen-chrome__tools">
          ${this.renderSearch(searchResult)}
          ${searchResult.query
            ? this.renderSearchActions(searchResult)
            : nothing}
          ${this.renderScreenActions(controls)}
        </div>
      </div>
    `;
  }

  private renderScreenMeta(
    chrome: TerminalScreenChromeState,
    inputStatus: ReturnType<typeof resolveTerminalScreenInputStatus>,
  ): TemplateResult {
    return html`
      <div class="screen-meta" part="meta" data-chrome-mode=${chrome.mode}>
        ${chrome.metaItems.map((item) => this.renderScreenMetaItem(item))}
        <span
          id="tp-screen-input-status"
          part=${`input-status input-status-${inputStatus.tone}`}
          data-testid="tp-screen-input-status"
          data-input-tone=${inputStatus.tone}
          title=${inputStatus.title}
          aria-live="polite"
        >
          ${inputStatus.label}
        </span>
      </div>
    `;
  }

  private renderScreenMetaItem(
    item: TerminalScreenChromeMetaItem,
  ): TemplateResult {
    return html`<span data-meta-id=${item.id} title=${item.title ?? item.label}
      >${item.label}</span
    >`;
  }

  private renderScreenActions(
    controls: ReturnType<typeof resolveTerminalScreenControlState>,
  ): TemplateResult {
    const actions = resolveTerminalScreenActions({
      canCopyVisibleOutput: controls.canCopyVisibleOutput,
      canLoadMoreHistory: controls.canLoadMoreHistory,
      copyState: this.copyState,
      followOutput: this.followOutput,
      historyLoadState: this.historyLoadState,
      placement: this.placement,
    });

    return html`
      <div class="screen-actions" part="screen-actions">
        ${actions.map(
          (action) => html`
            <button
              type="button"
              data-testid=${action.testId}
              data-screen-action=${action.id}
              data-screen-action-label-mode=${action.labelMode}
              data-screen-action-placement=${action.placement}
              data-screen-action-tone=${action.tone}
              aria-label=${action.ariaLabel}
              aria-pressed=${action.ariaPressed == null
                ? nothing
                : String(action.ariaPressed)}
              title=${action.title}
              ?disabled=${action.disabled}
              @click=${() => this.handleScreenActionClick(action.id)}
            >
              ${action.label}
            </button>
          `,
        )}
      </div>
    `;
  }

  private handleScreenActionClick(actionId: TerminalScreenActionId): void {
    switch (actionId) {
      case TERMINAL_SCREEN_ACTION_IDS.followOutput:
        this.toggleFollowOutput();
        return;
      case TERMINAL_SCREEN_ACTION_IDS.loadMoreHistory:
        void this.loadMoreHistory({ preserveScrollAnchor: true });
        return;
      case TERMINAL_SCREEN_ACTION_IDS.scrollLatest:
        this.scrollLatest();
        return;
      case TERMINAL_SCREEN_ACTION_IDS.copyVisible:
        void this.copyVisibleOutput();
        return;
    }
  }

  private async loadMoreHistory(
    options: { preserveScrollAnchor?: boolean; viewport?: HTMLElement } = {},
  ): Promise<void> {
    if (this.historyLoadState === "loading") {
      return;
    }

    const controls = resolveTerminalScreenControlState(this.snapshot);
    if (
      !this.kernel ||
      !controls.activePaneId ||
      !controls.canLoadMoreHistory
    ) {
      return;
    }

    this.followOutput = false;
    this.setHistoryLoadState("loading");
    const anchor = options.preserveScrollAnchor
      ? captureHistoryScrollAnchor(
          options.viewport ??
            this.shadowRoot?.querySelector<HTMLElement>(
              '[data-testid="tp-screen-viewport"]',
            ) ??
            null,
        )
      : null;
    try {
      const loaded = await this.kernel.commands.loadMorePaneHistory(
        controls.activePaneId,
      );
      if (loaded && anchor) {
        await this.updateComplete;
        restoreHistoryScrollAnchor(anchor);
      }
      this.setHistoryLoadState(loaded ? "idle" : "failed");
    } catch {
      this.setHistoryLoadState("failed");
    }
  }

  private renderSearch(
    searchResult: TerminalOutputSearchResult,
  ): TemplateResult {
    return html`
      <label class="search" part="search">
        <input
          data-testid="tp-screen-search"
          name="tp-screen-search"
          type="search"
          .value=${this.searchQuery}
          autocomplete="off"
          autocapitalize="off"
          autocorrect="off"
          enterkeyhint="search"
          inputmode="search"
          placeholder="Find output"
          spellcheck="false"
          aria-describedby=${TERMINAL_SCREEN_SEARCH_COUNT_ID}
          aria-label="Find terminal output"
          aria-keyshortcuts="Control+F Meta+F"
          @input=${(event: Event) => this.handleSearchInput(event)}
          @keydown=${(event: KeyboardEvent) => this.handleSearchKeydown(event)}
        />
        <span
          id=${TERMINAL_SCREEN_SEARCH_COUNT_ID}
          class="search-count"
          part="search-count"
          aria-atomic="true"
          aria-live="polite"
          data-search-active=${String(Boolean(searchResult.query))}
        >
          ${formatTerminalOutputSearchCount(
            searchResult.query,
            searchResult.matchCount,
            searchResult.activeMatchIndex,
          )}
        </span>
      </label>
    `;
  }

  private renderSearchActions(
    searchResult: TerminalOutputSearchResult,
  ): TemplateResult {
    const actions = resolveTerminalScreenSearchActions({
      matchCount: searchResult.matchCount,
      placement: this.placement,
      query: searchResult.query,
    });

    return html`
      <div class="search-actions" part="search-actions">
        ${actions.map(
          (action) => html`
            <button
              type="button"
              data-testid=${action.testId}
              data-screen-search-action=${action.id}
              data-screen-search-action-label-mode=${action.labelMode}
              data-screen-search-action-placement=${action.placement}
              data-screen-search-action-tone=${action.tone}
              aria-label=${action.ariaLabel}
              title=${action.title}
              ?disabled=${action.disabled}
              @click=${() => this.handleSearchActionClick(action.id)}
            >
              ${action.label}
            </button>
          `,
        )}
      </div>
    `;
  }

  private handleSearchActionClick(
    actionId: TerminalScreenSearchActionId,
  ): void {
    switch (actionId) {
      case TERMINAL_SCREEN_SEARCH_ACTION_IDS.previousMatch:
        this.selectSearchMatch("previous");
        return;
      case TERMINAL_SCREEN_SEARCH_ACTION_IDS.nextMatch:
        this.selectSearchMatch("next");
        return;
      case TERMINAL_SCREEN_SEARCH_ACTION_IDS.clearSearch:
        this.clearSearch();
        return;
    }
  }

  private toggleFollowOutput(): void {
    this.followOutput = !this.followOutput;
    if (this.followOutput) {
      this.scrollViewportToBottom();
    }
  }

  private scrollLatest(): void {
    this.followOutput = true;
    this.scrollViewportToBottom();
  }

  private handleSearchInput(event: Event): void {
    const target = event.currentTarget as HTMLInputElement;
    const nextQuery = target.value;
    const searchResult = this.createSearchResult(nextQuery);
    this.searchQuery = nextQuery;
    this.activeSearchMatchIndex = searchResult.matchCount > 0 ? 0 : null;
  }

  private renderCommandContextMenu(): TemplateResult | typeof nothing {
    const menu = this.commandContextMenu;
    if (!menu) {
      return nothing;
    }

    return html`
      <div
        class="command-context-menu"
        role="menu"
        data-testid="tp-command-context-menu"
        style=${`left: ${menu.x}px; top: ${menu.y}px;`}
        @contextmenu=${(event: MouseEvent) => event.preventDefault()}
        @keydown=${(event: KeyboardEvent) => {
          if (event.key === "Escape") {
            event.preventDefault();
            this.closeCommandContextMenu();
            this.focusViewport();
          }
        }}
        @pointerdown=${(event: PointerEvent) => event.stopPropagation()}
      >
        ${this.renderCommandContextMenuItem(
          "Copy",
          "⌘C",
          menu.blockText,
          "tp-command-context-copy",
        )}
        ${this.renderCommandContextMenuItem(
          "Copy command",
          "⇧⌘C",
          menu.commandText,
          "tp-command-context-copy-command",
        )}
        ${this.renderCommandContextMenuItem(
          "Copy output",
          "⌥⇧⌘C",
          menu.outputText,
          "tp-command-context-copy-output",
        )}
      </div>
    `;
  }

  private renderCommandContextMenuItem(
    label: string,
    shortcut: string,
    text: string,
    testId: string,
  ): TemplateResult {
    return html`
      <button
        class="command-context-menu__item"
        type="button"
        role="menuitem"
        data-testid=${testId}
        @click=${() => this.copyCommandContextText(text)}
      >
        <span>${label}</span>
        <span class="command-context-menu__shortcut" aria-hidden="true"
          >${shortcut}</span
        >
      </button>
    `;
  }

  private openCommandContextMenu(
    event: MouseEvent,
    entry: Extract<TerminalHistoryEntry, { kind: "command" }>,
  ): void {
    event.preventDefault();
    const copyText = createTerminalCommandContextCopyText(entry);
    this.commandContextMenu = {
      blockText: copyText.blockText,
      commandLineIndex: entry.commandLineIndex,
      commandText: copyText.commandText,
      outputText: copyText.outputText,
      x: clampContextMenuCoordinate(event.clientX, window.innerWidth, 224),
      y: clampContextMenuCoordinate(event.clientY, window.innerHeight, 132),
    };
    requestAnimationFrame(() => {
      this.shadowRoot
        ?.querySelector<HTMLButtonElement>(
          '[data-testid="tp-command-context-copy"]',
        )
        ?.focus({ preventScroll: true });
    });
  }

  private closeCommandContextMenu(): void {
    this.commandContextMenu = null;
  }

  private async copyCommandContextText(text: string): Promise<void> {
    this.closeCommandContextMenu();
    try {
      await writeClipboardText(text);
      this.setCopyState("copied");
    } catch {
      this.setCopyState("failed");
    }
  }

  private handleSearchKeydown(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    ) {
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      this.selectSearchMatch(event.shiftKey ? "previous" : "next");
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      this.clearSearch();
      this.focusViewport();
    }
  }

  private selectSearchMatch(direction: "next" | "previous"): void {
    const searchResult = this.createSearchResult();
    if (searchResult.matchCount === 0) {
      return;
    }

    const currentMatchIndex = searchResult.activeMatchIndex ?? 0;
    this.activeSearchMatchIndex =
      direction === "next"
        ? (currentMatchIndex + 1) % searchResult.matchCount
        : (currentMatchIndex - 1 + searchResult.matchCount) %
          searchResult.matchCount;
  }

  private clearSearch(): void {
    this.searchQuery = "";
    this.activeSearchMatchIndex = null;
  }

  private async copyVisibleOutput(): Promise<void> {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    const screen = controls.screen;
    if (!screen || !controls.canCopyVisibleOutput) {
      return;
    }

    const outputLines = createVisibleOutputLines(controls.history, screen, {
      hideShellPromptNoise: this.hideShellPromptNoise,
    });
    const output = serializeTerminalOutputLines(
      outputLines.map((line) => line.text),
    );
    try {
      await writeClipboardText(output);
      this.setCopyState("copied");
      this.dispatchEvent(
        new CustomEvent<TerminalScreenCopiedDetail>(
          TERMINAL_SCREEN_EVENTS.copied,
          {
            bubbles: true,
            composed: true,
            detail: { paneId: screen.pane_id, lineCount: outputLines.length },
          },
        ),
      );
    } catch (error) {
      this.setCopyState("failed");
      this.dispatchEvent(
        new CustomEvent<TerminalScreenCopyFailedDetail>(
          TERMINAL_SCREEN_EVENTS.copyFailed,
          {
            bubbles: true,
            composed: true,
            detail: { paneId: screen.pane_id, error },
          },
        ),
      );
    }
  }

  private setCopyState(copyState: TerminalScreenCopyState): void {
    this.copyState = copyState;
    this.clearCopyStateResetTimer();
    this.#copyStateResetTimer = setTimeout(() => {
      this.copyState = "idle";
      this.#copyStateResetTimer = null;
    }, 1600);
  }

  private clearCopyStateResetTimer(): void {
    if (this.#copyStateResetTimer) {
      clearTimeout(this.#copyStateResetTimer);
      this.#copyStateResetTimer = null;
    }
  }

  private setHistoryLoadState(
    historyLoadState: TerminalScreenHistoryLoadState,
  ): void {
    this.historyLoadState = historyLoadState;
    this.clearHistoryLoadStateResetTimer();
    if (historyLoadState === "failed") {
      this.#historyLoadStateResetTimer = setTimeout(() => {
        this.historyLoadState = "idle";
        this.#historyLoadStateResetTimer = null;
      }, 2800);
    }
  }

  private clearHistoryLoadStateResetTimer(): void {
    if (this.#historyLoadStateResetTimer) {
      clearTimeout(this.#historyLoadStateResetTimer);
      this.#historyLoadStateResetTimer = null;
    }
  }

  private setDirectInputActivity(activity: TerminalScreenInputActivity): void {
    this.directInputActivity = activity;
    this.clearDirectInputActivityResetTimer();
    if (activity === "failed") {
      this.#directInputActivityResetTimer = setTimeout(() => {
        this.directInputActivity = "idle";
        this.#directInputActivityResetTimer = null;
      }, 2800);
    }
  }

  private clearDirectInputActivityResetTimer(): void {
    if (this.#directInputActivityResetTimer) {
      clearTimeout(this.#directInputActivityResetTimer);
      this.#directInputActivityResetTimer = null;
    }
  }

  private syncActiveSearchMatch(): boolean {
    const searchResult = this.createSearchResult();
    if (searchResult.matchCount === 0) {
      if (this.activeSearchMatchIndex !== null) {
        this.activeSearchMatchIndex = null;
        return true;
      }
      return false;
    }

    const activeSearchMatchIndex =
      resolveTerminalOutputSearchMatchIndex(
        this.activeSearchMatchIndex,
        searchResult.matchCount,
      ) ?? 0;
    if (activeSearchMatchIndex !== this.activeSearchMatchIndex) {
      this.activeSearchMatchIndex = activeSearchMatchIndex;
      return true;
    }

    this.scrollActiveSearchMatchIntoView();
    return true;
  }

  private scrollActiveSearchMatchIntoView(): void {
    const activeMatch = this.shadowRoot?.querySelector<HTMLElement>(
      '[data-testid="tp-screen-active-search-match"]',
    );
    activeMatch?.scrollIntoView({
      block: "center",
      inline: "nearest",
    });
  }

  private createSearchResult(
    searchQuery = this.searchQuery,
    lines?: readonly string[],
  ): TerminalOutputSearchResult {
    const fallbackControls = resolveTerminalScreenControlState(this.snapshot);
    const fallbackLines = createVisibleOutputLines(
      fallbackControls.history,
      fallbackControls.screen,
    ).map((line) => line.text);
    return createTerminalOutputSearchResult(
      lines ?? fallbackLines,
      searchQuery,
      { activeMatchIndex: this.activeSearchMatchIndex },
    );
  }

  private syncTerminalDisplayAttributes(): void {
    this.setAttribute(
      "data-font-scale",
      this.snapshot.terminalDisplay.fontScale,
    );
    this.setAttribute(
      "data-line-wrap",
      String(this.snapshot.terminalDisplay.lineWrap),
    );
  }

  private handleViewportScroll(event: Event): void {
    if (this.#autoScrolling) {
      return;
    }

    this.closeCommandContextMenu();
    const viewport = event.currentTarget as HTMLElement;
    if (!isViewportAtBottom(viewport)) {
      this.followOutput = false;
    }

    if (
      shouldAutoLoadMoreHistoryFromViewport(
        viewport,
        resolveTerminalScreenControlState(this.snapshot).canLoadMoreHistory,
        this.historyLoadState,
      )
    ) {
      void this.loadMoreHistory({ preserveScrollAnchor: true, viewport });
    }
  }

  private handleViewportKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented) {
      return;
    }

    if (isTerminalScreenSearchShortcut(event)) {
      event.preventDefault();
      this.focusSearchInput();
      return;
    }

    const input = terminalInputForKeyboardEvent(event);
    if (!input) {
      return;
    }

    event.preventDefault();
    this.#directInputBuffer.push(input);
  }

  private handleViewportPaste(event: ClipboardEvent): void {
    if (event.defaultPrevented) {
      return;
    }

    const controls = resolveTerminalScreenControlState(this.snapshot);
    if (!controls.canUseDirectPaste) {
      return;
    }

    const pastedText = event.clipboardData?.getData("text/plain") ?? "";
    if (pastedText.length === 0) {
      return;
    }

    event.preventDefault();
    this.#directInputBuffer.flush();
    this.queueDirectPaste(pastedText);
  }

  private queueDirectInput(input: string): void {
    this.#directInputQueue = this.#directInputQueue
      .catch(() => undefined)
      .then(() => this.dispatchDirectInput(input));
  }

  private queueDirectPaste(data: string): void {
    this.#directInputQueue = this.#directInputQueue
      .catch(() => undefined)
      .then(() => this.dispatchDirectPaste(data));
  }

  private async dispatchDirectInput(input: string): Promise<void> {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    if (
      !controls.activeSessionId ||
      !controls.activePaneId ||
      !controls.canUseDirectInput
    ) {
      return;
    }

    try {
      await this.kernel?.commands.dispatchMuxCommand(controls.activeSessionId, {
        kind: "send_input",
        pane_id: controls.activePaneId,
        data: input,
        client_event_id: createTerminalClientEventId("screen-input"),
      });
      if (shouldRefreshAfterTerminalDirectInput(input)) {
        await this.kernel?.commands.attachSession(controls.activeSessionId);
      }
      if (this.directInputActivity !== "idle") {
        this.setDirectInputActivity("idle");
      }
      this.dispatchEvent(
        new CustomEvent<TerminalScreenInputSubmittedDetail>(
          TERMINAL_SCREEN_EVENTS.inputSubmitted,
          {
            bubbles: true,
            composed: true,
            detail: {
              sessionId: controls.activeSessionId,
              paneId: controls.activePaneId,
              inputLength: input.length,
            },
          },
        ),
      );
    } catch (error) {
      this.setDirectInputActivity("failed");
      this.dispatchEvent(
        new CustomEvent<TerminalScreenInputFailedDetail>(
          TERMINAL_SCREEN_EVENTS.inputFailed,
          {
            bubbles: true,
            composed: true,
            detail: {
              sessionId: controls.activeSessionId,
              paneId: controls.activePaneId,
              error,
            },
          },
        ),
      );
    }
  }

  private async dispatchDirectPaste(data: string): Promise<void> {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    if (
      !controls.activeSessionId ||
      !controls.activePaneId ||
      !controls.canUseDirectPaste
    ) {
      return;
    }

    try {
      await this.kernel?.commands.dispatchMuxCommand(controls.activeSessionId, {
        kind: "send_paste",
        pane_id: controls.activePaneId,
        data,
        client_event_id: createTerminalClientEventId("screen-paste"),
      });
      await this.kernel?.commands.attachSession(controls.activeSessionId);
      if (this.directInputActivity !== "idle") {
        this.setDirectInputActivity("idle");
      }
      this.dispatchEvent(
        new CustomEvent<TerminalScreenPasteSubmittedDetail>(
          TERMINAL_SCREEN_EVENTS.pasteSubmitted,
          {
            bubbles: true,
            composed: true,
            detail: {
              sessionId: controls.activeSessionId,
              paneId: controls.activePaneId,
              inputLength: data.length,
            },
          },
        ),
      );
    } catch (error) {
      this.setDirectInputActivity("failed");
      this.dispatchEvent(
        new CustomEvent<TerminalScreenPasteFailedDetail>(
          TERMINAL_SCREEN_EVENTS.pasteFailed,
          {
            bubbles: true,
            composed: true,
            detail: {
              sessionId: controls.activeSessionId,
              paneId: controls.activePaneId,
              error,
            },
          },
        ),
      );
    }
  }

  private scrollViewportToBottom(): void {
    const viewport = this.shadowRoot?.querySelector<HTMLElement>(
      '[data-testid="tp-screen-viewport"]',
    );
    if (!viewport) {
      return;
    }

    const scrollToLatest = () => {
      viewport.scrollTop = viewport.scrollHeight;
    };

    this.#autoScrolling = true;
    scrollToLatest();
    requestAnimationFrame(() => {
      scrollToLatest();
      requestAnimationFrame(() => {
        scrollToLatest();
        this.#autoScrolling = false;
      });
    });
  }

  private focusSearchInput(): void {
    const searchInput = this.shadowRoot?.querySelector<HTMLInputElement>(
      '[data-testid="tp-screen-search"]',
    );
    if (!searchInput || searchInput.disabled) {
      return;
    }

    searchInput.focus({ preventScroll: true });
    searchInput.select();
  }

  private focusViewport(): void {
    const viewport = this.shadowRoot?.querySelector<HTMLElement>(
      '[data-testid="tp-screen-viewport"]',
    );
    viewport?.focus({ preventScroll: true });
  }
}

function renderLine(
  index: number,
  segments: readonly TerminalOutputSearchSegment[],
  source: VisibleOutputLineSource = "live",
): TemplateResult {
  return html`
    <div class="line" part="screen-line" data-line-source=${source}>
      <span class="gutter" part="line-number" aria-hidden="true">${index}</span>
      <span class="text" part="line-text"
        >${renderHighlightedSegments(segments)}</span
      >
    </div>
  `;
}

function renderTerminalHistoryEntries(
  entries: readonly TerminalHistoryEntry[],
  searchResult: TerminalOutputSearchResult,
  options: TerminalHistoryRenderOptions = {},
): TemplateResult[] {
  const commandMetadataByEntryIndex = matchCommandPresentationMetadata(
    entries,
    options.commandMetadata ?? [],
  );

  return entries.map((entry) => {
    if (entry.kind === "line") {
      return renderLine(
        entry.lineIndex + 1,
        getSearchSegments(searchResult, entry.lineIndex),
        entry.line.source,
      );
    }

    return html`
      <section
        class="history-entry"
        part="history-entry"
        data-line-source=${entry.commandLine.source}
        data-command-context-menu=${String(
          options.activeCommandContextLineIndex === entry.commandLineIndex,
        )}
        data-command-status=${commandMetadataByEntryIndex.get(
          entry.commandLineIndex,
        )?.status ?? "unknown"}
        @contextmenu=${(event: MouseEvent) =>
          options.onCommandContextMenu?.(event, entry)}
      >
        <div class="history-entry-prompt" part="history-entry-prompt">
          <span>${entry.prompt}</span>${renderCommandPresentationMetadata(
            commandMetadataByEntryIndex.get(entry.commandLineIndex),
          )}
        </div>
        <div class="history-entry-command" part="history-entry-command">
          <span class="history-entry-text" part="history-entry-command-text">
            ${renderCommandSegments(entry.command, searchResult.query)}
          </span>
        </div>
        ${entry.output.map(
          (outputLine) => html`
            <div
              class="history-entry-output"
              part="history-entry-output"
              data-line-source=${outputLine.line.source}
            >
              <span class="history-entry-text" part="history-entry-output-text">
                ${renderHighlightedSegments(
                  getSearchSegments(searchResult, outputLine.lineIndex),
                )}
              </span>
            </div>
          `,
        )}
      </section>
    `;
  });
}

function matchCommandPresentationMetadata(
  entries: readonly TerminalHistoryEntry[],
  metadata: readonly TerminalCommandPresentationMetadata[],
): Map<number, TerminalCommandPresentationMetadata> {
  const matched = new Map<number, TerminalCommandPresentationMetadata>();
  if (metadata.length === 0) {
    return matched;
  }

  const candidates = metadata
    .map((item, index) => ({
      index,
      item,
      command: normalizeCommandPresentationMatch(item.command),
    }))
    .filter((candidate) => candidate.command.length > 0);
  const used = new Set<number>();

  for (let entryIndex = entries.length - 1; entryIndex >= 0; entryIndex -= 1) {
    const entry = entries[entryIndex];
    if (!entry || entry.kind !== "command") {
      continue;
    }

    const command = normalizeCommandPresentationMatch(entry.command);
    for (
      let candidateIndex = candidates.length - 1;
      candidateIndex >= 0;
      candidateIndex -= 1
    ) {
      const candidate = candidates[candidateIndex];
      if (
        !candidate ||
        used.has(candidate.index) ||
        !doesCommandPresentationMatchHistoryEntry(command, candidate.command)
      ) {
        continue;
      }

      matched.set(entry.commandLineIndex, candidate.item);
      used.add(candidate.index);
      break;
    }
  }

  return matched;
}

function renderCommandPresentationMetadata(
  metadata: TerminalCommandPresentationMetadata | undefined,
): TemplateResult | typeof nothing {
  if (!metadata) {
    return nothing;
  }

  const status = metadata.status ?? "unknown";
  const durationLabel =
    typeof metadata.durationMs === "number" &&
    Number.isFinite(metadata.durationMs)
      ? formatCommandDuration(metadata.durationMs)
      : null;
  const exitCodeLabel =
    typeof metadata.exitCode === "number" && Number.isFinite(metadata.exitCode)
      ? `exit ${Math.trunc(metadata.exitCode)}`
      : null;
  const label = formatCommandMetadataLabel(
    status,
    durationLabel,
    exitCodeLabel,
  );

  return label
    ? html`
        <span
          class="history-entry-meta"
          part="history-entry-meta"
          data-command-status=${status}
          title=${formatCommandMetadataTitle(
            status,
            durationLabel,
            exitCodeLabel,
          )}
        >
          ${label}
        </span>
      `
    : nothing;
}

function formatCommandMetadataLabel(
  status: TerminalCommandPresentationStatus,
  durationLabel: string | null,
  exitCodeLabel: string | null,
): string | null {
  if (status === "running") {
    return durationLabel ? `running (${durationLabel})` : "running";
  }

  if (status === "failed") {
    return [
      exitCodeLabel ?? "error",
      durationLabel ? `(${durationLabel})` : null,
    ]
      .filter(Boolean)
      .join(" ");
  }

  if (durationLabel) {
    return `(${durationLabel})`;
  }

  return exitCodeLabel;
}

function formatCommandMetadataTitle(
  status: TerminalCommandPresentationStatus,
  durationLabel: string | null,
  exitCodeLabel: string | null,
): string {
  const statusLabel =
    status === "failed"
      ? "Command failed"
      : status === "running"
        ? "Command is running"
        : status === "succeeded"
          ? "Command completed"
          : "Command status unknown";
  return [
    statusLabel,
    durationLabel ? `Duration ${durationLabel}` : null,
    exitCodeLabel,
  ]
    .filter(Boolean)
    .join(" - ");
}

function formatCommandDuration(durationMs: number): string {
  const safeMs = Math.max(0, durationMs);
  if (safeMs < 1000) {
    return `${(safeMs / 1000).toFixed(3)}s`;
  }

  if (safeMs < 10_000) {
    return `${(safeMs / 1000).toFixed(2)}s`;
  }

  return `${(safeMs / 1000).toFixed(1)}s`;
}

function clampContextMenuCoordinate(
  coordinate: number,
  viewportSize: number,
  menuSize: number,
): number {
  if (!Number.isFinite(coordinate) || !Number.isFinite(viewportSize)) {
    return 0;
  }

  return Math.max(
    8,
    Math.min(coordinate, Math.max(8, viewportSize - menuSize - 8)),
  );
}

function normalizeCommandPresentationMatch(command: string): string {
  return command.trim().replace(/\s+/gu, " ");
}

export function doesCommandPresentationMatchHistoryEntry(
  entryCommand: string,
  metadataCommand: string,
): boolean {
  const entry = normalizeCommandPresentationMatch(entryCommand);
  const metadata = normalizeCommandPresentationMatch(metadataCommand);
  if (!entry || !metadata) {
    return false;
  }

  if (entry === metadata) {
    return true;
  }

  if (entry.length < 8) {
    return false;
  }

  return metadata.startsWith(entry) || metadata.includes(entry);
}

function getSearchSegments(
  searchResult: TerminalOutputSearchResult,
  lineIndex: number,
): readonly TerminalOutputSearchSegment[] {
  return (
    searchResult.lines[lineIndex]?.segments ?? [{ kind: "text", value: "" }]
  );
}

function renderCommandSegments(command: string, query: string): TemplateResult {
  const result = createTerminalOutputSearchResult([command], query);
  return renderHighlightedSegments(
    result.lines[0]?.segments ?? [{ kind: "text", value: command }],
  );
}

export function createVisibleOutputLines(
  history: ReturnType<typeof resolveTerminalScreenControlState>["history"],
  screen: ReturnType<typeof resolveTerminalScreenControlState>["screen"],
  options: VisibleOutputLineOptions = {},
): VisibleOutputLine[] {
  const liveLines = trimTrailingEmptyLiveLines(
    screen?.surface.lines.map((line) => ({
      text: line.text,
      source: "live" as const,
    })) ?? [],
  );
  const historyLines =
    history?.lines
      .filter(
        (line, index, lines) => line.length > 0 || index < lines.length - 1,
      )
      .map((line) => ({
        text: line,
        source: "history" as const,
      })) ?? [];
  const dedupedHistoryLines =
    liveLines.length > 0
      ? removeHistorySuffixOverlappingLivePrefix(historyLines, liveLines)
      : historyLines;
  const hasPartialRestoredHistory = history?.hasMoreSegments === true;
  const historyBoundaryLines: VisibleOutputLine[] = hasPartialRestoredHistory
    ? [{ text: RESTORED_HISTORY_PARTIAL_TEXT, source: "boundary" }]
    : [];

  if (dedupedHistoryLines.length === 0 && historyBoundaryLines.length === 0) {
    return dedupeVisibleHistoryLiveOverlap(
      filterVisibleOutputLines(liveLines, options),
    );
  }

  if (liveLines.length === 0) {
    return dedupeVisibleHistoryLiveOverlap(
      filterVisibleOutputLines(
        [...dedupedHistoryLines, ...historyBoundaryLines],
        options,
      ),
    );
  }

  return dedupeVisibleHistoryLiveOverlap(
    filterVisibleOutputLines(
      [
        ...dedupedHistoryLines,
        ...historyBoundaryLines,
        ...(dedupedHistoryLines.length > 0 || hasPartialRestoredHistory
          ? [
              {
                text: RESTORED_HISTORY_BOUNDARY_TEXT,
                source: "boundary",
              } as const,
            ]
          : []),
        ...liveLines,
      ],
      options,
    ),
  );
}

function removeHistorySuffixOverlappingLivePrefix(
  historyLines: readonly VisibleOutputLine[],
  liveLines: readonly VisibleOutputLine[],
): VisibleOutputLine[] {
  const overlap = findHistoryLiveLineOverlap(historyLines, liveLines);
  return overlap > 0 ? historyLines.slice(0, -overlap) : [...historyLines];
}

function findHistoryLiveLineOverlap(
  historyLines: readonly VisibleOutputLine[],
  liveLines: readonly VisibleOutputLine[],
): number {
  const maxOverlap = Math.min(historyLines.length, liveLines.length, 240);
  for (let size = maxOverlap; size > 0; size -= 1) {
    let matches = true;
    for (let index = 0; index < size; index += 1) {
      const historyText =
        historyLines[historyLines.length - size + index]?.text ?? "";
      const liveText = liveLines[index]?.text ?? "";
      if (historyText !== liveText) {
        matches = false;
        break;
      }
    }
    if (matches) {
      return size;
    }
  }

  return 0;
}

function dedupeVisibleHistoryLiveOverlap(
  lines: readonly VisibleOutputLine[],
): VisibleOutputLine[] {
  const firstLiveIndex = lines.findIndex((line) => line.source === "live");
  if (firstLiveIndex <= 0) {
    return [...lines];
  }

  let historyBlockStart = firstLiveIndex;
  while (
    historyBlockStart > 0 &&
    lines[historyBlockStart - 1]?.source === "history"
  ) {
    historyBlockStart -= 1;
  }

  const historyBlock = lines.slice(historyBlockStart, firstLiveIndex);
  const liveBlock = lines
    .slice(firstLiveIndex)
    .filter((line) => line.source === "live");
  const overlap = findHistoryLiveLineOverlap(historyBlock, liveBlock);
  if (overlap === 0) {
    return [...lines];
  }

  return [
    ...lines.slice(0, historyBlockStart),
    ...historyBlock.slice(0, -overlap),
    ...lines.slice(firstLiveIndex),
  ];
}

export function createTerminalHistoryEntries(
  lines: readonly VisibleOutputLine[],
  options: TerminalHistoryEntryOptions = {},
): TerminalHistoryEntry[] {
  const entries: TerminalHistoryEntry[] = [];
  let activeEntry: Extract<TerminalHistoryEntry, { kind: "command" }> | null =
    null;
  const terminalPromptLabel = normalizeTerminalPromptLabel(
    options.terminalPromptLabel,
  );

  const flushActiveEntry = () => {
    if (activeEntry) {
      entries.push(activeEntry);
      activeEntry = null;
    }
  };

  lines.forEach((line, lineIndex) => {
    if (line.source === "boundary") {
      flushActiveEntry();
      entries.push({ kind: "line", line, lineIndex });
      return;
    }

    const promptCommand = parseShellPromptCommandLine(line.text);
    if (promptCommand) {
      flushActiveEntry();
      activeEntry = {
        kind: "command",
        prompt: promptCommand.prompt,
        commandLine: line,
        commandLineIndex: lineIndex,
        command: promptCommand.command,
        output: [],
      };
      return;
    }

    const wrappedInputCommand = parseWrappedInputCommandLine(
      line.text,
      terminalPromptLabel,
    );
    if (wrappedInputCommand) {
      flushActiveEntry();
      activeEntry = {
        kind: "command",
        prompt: wrappedInputCommand.prompt,
        commandLine: line,
        commandLineIndex: lineIndex,
        command: wrappedInputCommand.command,
        output: [],
      };
      return;
    }

    if (activeEntry && activeEntry.commandLine.source !== line.source) {
      flushActiveEntry();
    }

    if (activeEntry) {
      activeEntry.output.push({ line, lineIndex });
      return;
    }

    entries.push({ kind: "line", line, lineIndex });
  });

  flushActiveEntry();
  return dedupeHistoryCommandEntriesAgainstLive(
    removeRedundantTerminalCommandFragments(entries),
  );
}

export function createTerminalCommandContextCopyText(
  entry: Extract<TerminalHistoryEntry, { kind: "command" }>,
): {
  blockText: string;
  commandText: string;
  outputText: string;
} {
  const outputLines = entry.output.map((outputLine) => outputLine.line.text);
  return {
    blockText: serializeTerminalOutputLines([entry.command, ...outputLines]),
    commandText: entry.command,
    outputText: serializeTerminalOutputLines(outputLines),
  };
}

function dedupeHistoryCommandEntriesAgainstLive(
  entries: readonly TerminalHistoryEntry[],
): TerminalHistoryEntry[] {
  const liveCommandSignatures = new Set(
    entries
      .filter(
        (entry): entry is Extract<TerminalHistoryEntry, { kind: "command" }> =>
          entry.kind === "command" && entry.commandLine.source === "live",
      )
      .map(createTerminalCommandEntrySignature),
  );

  if (liveCommandSignatures.size === 0) {
    return [...entries];
  }

  const historyCommandsWithOutput = new Set(
    entries
      .filter(
        (entry): entry is Extract<TerminalHistoryEntry, { kind: "command" }> =>
          entry.kind === "command" &&
          entry.commandLine.source === "history" &&
          hasMeaningfulTerminalCommandOutput(entry),
      )
      .map((entry) => entry.command),
  );

  return entries.filter((entry) => {
    if (
      entry.kind === "command" &&
      entry.commandLine.source === "live" &&
      !hasMeaningfulTerminalCommandOutput(entry) &&
      historyCommandsWithOutput.has(entry.command)
    ) {
      return false;
    }

    if (entry.kind !== "command" || entry.commandLine.source !== "history") {
      return true;
    }
    return !liveCommandSignatures.has(
      createTerminalCommandEntrySignature(entry),
    );
  });
}

function hasMeaningfulTerminalCommandOutput(
  entry: Extract<TerminalHistoryEntry, { kind: "command" }>,
): boolean {
  return entry.output.some(({ line }) => line.text.trim().length > 0);
}

function removeRedundantTerminalCommandFragments(
  entries: readonly TerminalHistoryEntry[],
): TerminalHistoryEntry[] {
  return entries.filter((entry, index) => {
    if (entry.kind !== "command" || hasMeaningfulTerminalCommandOutput(entry)) {
      return true;
    }

    return !entries
      .slice(index + 1, index + 8)
      .some(
        (candidate) =>
          candidate.kind === "command" &&
          hasMeaningfulTerminalCommandOutput(candidate) &&
          isLikelyRedundantTerminalCommandFragment(entry, candidate),
      );
  });
}

function isLikelyRedundantTerminalCommandFragment(
  fragmentEntry: Extract<TerminalHistoryEntry, { kind: "command" }>,
  fullEntry: Extract<TerminalHistoryEntry, { kind: "command" }>,
): boolean {
  if (fragmentEntry.commandLine.source !== fullEntry.commandLine.source) {
    return false;
  }

  const fragment = normalizeCommandPresentationMatch(fragmentEntry.command);
  const full = normalizeCommandPresentationMatch(fullEntry.command);
  if (!fragment || !full || fragment === full) {
    return false;
  }

  if (full.startsWith(fragment)) {
    const nextCharacter = full[fragment.length] ?? "";
    return nextCharacter !== " ";
  }

  return (
    fragment.length >= 8 &&
    fragmentEntry.commandLine.text.trimStart().startsWith("<") &&
    full.includes(fragment)
  );
}

function createTerminalCommandEntrySignature(
  entry: Extract<TerminalHistoryEntry, { kind: "command" }>,
): string {
  const outputText = entry.output
    .map(({ line }) => line.text.trim())
    .filter(Boolean)
    .join("\u0000");
  return `${entry.prompt}\u0000${entry.command}\u0000${outputText}`;
}

function trimTrailingEmptyLiveLines(
  lines: readonly VisibleOutputLine[],
): VisibleOutputLine[] {
  let endIndex = lines.length;
  while (endIndex > 0 && isBlankTerminalLine(lines[endIndex - 1]?.text ?? "")) {
    endIndex -= 1;
  }
  return lines.slice(0, endIndex);
}

function isBlankTerminalLine(text: string): boolean {
  return text.trim().length === 0;
}

function filterVisibleOutputLines(
  lines: readonly VisibleOutputLine[],
  options: VisibleOutputLineOptions,
): VisibleOutputLine[] {
  if (!options.hideShellPromptNoise) {
    return [...lines];
  }

  const filteredLines: VisibleOutputLine[] = [];
  for (const line of lines) {
    const normalizedLine = normalizeShellPromptNoiseLine(line, options);
    if (normalizedLine) {
      filteredLines.push(normalizedLine);
    }
  }

  return filteredLines;
}

function normalizeShellPromptNoiseLine(
  line: VisibleOutputLine,
  options: VisibleOutputLineOptions,
): VisibleOutputLine | null {
  const text = line.text.trim();
  if (!text) {
    return line;
  }

  if (text === RESTORED_HISTORY_BOUNDARY_TEXT) {
    return null;
  }

  if (text === "%" || text === "$" || text === "#") {
    return null;
  }

  if (/^(?:dquote|quote|bquote|cmdsubst|heredoc)>/u.test(text)) {
    return null;
  }

  if (/^printf\s+"TP_[A-Z0-9_]/u.test(text)) {
    return null;
  }

  const wrappedInputCommand = parseWrappedInputCommandLine(
    line.text,
    options.terminalPromptLabel,
  );
  if (wrappedInputCommand) {
    return isInternalSmokeCommand(wrappedInputCommand.command) ||
      !options.preserveShellPromptCommands
      ? null
      : { ...line, text: wrappedInputCommand.text };
  }

  if (parseShellPromptOnlyLine(line.text)) {
    return null;
  }

  const promptCommand = parseShellPromptCommandLine(line.text);
  if (promptCommand) {
    return isInternalSmokeCommand(promptCommand.command) ||
      !options.preserveShellPromptCommands
      ? null
      : line;
  }

  const wrappedPromptCommandPattern =
    /^(\s*)[\w.-]{1,64}\s+(?:~(?:\/[\w.,@:+-][\w.,@:+/-]{0,180})?|(?:\/|\.\/|\.\.\/)?[\w.,@:+-][\w.,@:+/-]{0,180}|[A-Za-z]:[\w.,@:+/\\-]{0,180})\s[%$#]\s+(.+)$/u;
  const wrappedPromptCommandMatch = wrappedPromptCommandPattern.exec(line.text);
  if (wrappedPromptCommandMatch) {
    return isInternalSmokeCommand(wrappedPromptCommandMatch[2] ?? "") ||
      !options.preserveShellPromptCommands
      ? null
      : line;
  }

  const wrappedPromptOnlyPattern =
    /^\s*[\w.-]{1,64}\s+(?:~(?:\/[\w.,@:+-][\w.,@:+/-]{0,180})?|(?:\/|\.\/|\.\.\/)?[\w.,@:+-][\w.,@:+/-]{0,180}|[A-Za-z]:[\w.,@:+/\\-]{0,180})\s[%$#]\s*$/u;
  return wrappedPromptOnlyPattern.test(text) ? null : line;
}

function parseShellPromptCommandLine(
  value: string,
): ShellPromptCommandLine | null {
  const trimmed = value.trimEnd();
  for (let index = trimmed.length - 1; index >= 0; index -= 1) {
    const marker = trimmed[index] ?? "";
    if (!isShellPromptMarker(marker)) {
      continue;
    }

    const command = trimmed.slice(index + 1);
    if (!command.startsWith(" ") || command.trim().length === 0) {
      continue;
    }

    const prompt = trimmed.slice(0, index).trim();
    if (looksLikeShellPromptPrefix(prompt)) {
      return {
        prompt: normalizeShellPromptDisplay(prompt),
        command: command.trimStart(),
      };
    }
  }

  return null;
}

function parseWrappedInputCommandLine(
  value: string,
  promptLabel?: string,
): (ShellPromptCommandLine & { text: string }) | null {
  const match = /^<\s{4,}(.+)$/u.exec(value.trimEnd());
  const command = match?.[1]?.trim();
  if (!command) {
    return null;
  }

  const prompt = normalizeTerminalPromptLabel(promptLabel);
  return {
    prompt,
    command,
    text: `${prompt} % ${command}`,
  };
}

function normalizeTerminalPromptLabel(
  value: string | null | undefined,
): string {
  const trimmed = value?.trim() ?? "";
  return trimmed.length > 0 ? trimmed : "shell";
}

function parseShellPromptOnlyLine(value: string): string | null {
  const trimmed = value.trimEnd();
  const marker = trimmed.at(-1) ?? "";
  if (!isShellPromptMarker(marker)) {
    return null;
  }

  const prompt = trimmed.slice(0, -1).trim();
  return looksLikeShellPromptPrefix(prompt)
    ? normalizeShellPromptDisplay(prompt)
    : null;
}

function isShellPromptMarker(value: string): boolean {
  return value === "%" || value === "$" || value === "#";
}

function looksLikeShellPromptPrefix(value: string): boolean {
  const normalized = stripShellPromptTiming(value).trim();
  if (!normalized || normalized.length > 320) {
    return false;
  }
  if (normalized === "shell") {
    return true;
  }

  const tokens = normalized.split(/\s+/u).filter(Boolean);
  if (tokens.length === 0) {
    return false;
  }

  let remaining = normalized;
  let hasEnvironmentPrefix = false;
  while (remaining.startsWith("(")) {
    const closeIndex = remaining.indexOf(")");
    if (closeIndex < 2 || closeIndex > 48) {
      break;
    }

    hasEnvironmentPrefix = true;
    remaining = remaining.slice(closeIndex + 1).trimStart();
  }

  const remainingTokens = remaining.split(/\s+/u).filter(Boolean);
  const firstToken = remainingTokens[0] ?? "";
  const lastToken = remainingTokens.at(-1) ?? "";
  const hasUserHostPrefix =
    firstToken.includes("@") && remainingTokens.length > 1;
  const hasPathToken = tokens.some(isShellPromptPathToken);

  return (
    hasPathToken ||
    ((hasEnvironmentPrefix || hasUserHostPrefix) &&
      isSafeShellPromptToken(lastToken))
  );
}

function stripShellPromptTiming(value: string): string {
  return value.replace(/\s+\({1,2}\d+(?:\.\d+)?s\){1,2}\s*$/u, "");
}

function normalizeShellPromptDisplay(value: string): string {
  return value.replace(/\s+/gu, " ").trim();
}

function isShellPromptPathToken(value: string): boolean {
  return (
    value === "~" ||
    value.startsWith("~/") ||
    value.startsWith("/") ||
    value.startsWith("./") ||
    value.startsWith("../") ||
    /^[A-Za-z]:[\\/]/u.test(value)
  );
}

function isSafeShellPromptToken(value: string): boolean {
  if (value.length === 0 || value.length > 181) {
    return false;
  }

  return Array.from(value).every((char) => {
    const code = char.charCodeAt(0);
    return code > 32 && !isShellPromptMarker(char);
  });
}

function isInternalSmokeCommand(command: string): boolean {
  return /^printf\s+"TP_[A-Z0-9_]/u.test(command.trim());
}

function renderHighlightedSegments(
  segments: readonly TerminalOutputSearchSegment[],
): TemplateResult {
  return html`${segments.map((segment) => {
    if (segment.kind === "text") {
      return html`<span class="terminal-output-segment"
        >${segment.value}</span
      >`;
    }

    return html`<mark
      class="terminal-output-segment"
      part=${segment.active
        ? "search-match active-search-match"
        : "search-match"}
      data-active=${String(segment.active)}
      data-testid=${segment.active ? "tp-screen-active-search-match" : nothing}
      >${segment.value}</mark
    >`;
  })}`;
}

function isViewportAtBottom(viewport: HTMLElement): boolean {
  return (
    viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 2
  );
}

export function shouldAutoLoadMoreHistoryFromViewport(
  viewport: Pick<HTMLElement, "scrollTop">,
  canLoadMoreHistory: boolean,
  historyLoadState: TerminalScreenHistoryLoadState,
): boolean {
  return (
    canLoadMoreHistory &&
    historyLoadState === "idle" &&
    viewport.scrollTop <= HISTORY_AUTO_LOAD_TOP_THRESHOLD_PX
  );
}

interface HistoryScrollAnchor {
  readonly viewport: HTMLElement;
  readonly scrollHeight: number;
  readonly scrollTop: number;
}

function captureHistoryScrollAnchor(
  viewport: HTMLElement | null,
): HistoryScrollAnchor | null {
  if (!viewport) {
    return null;
  }

  return {
    viewport,
    scrollHeight: viewport.scrollHeight,
    scrollTop: viewport.scrollTop,
  };
}

function restoreHistoryScrollAnchor(anchor: HistoryScrollAnchor): void {
  anchor.viewport.scrollTop = resolveScrollTopAfterHistoryPrepend(
    anchor.scrollHeight,
    anchor.scrollTop,
    anchor.viewport.scrollHeight,
  );
}

export function resolveScrollTopAfterHistoryPrepend(
  previousScrollHeight: number,
  previousScrollTop: number,
  nextScrollHeight: number,
): number {
  return Math.max(
    0,
    previousScrollTop + Math.max(0, nextScrollHeight - previousScrollHeight),
  );
}

function createTerminalClientEventId(prefix: string): string {
  return `${prefix}:${
    globalThis.crypto?.randomUUID?.() ??
    `${Date.now()}-${Math.random().toString(36).slice(2)}`
  }`;
}
