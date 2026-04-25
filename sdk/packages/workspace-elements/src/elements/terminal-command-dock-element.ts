import { css, html, nothing } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { readClipboardText } from "./terminal-clipboard.js";
import { resolveTerminalCommandInputStatus } from "./terminal-command-input-status.js";
import {
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";
import { resolveTerminalCommandDockControlState } from "./terminal-command-dock-controls.js";

export class TerminalCommandDockElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    quickCommands: { attribute: false },
    pending: { state: true },
    actionError: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .dock {
        display: grid;
        gap: var(--tp-space-2);
        padding: var(--tp-space-3);
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--tp-color-panel-raised) 82%, transparent), transparent),
          var(--tp-color-panel);
      }

      .dock .panel-header {
        margin-bottom: 0;
      }

      .dock-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: var(--tp-space-3);
      }

      .dock-footer {
        display: grid;
        grid-template-columns: minmax(12rem, 1fr) max-content;
        align-items: flex-start;
        gap: var(--tp-space-3);
      }

      .dock-footer .actions {
        flex-wrap: nowrap;
        justify-content: flex-end;
      }

      .dock-status {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        justify-content: flex-end;
      }

      .chip-row,
      .history-actions,
      .actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
      }

      .chip {
        color: var(--tp-color-text-muted);
        font-family: var(--tp-font-family-mono);
        font-size: 0.82rem;
        padding: 0.35rem 0.55rem;
      }

      .history-row {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: var(--tp-space-2);
        align-items: center;
      }

      .history-label {
        color: var(--tp-color-text-muted);
        font-size: 0.74rem;
        font-weight: 700;
        letter-spacing: 0;
        text-transform: uppercase;
      }

      .history-chip {
        max-width: min(22rem, 100%);
        justify-content: flex-start;
        color: var(--tp-color-text);
        font-family: var(--tp-font-family-mono);
        font-size: 0.82rem;
        padding: 0.34rem 0.55rem;
      }

      .history-command {
        display: block;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .composer {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: var(--tp-space-2);
        align-items: stretch;
        border: 1px solid color-mix(in srgb, var(--tp-color-border) 82%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-bg) 74%, transparent);
        padding: var(--tp-space-2);
      }

      .prompt {
        color: var(--tp-color-accent);
        font-family: var(--tp-font-family-mono);
        font-weight: 700;
        padding-top: 0.55rem;
      }

      textarea {
        min-height: 3.15rem;
        resize: vertical;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--tp-color-text);
        font: 0.94rem/1.45 var(--tp-font-family-mono);
      }

      textarea::placeholder {
        color: color-mix(in srgb, var(--tp-color-text-muted) 72%, transparent);
      }

      textarea:disabled {
        cursor: not-allowed;
      }

      .hint {
        color: var(--tp-color-text-muted);
        font-size: 0.84rem;
        line-height: 1.45;
      }

      .primary {
        border-color: color-mix(in srgb, var(--tp-color-accent) 52%, transparent);
        background: color-mix(in srgb, var(--tp-color-accent) 18%, var(--tp-color-panel-raised));
      }

      .badge {
        display: inline-flex;
        align-items: center;
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        background: color-mix(in srgb, var(--tp-color-panel-raised) 68%, transparent);
        color: var(--tp-color-text-muted);
        font-size: 0.78rem;
        padding: 0.22rem 0.55rem;
      }

      .badge[data-tone="ready"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 55%, transparent);
        color: var(--tp-color-success);
      }

      .badge[data-tone="pending"] {
        border-color: color-mix(in srgb, var(--tp-color-warning) 55%, transparent);
        color: var(--tp-color-warning);
      }

      .notice {
        border: 1px solid color-mix(in srgb, var(--tp-color-warning) 45%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-warning) 10%, transparent);
        color: var(--tp-color-text);
        padding: var(--tp-space-3);
      }

      details {
        border-top: 1px solid var(--tp-color-border);
        padding-top: var(--tp-space-3);
      }

      summary {
        cursor: pointer;
        color: var(--tp-color-text-muted);
        font-weight: 600;
      }

      @media (max-width: 720px) {
        .dock-header {
          display: grid;
        }

        .dock-footer {
          grid-template-columns: 1fr;
        }

        .dock-footer .actions {
          flex-wrap: wrap;
          justify-content: flex-start;
        }

        .dock-status {
          justify-content: flex-start;
        }
      }
    `,
  ];

  declare quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined;
  protected declare pending: boolean;
  protected declare actionError: string | null;

  #historyCursor: number | null = null;
  #historyDraftBeforeNavigation = "";

  constructor() {
    super();
    this.quickCommands = defaultTerminalCommandQuickCommands;
    this.pending = false;
    this.actionError = null;
  }

  override render() {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    const quickCommands = resolveTerminalCommandQuickCommands(this.quickCommands);
    const inputStatus = resolveTerminalCommandInputStatus(controls);
    const pasteTitle = controls.pasteCapabilityStatus === "known" && !controls.canPasteClipboard
      ? "Paste is not supported by the active backend"
      : "Paste clipboard into the focused pane";

    return html`
      <div
        class="panel dock"
        part="command-dock"
        data-testid="tp-command-dock"
        data-command-input=${String(controls.canWriteInput)}
        data-input-capability=${controls.inputCapabilityStatus}
      >
        <div class="dock-header">
          <div class="panel-header">
            <div class="panel-eyebrow">Command Input</div>
            <div class="panel-title">Focused pane command lane</div>
            <div class="panel-copy">Send shell input to the selected pane without leaving the workspace.</div>
          </div>

          <div class="dock-status" part="status">
            <span class="badge" data-tone=${controls.activePaneId ? "ready" : "idle"}>
              ${controls.activePaneId ? `Pane ${controls.activePaneId}` : "No pane"}
            </span>
            <span
              class="badge"
              data-testid="tp-command-input-status"
              data-tone=${inputStatus.tone}
              title=${inputStatus.title}
            >
              ${inputStatus.label}
            </span>
            <span class="badge" data-testid="tp-command-history-count">
              ${controls.commandHistory.length} history
            </span>
          </div>
        </div>

        ${quickCommands.length > 0
          ? html`
              <div class="chip-row" part="quick-commands" aria-label="Quick commands">
                ${quickCommands.map(
                  (command) => html`
                    <button
                      class="chip"
                      type="button"
                      part="quick-command"
                      data-testid="tp-quick-command"
                      title=${command.description ?? `Insert ${command.label}`}
                      ?disabled=${!controls.canWriteInput}
                      @click=${() => this.setDraft(command.value)}
                    >
                      ${command.label}
                    </button>
                  `,
                )}
              </div>
            `
          : nothing}

        ${controls.recentCommands.length > 0
          ? html`
              <div class="history-row" part="command-history" aria-label="Recent commands">
                <span class="history-label">Recent</span>
                <div class="history-actions">
                  ${controls.recentCommands.map(
                    (command, index) => html`
                      <button
                        class="history-chip"
                        type="button"
                        data-testid="tp-command-history-entry"
                        data-history-index=${index}
                        title=${command}
                        ?disabled=${!controls.activePaneId || this.pending}
                        @click=${() => this.setDraft(command)}
                      >
                        <span class="history-command">${command}</span>
                      </button>
                    `,
                  )}
                </div>
              </div>
            `
          : nothing}

        <label class="composer" part="composer">
          <span class="prompt" aria-hidden="true">&gt;_</span>
          <textarea
            data-testid="tp-command-input"
            name="tp-command-input"
            .value=${controls.draft}
            ?disabled=${!controls.canWriteInput}
            placeholder=${inputStatus.placeholder}
            aria-label="Focused pane command input"
            @input=${(event: Event) => this.handleInput(event)}
            @keydown=${(event: KeyboardEvent) => this.handleKeydown(event)}
          ></textarea>
        </label>

        ${this.actionError
          ? html`
              <div class="notice" part="error">
                <strong>Command failed</strong>
                <div>${this.actionError}</div>
              </div>
            `
          : nothing}

        <div class="dock-footer">
          <div class="hint" part="hint">
            ${inputStatus.hint}
          </div>

          <div class="actions">
            <button
              class="primary"
              data-testid="tp-send-command"
              ?disabled=${!controls.canSend}
              @click=${() => this.sendDraft()}
            >
              Send command
            </button>
            <button
              data-testid="tp-paste-clipboard"
              title=${pasteTitle}
              ?disabled=${!controls.canPasteClipboard}
              @click=${() => this.pasteClipboard()}
            >
              Paste
            </button>
            <button
              data-testid="tp-send-interrupt"
              ?disabled=${!controls.canWriteInput}
              @click=${() => this.sendShortcut("\u0003")}
            >
              Ctrl+C
            </button>
            <button
              data-testid="tp-send-enter"
              ?disabled=${!controls.canWriteInput}
              @click=${() => this.sendShortcut("\r")}
            >
              Enter
            </button>
          </div>
        </div>

        <details part="session-tools" data-testid="tp-session-tools">
          <summary>Session tools</summary>
          <div class="actions">
            <button
              data-testid="tp-save-layout"
              ?disabled=${!controls.activeSessionId || this.pending}
              @click=${() => this.saveLayout()}
            >
              Save layout
            </button>
            <button
              data-testid="tp-refresh-terminal"
              ?disabled=${!controls.activeSessionId || this.pending}
              @click=${() => this.refreshActiveSession()}
            >
              Refresh terminal
            </button>
            <button
              data-testid="tp-clear-command-history"
              ?disabled=${controls.commandHistory.length === 0 || this.pending}
              @click=${() => this.clearCommandHistory()}
            >
              Clear history
            </button>
          </div>
        </details>
      </div>
    `;
  }

  private setDraft(value: string): void {
    const paneId = this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;
    if (!paneId) {
      return;
    }

    this.actionError = null;
    this.resetHistoryNavigation();
    this.kernel?.commands.updateDraft(paneId, value);
  }

  private handleInput(event: Event): void {
    const target = event.currentTarget as HTMLTextAreaElement;
    this.setDraft(target.value);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }

    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const target = event.currentTarget as HTMLTextAreaElement;
      if (this.navigateCommandHistory(event.key === "ArrowUp" ? "previous" : "next", target)) {
        event.preventDefault();
      }
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void this.sendDraft();
    }
  }

  private async sendDraft(): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canSend) {
      return;
    }

    await this.dispatchInput(controls.activeSessionId, controls.activePaneId, `${controls.draft}\n`);
    this.recordCommandHistory(controls.draft);
    this.resetHistoryNavigation();
    this.kernel?.commands.clearDraft(controls.activePaneId);
  }

  private navigateCommandHistory(direction: "previous" | "next", target: HTMLTextAreaElement): boolean {
    const paneId = this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;
    const commandHistory = this.snapshot.commandHistory.entries;
    if (!paneId || commandHistory.length === 0 || !this.canNavigateHistory(direction, target)) {
      return false;
    }

    if (direction === "previous") {
      if (this.#historyCursor === null) {
        this.#historyDraftBeforeNavigation = target.value;
      }

      this.#historyCursor = this.#historyCursor === null
        ? commandHistory.length - 1
        : Math.max(0, this.#historyCursor - 1);
    } else {
      if (this.#historyCursor === null) {
        return false;
      }

      if (this.#historyCursor === commandHistory.length - 1) {
        this.#historyCursor = null;
        this.applyHistoryDraft(paneId, target, this.#historyDraftBeforeNavigation);
        this.#historyDraftBeforeNavigation = "";
        return true;
      }

      this.#historyCursor += 1;
    }

    const historyDraft = commandHistory[this.#historyCursor];
    if (!historyDraft) {
      return false;
    }

    this.applyHistoryDraft(paneId, target, historyDraft);
    return true;
  }

  private canNavigateHistory(direction: "previous" | "next", target: HTMLTextAreaElement): boolean {
    const selectionStart = target.selectionStart ?? target.value.length;
    const selectionEnd = target.selectionEnd ?? selectionStart;
    if (selectionStart !== selectionEnd) {
      return false;
    }

    if (direction === "previous") {
      return !target.value.slice(0, selectionStart).includes("\n");
    }

    return !target.value.slice(selectionEnd).includes("\n");
  }

  private applyHistoryDraft(paneId: string, target: HTMLTextAreaElement, value: string): void {
    target.value = value;
    target.setSelectionRange(value.length, value.length);
    this.kernel?.commands.updateDraft(paneId, value);
  }

  private recordCommandHistory(value: string): void {
    this.kernel?.commands.recordCommandHistory(value);
  }

  private resetHistoryNavigation(): void {
    this.#historyCursor = null;
    this.#historyDraftBeforeNavigation = "";
  }

  private async pasteClipboard(): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canPasteClipboard) {
      return;
    }

    this.pending = true;
    this.actionError = null;

    let pastedText = "";
    try {
      pastedText = await readClipboardText();
    } catch (error) {
      this.pending = false;
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-paste-failed", {
          bubbles: true,
          composed: true,
          detail: {
            sessionId: controls.activeSessionId,
            paneId: controls.activePaneId,
            error,
          },
        }),
      );
      return;
    }

    if (pastedText.length === 0) {
      this.pending = false;
      return;
    }

    await this.dispatchPaste(controls.activeSessionId, controls.activePaneId, pastedText);
  }

  private async sendShortcut(data: string): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canWriteInput) {
      return;
    }

    await this.dispatchInput(controls.activeSessionId, controls.activePaneId, data);
  }

  private async dispatchPaste(sessionId: string, paneId: string, data: string): Promise<void> {
    this.pending = true;
    this.actionError = null;

    try {
      await this.kernel?.commands.dispatchMuxCommand(sessionId, {
        kind: "send_paste",
        pane_id: paneId,
        data,
      });
      await this.kernel?.commands.attachSession(sessionId);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-paste-submitted", {
          bubbles: true,
          composed: true,
          detail: { sessionId, paneId, inputLength: data.length },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-paste-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, paneId, error },
        }),
      );
    } finally {
      this.pending = false;
    }
  }

  private async dispatchInput(sessionId: string, paneId: string, data: string): Promise<void> {
    this.pending = true;
    this.actionError = null;

    try {
      await this.kernel?.commands.dispatchMuxCommand(sessionId, {
        kind: "send_input",
        pane_id: paneId,
        data,
      });
      await this.kernel?.commands.attachSession(sessionId);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-command-submitted", {
          bubbles: true,
          composed: true,
          detail: { sessionId, paneId },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-command-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, paneId, error },
        }),
      );
    } finally {
      this.pending = false;
    }
  }

  private async saveLayout(): Promise<void> {
    const sessionId = this.snapshot.selection.activeSessionId ?? this.snapshot.attachedSession?.session.session_id ?? null;
    if (!sessionId || this.pending) {
      return;
    }

    this.pending = true;
    this.actionError = null;

    try {
      await this.kernel?.commands.dispatchMuxCommand(sessionId, { kind: "save_session" });
      await this.kernel?.commands.refreshSavedSessions();
      const savedSessions = this.kernel?.getSnapshot().catalog.savedSessions ?? [];
      this.dispatchEvent(
        new CustomEvent("tp-terminal-layout-saved", {
          bubbles: true,
          composed: true,
          detail: {
            sessionId,
            savedSessionCount: savedSessions.length,
            savedSessionId: savedSessions[0]?.session_id ?? null,
          },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-layout-save-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, error },
        }),
      );
    } finally {
      this.pending = false;
    }
  }

  private async refreshActiveSession(): Promise<void> {
    const sessionId = this.snapshot.selection.activeSessionId ?? this.snapshot.attachedSession?.session.session_id ?? null;
    if (!sessionId || this.pending) {
      return;
    }

    this.pending = true;
    this.actionError = null;

    try {
      await this.kernel?.commands.attachSession(sessionId);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-session-refreshed", {
          bubbles: true,
          composed: true,
          detail: { sessionId },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-terminal-session-refresh-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, error },
        }),
      );
    } finally {
      this.pending = false;
    }
  }

  private clearCommandHistory(): void {
    this.resetHistoryNavigation();
    this.kernel?.commands.clearCommandHistory();
    this.dispatchEvent(
      new CustomEvent("tp-terminal-command-history-cleared", {
        bubbles: true,
        composed: true,
      }),
    );
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace command failed";
}
