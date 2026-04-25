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

type ScreenCopyState = "idle" | "copied" | "failed";
type TerminalScreenPlacement = "panel" | "terminal";

export class TerminalScreenElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    placement: { type: String },
    followOutput: { state: true },
    searchQuery: { state: true },
    activeSearchMatchIndex: { state: true },
    copyState: { state: true },
    directInputActivity: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .screen {
        display: grid;
        gap: var(--tp-space-3);
        padding: var(--tp-terminal-screen-panel-padding, var(--tp-space-4));
        padding-bottom: var(--tp-terminal-screen-panel-padding-bottom, var(--tp-space-4));
        border-bottom-left-radius: var(
          --tp-terminal-screen-panel-border-bottom-left-radius,
          var(--tp-radius-md)
        );
        border-bottom-right-radius: var(
          --tp-terminal-screen-panel-border-bottom-right-radius,
          var(--tp-radius-md)
        );
        box-shadow: var(--tp-terminal-screen-panel-shadow, var(--tp-shadow-panel));
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--tp-color-bg-inset) 92%, transparent), var(--tp-color-bg)),
          var(--tp-color-bg);
        min-height: 18rem;
      }

      .screen[data-placement="terminal"] {
        gap: var(--tp-space-2);
        color: var(--tp-terminal-color-text);
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 92%, transparent),
            var(--tp-terminal-color-bg)
          ),
          var(--tp-terminal-color-bg);
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
        gap: 0.35rem;
      }

      .screen-actions button {
        white-space: nowrap;
      }

      .screen[data-placement="terminal"] .screen-actions button {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, transparent);
        color: var(--tp-terminal-color-text);
        font-size: 0.82rem;
        padding: 0.32rem 0.55rem;
      }

      .screen-actions button[aria-pressed="true"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 52%, transparent);
        color: var(--tp-color-success);
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
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, transparent);
        color: var(--tp-terminal-color-text);
        font-size: 0.84rem;
        padding: 0.34rem 0.55rem;
      }

      .screen[data-placement="terminal"] .search input::placeholder {
        color: color-mix(in srgb, var(--tp-terminal-color-text-muted) 72%, transparent);
      }

      .search input:focus-visible {
        outline: 2px solid color-mix(in srgb, var(--tp-color-accent) 62%, transparent);
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

      .screen[data-placement="terminal"] .search-count[data-search-active="false"] {
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
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, transparent);
        color: var(--tp-terminal-color-text);
        font-size: 0.82rem;
        padding: 0.34rem 0.55rem;
      }

      .viewport {
        margin: 0;
        min-height: var(--tp-terminal-screen-viewport-min-height, clamp(18rem, 42vh, 34rem));
        max-height: var(--tp-terminal-screen-viewport-max-height, min(58vh, 44rem));
        overflow: auto;
        border: 1px solid color-mix(in srgb, var(--tp-color-border) 70%, transparent);
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
        font-size: 0.9rem;
        line-height: 1.48;
        scrollbar-gutter: stable;
      }

      .screen[data-placement="terminal"] .viewport {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.6rem 0.6rem 0 0;
        border-bottom-left-radius: var(--tp-terminal-screen-viewport-border-bottom-left-radius, 0);
        border-bottom-right-radius: var(--tp-terminal-screen-viewport-border-bottom-right-radius, 0);
        box-shadow: inset 0 1px 0 color-mix(in srgb, var(--tp-terminal-color-accent) 18%, transparent);
      }

      .viewport:focus-visible {
        outline: 2px solid color-mix(in srgb, var(--tp-color-accent) 64%, transparent);
        outline-offset: 3px;
      }

      .screen[data-direct-input="true"] .viewport {
        cursor: text;
      }

      .screen[data-font-scale="compact"] .viewport {
        font-size: 0.82rem;
        line-height: 1.42;
      }

      .screen[data-font-scale="large"] .viewport {
        font-size: 1rem;
        line-height: 1.56;
      }

      .screen[data-line-wrap="false"] .text {
        white-space: pre;
        overflow-wrap: normal;
      }

      .line {
        display: grid;
        grid-template-columns: 3.25rem minmax(0, 1fr);
        gap: var(--tp-space-2);
        min-height: 1.35rem;
      }

      .gutter {
        color: color-mix(in srgb, var(--tp-color-text-muted) 48%, transparent);
        text-align: right;
        user-select: none;
      }

      .screen[data-placement="terminal"] .gutter {
        color: color-mix(in srgb, var(--tp-terminal-color-text-muted) 58%, transparent);
      }

      .text {
        white-space: pre-wrap;
        overflow-wrap: anywhere;
      }

      mark {
        border-radius: 0.2rem;
        background: color-mix(in srgb, var(--tp-color-warning) 36%, transparent);
        color: var(--tp-color-text);
        padding: 0 0.08rem;
      }

      mark[data-active="true"] {
        outline: 1px solid color-mix(in srgb, var(--tp-color-warning) 80%, transparent);
        background: color-mix(in srgb, var(--tp-color-warning) 58%, var(--tp-color-bg));
      }

      .screen[data-placement="terminal"] mark {
        color: var(--tp-terminal-color-text);
      }

      .screen[data-placement="terminal"] mark[data-active="true"] {
        background: color-mix(in srgb, var(--tp-color-warning) 58%, var(--tp-terminal-color-bg));
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
        background: color-mix(in srgb, var(--tp-color-panel-raised) 60%, transparent);
      }

      .screen[data-placement="terminal"] .screen-meta {
        color: var(--tp-terminal-color-text-muted);
        gap: 0.35rem;
        font-size: 0.78rem;
      }

      .screen[data-placement="terminal"] .screen-meta span {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 78%, transparent);
        padding: 0.18rem 0.45rem;
      }

      .screen-meta [data-input-tone="ready"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 52%, transparent);
        color: var(--tp-color-success);
      }

      .screen-meta [data-input-tone="pending"] {
        border-color: color-mix(in srgb, var(--tp-color-warning) 56%, transparent);
        color: var(--tp-color-warning);
      }

      .screen-meta [data-input-tone="failed"] {
        border-color: color-mix(in srgb, var(--tp-color-danger) 62%, transparent);
        background: color-mix(in srgb, var(--tp-color-danger-soft) 70%, transparent);
        color: var(--tp-color-danger);
      }

      @media (max-width: 720px) {
        .screen {
          gap: var(--tp-space-2);
          padding: var(--tp-terminal-screen-mobile-panel-padding, var(--tp-space-3));
          padding-bottom: var(--tp-terminal-screen-panel-padding-bottom, var(--tp-space-3));
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

        .viewport {
          min-height: var(--tp-terminal-screen-mobile-viewport-min-height, clamp(14rem, 38vh, 22rem));
          max-height: var(--tp-terminal-screen-mobile-viewport-max-height, min(48vh, 26rem));
          padding: var(--tp-space-2);
        }

        .line {
          grid-template-columns: 2.45rem minmax(0, 1fr);
          gap: var(--tp-space-1);
        }
      }
    `,
  ];

  declare placement: TerminalScreenPlacement;
  protected declare followOutput: boolean;
  protected declare searchQuery: string;
  protected declare activeSearchMatchIndex: number | null;
  protected declare copyState: ScreenCopyState;
  protected declare directInputActivity: TerminalScreenInputActivity;

  #autoScrolling = false;
  #copyStateResetTimer: ReturnType<typeof setTimeout> | null = null;
  #directInputActivityResetTimer: ReturnType<typeof setTimeout> | null = null;
  #directInputQueue = Promise.resolve();
  #directInputBuffer: TerminalDirectInputBuffer;

  constructor() {
    super();
    this.placement = "panel";
    this.followOutput = true;
    this.searchQuery = "";
    this.activeSearchMatchIndex = null;
    this.copyState = "idle";
    this.directInputActivity = "idle";
    this.#directInputBuffer = new TerminalDirectInputBuffer({
      flush: (input) => this.queueDirectInput(input),
    });
  }

  override disconnectedCallback(): void {
    this.clearCopyStateResetTimer();
    this.clearDirectInputActivityResetTimer();
    this.#directInputBuffer.dispose();
    super.disconnectedCallback();
  }

  protected override willUpdate(changedProperties: PropertyValues): void {
    super.willUpdate(changedProperties);
    if (changedProperties.has("snapshot")) {
      this.syncTerminalDisplayAttributes();
    }
  }

  protected override updated(changedProperties: PropertyValues): void {
    const shouldSyncSearch =
      changedProperties.has("snapshot")
      || changedProperties.has("searchQuery")
      || changedProperties.has("activeSearchMatchIndex");
    if (shouldSyncSearch && this.syncActiveSearchMatch()) {
      return;
    }

    if (
      changedProperties.has("snapshot")
      || changedProperties.has("followOutput")
    ) {
      if (this.followOutput && this.snapshot.attachedSession?.focused_screen) {
        this.scrollViewportToBottom();
      }
    }
  }

  override render() {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    const screen = controls.screen;
    const inputStatus = resolveTerminalScreenInputStatus(controls, this.directInputActivity);
    const searchResult = this.createSearchResult();
    const terminalDisplay = this.snapshot.terminalDisplay;
    const isTerminalPlacement = this.placement === "terminal";

    return html`
      <div
        class="panel screen"
        part="screen"
        data-testid="tp-terminal-screen"
        data-placement=${this.placement}
        data-font-scale=${terminalDisplay.fontScale}
        data-line-wrap=${String(terminalDisplay.lineWrap)}
        data-direct-input=${String(controls.canUseDirectInput)}
        data-input-capability=${controls.inputCapabilityStatus}
        data-input-status=${inputStatus.tone}
      >
        <div class="screen-header">
          <div class="panel-header">
            ${isTerminalPlacement ? nothing : html`<div class="panel-eyebrow">Terminal</div>`}
            <div class="panel-title">${screen?.surface.title ?? "Live output"}</div>
            ${isTerminalPlacement ? nothing : html`<div class="panel-copy">Focused pane output.</div>`}
          </div>
          <div class="screen-actions" part="screen-actions">
            <button
              type="button"
              data-testid="tp-screen-follow"
              aria-pressed=${String(this.followOutput)}
              ?disabled=${!screen}
              @click=${() => this.toggleFollowOutput()}
            >
              ${this.followOutput ? "Following" : "Paused"}
            </button>
            <button
              type="button"
              data-testid="tp-screen-scroll-latest"
              ?disabled=${!screen}
              @click=${() => this.scrollLatest()}
            >
              Scroll latest
            </button>
            <button
              type="button"
              data-testid="tp-screen-copy"
              ?disabled=${!controls.canCopyVisibleOutput}
              @click=${() => this.copyVisibleOutput()}
            >
              ${this.copyState === "copied" ? "Copied" : this.copyState === "failed" ? "Copy failed" : "Copy visible"}
            </button>
          </div>
        </div>
        ${screen
          ? html`
              <div class="screen-meta" part="meta">
                <span>${screen.cols} columns</span>
                <span>${screen.rows} rows</span>
                <span>seq ${String(screen.sequence)}</span>
                <span>${screen.source}</span>
                <span>${terminalDisplay.fontScale}</span>
                <span>${terminalDisplay.lineWrap ? "wrapped" : "nowrap"}</span>
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
                ${screen.surface.cursor
                  ? html`<span>cursor ${screen.surface.cursor.row + 1}:${screen.surface.cursor.col + 1}</span>`
                  : null}
              </div>
              <div class="screen-tools" part="screen-tools">
                <label class="search" part="search">
                  <input
                    data-testid="tp-screen-search"
                    name="tp-screen-search"
                    .value=${this.searchQuery}
                    placeholder="Find output"
                    aria-label="Find terminal output"
                    aria-keyshortcuts="Control+F Meta+F"
                    @input=${(event: Event) => this.handleSearchInput(event)}
                    @keydown=${(event: KeyboardEvent) => this.handleSearchKeydown(event)}
                  />
                  <span
                    class="search-count"
                    part="search-count"
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
                <div class="search-actions" part="search-actions">
                  <button
                    type="button"
                    data-testid="tp-screen-search-prev"
                    ?disabled=${searchResult.matchCount === 0}
                    @click=${() => this.selectSearchMatch("previous")}
                  >
                    Prev
                  </button>
                  <button
                    type="button"
                    data-testid="tp-screen-search-next"
                    ?disabled=${searchResult.matchCount === 0}
                    @click=${() => this.selectSearchMatch("next")}
                  >
                    Next
                  </button>
                  <button
                    type="button"
                    data-testid="tp-screen-search-clear"
                    ?disabled=${!searchResult.query}
                    @click=${() => this.clearSearch()}
                  >
                    Clear
                  </button>
                </div>
              </div>
              <div
                class="viewport"
                part="screen-lines"
                data-testid="tp-screen-viewport"
                tabindex=${controls.canUseDirectInput ? "0" : nothing}
                role="region"
                aria-describedby="tp-screen-input-status"
                aria-keyshortcuts="Control+F Meta+F"
                aria-label=${controls.canUseDirectInput
                  ? "Terminal output and focused pane input"
                  : "Terminal output"}
                @keydown=${(event: KeyboardEvent) => this.handleViewportKeydown(event)}
                @scroll=${(event: Event) => this.handleViewportScroll(event)}
              >
                ${searchResult.lines.map((line) => renderLine(line.lineIndex + 1, line.segments))}
              </div>
            `
          : html`<div class="empty-state" part="empty">No active screen yet. Start or attach a session to see output here.</div>`}
      </div>
    `;
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

  private handleSearchKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
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
    this.activeSearchMatchIndex = direction === "next"
      ? (currentMatchIndex + 1) % searchResult.matchCount
      : (currentMatchIndex - 1 + searchResult.matchCount) % searchResult.matchCount;
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

    const output = serializeTerminalOutputLines(screen.surface.lines.map((line) => line.text));
    try {
      await writeClipboardText(output);
      this.setCopyState("copied");
      this.dispatchEvent(
        new CustomEvent("tp-terminal-screen-copied", {
          bubbles: true,
          composed: true,
          detail: { paneId: screen.pane_id, lineCount: screen.surface.lines.length },
        }),
      );
    } catch (error) {
      this.setCopyState("failed");
      this.dispatchEvent(
        new CustomEvent("tp-terminal-screen-copy-failed", {
          bubbles: true,
          composed: true,
          detail: { paneId: screen.pane_id, error },
        }),
      );
    }
  }

  private setCopyState(copyState: ScreenCopyState): void {
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

    const activeSearchMatchIndex = resolveTerminalOutputSearchMatchIndex(
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
    const activeMatch = this.shadowRoot?.querySelector<HTMLElement>('[data-testid="tp-screen-active-search-match"]');
    activeMatch?.scrollIntoView({
      block: "center",
      inline: "nearest",
    });
  }

  private createSearchResult(searchQuery = this.searchQuery): TerminalOutputSearchResult {
    const screen = this.snapshot.attachedSession?.focused_screen;
    return createTerminalOutputSearchResult(
      screen ? screen.surface.lines.map((line) => line.text) : [],
      searchQuery,
      { activeMatchIndex: this.activeSearchMatchIndex },
    );
  }

  private syncTerminalDisplayAttributes(): void {
    this.setAttribute("data-font-scale", this.snapshot.terminalDisplay.fontScale);
    this.setAttribute("data-line-wrap", String(this.snapshot.terminalDisplay.lineWrap));
  }

  private handleViewportScroll(event: Event): void {
    if (this.#autoScrolling) {
      return;
    }

    const viewport = event.currentTarget as HTMLElement;
    if (!isViewportAtBottom(viewport)) {
      this.followOutput = false;
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

  private queueDirectInput(input: string): void {
    this.#directInputQueue = this.#directInputQueue
      .catch(() => undefined)
      .then(() => this.dispatchDirectInput(input));
  }

  private async dispatchDirectInput(input: string): Promise<void> {
    const controls = resolveTerminalScreenControlState(this.snapshot);
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canUseDirectInput) {
      return;
    }

    try {
      await this.kernel?.commands.dispatchMuxCommand(controls.activeSessionId, {
        kind: "send_input",
        pane_id: controls.activePaneId,
        data: input,
      });
      if (shouldRefreshAfterTerminalDirectInput(input)) {
        await this.kernel?.commands.attachSession(controls.activeSessionId);
      }
      if (this.directInputActivity !== "idle") {
        this.setDirectInputActivity("idle");
      }
      this.dispatchEvent(
        new CustomEvent("tp-terminal-screen-input-submitted", {
          bubbles: true,
          composed: true,
          detail: {
            sessionId: controls.activeSessionId,
            paneId: controls.activePaneId,
            inputLength: input.length,
          },
        }),
      );
    } catch (error) {
      this.setDirectInputActivity("failed");
      this.dispatchEvent(
        new CustomEvent("tp-terminal-screen-input-failed", {
          bubbles: true,
          composed: true,
          detail: {
            sessionId: controls.activeSessionId,
            paneId: controls.activePaneId,
            error,
          },
        }),
      );
    }
  }

  private scrollViewportToBottom(): void {
    const viewport = this.shadowRoot?.querySelector<HTMLElement>('[data-testid="tp-screen-viewport"]');
    if (!viewport) {
      return;
    }

    this.#autoScrolling = true;
    viewport.scrollTop = viewport.scrollHeight;
    requestAnimationFrame(() => {
      this.#autoScrolling = false;
    });
  }

  private focusSearchInput(): void {
    const searchInput = this.shadowRoot?.querySelector<HTMLInputElement>('[data-testid="tp-screen-search"]');
    if (!searchInput || searchInput.disabled) {
      return;
    }

    searchInput.focus({ preventScroll: true });
    searchInput.select();
  }

  private focusViewport(): void {
    const viewport = this.shadowRoot?.querySelector<HTMLElement>('[data-testid="tp-screen-viewport"]');
    viewport?.focus({ preventScroll: true });
  }
}

function renderLine(
  index: number,
  segments: readonly TerminalOutputSearchSegment[],
): TemplateResult {
  return html`
    <div class="line" part="screen-line">
      <span class="gutter" part="line-number">${index}</span>
      <span class="text" part="line-text">${renderHighlightedSegments(segments)}</span>
    </div>
  `;
}

function renderHighlightedSegments(
  segments: readonly TerminalOutputSearchSegment[],
): TemplateResult {
  return html`${segments.map((segment) => {
    if (segment.kind === "text") {
      return segment.value;
    }

    return html`
      <mark
        part=${segment.active ? "search-match active-search-match" : "search-match"}
        data-active=${String(segment.active)}
        data-testid=${segment.active ? "tp-screen-active-search-match" : nothing}
      >
        ${segment.value}
      </mark>
    `;
  })}`;
}

function isViewportAtBottom(viewport: HTMLElement): boolean {
  return viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 2;
}
