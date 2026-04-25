import { css, html, nothing, type PropertyValues } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { readClipboardText } from "./terminal-clipboard.js";
import { resolveTerminalCommandInputStatus } from "./terminal-command-input-status.js";
import { TERMINAL_DESTRUCTIVE_CONFIRMATION_RESET_MS } from "./terminal-destructive-action.js";
import {
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";
import { resolveTerminalCommandDockControlState } from "./terminal-command-dock-controls.js";
import { resolveTerminalEntityIdLabel } from "./terminal-identity.js";

type TerminalCommandInputFocusOptions = {
  focusInput?: boolean;
};

type TerminalCommandDockPlacement = "panel" | "terminal";

export class TerminalCommandDockElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    quickCommands: { attribute: false },
    autoFocusInput: { attribute: "auto-focus-input", type: Boolean },
    placement: { type: String },
    pending: { state: true },
    actionError: { state: true },
    historyClearConfirmationArmed: { state: true },
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

      .dock[data-placement="terminal"] {
        border-top-width: 0;
        border-top-left-radius: 0;
        border-top-right-radius: 0;
        box-shadow: none;
        color: var(--tp-terminal-color-text);
        gap: var(--tp-space-2);
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg) 86%, transparent),
            var(--tp-terminal-color-bg)
          ),
          var(--tp-terminal-color-bg);
        padding: 0 var(--tp-space-4) var(--tp-space-3);
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

      .dock[data-placement="terminal"] .dock-header {
        order: 3;
        align-items: center;
        justify-content: flex-start;
        min-height: 1.35rem;
      }

      .dock-footer {
        display: grid;
        grid-template-columns: minmax(12rem, 1fr) minmax(0, auto);
        align-items: flex-start;
        gap: var(--tp-space-3);
      }

      .dock[data-placement="terminal"] .dock-footer {
        order: 5;
        grid-template-columns: 1fr;
        align-items: center;
        gap: var(--tp-space-2);
      }

      .dock[data-placement="terminal"] .dock-footer .actions {
        justify-content: flex-end;
      }

      .dock-footer .actions {
        min-width: 0;
        flex-wrap: wrap;
        justify-content: flex-end;
      }

      .dock-status {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        justify-content: flex-end;
      }

      .dock[data-placement="terminal"] .dock-status {
        justify-content: flex-start;
      }

      .chip-row,
      .history-actions,
      .actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
      }

      .dock[data-placement="terminal"] .chip-row,
      .dock[data-placement="terminal"] .history-actions {
        gap: 0.35rem;
      }

      .dock[data-placement="terminal"] .chip-row {
        order: 4;
      }

      .chip {
        color: var(--tp-color-text-muted);
        font-family: var(--tp-font-family-mono);
        font-size: 0.82rem;
        padding: 0.35rem 0.55rem;
      }

      .dock[data-placement="terminal"] .chip {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 78%, transparent);
        color: var(--tp-terminal-color-text-muted);
        padding: 0.3rem 0.5rem;
      }

      .history-row {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr);
        gap: var(--tp-space-2);
        align-items: center;
      }

      .dock[data-placement="terminal"] .history-row {
        order: 4;
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

      .dock[data-placement="terminal"] .history-chip {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 78%, transparent);
        color: var(--tp-terminal-color-text);
        padding: 0.3rem 0.5rem;
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

      .dock[data-placement="terminal"] .composer {
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 36%,
          var(--tp-terminal-color-border)
        );
        order: 1;
        border-top-width: 0;
        border-radius: 0 0 0.6rem 0.6rem;
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 86%, transparent),
            transparent
          ),
          color-mix(in srgb, var(--tp-terminal-color-bg) 92%, var(--tp-terminal-color-bg-raised));
        padding: 0.48rem 0.62rem;
      }

      .prompt {
        color: var(--tp-color-accent);
        font-family: var(--tp-font-family-mono);
        font-weight: 700;
        padding-top: 0.55rem;
      }

      .dock[data-placement="terminal"] .prompt {
        color: var(--tp-terminal-color-accent);
        padding-top: 0.28rem;
      }

      textarea {
        min-width: 0;
        min-height: 3.15rem;
        resize: vertical;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--tp-color-text);
        font: 0.94rem/1.45 var(--tp-font-family-mono);
      }

      .dock[data-placement="terminal"] textarea {
        color: var(--tp-terminal-color-text);
        min-height: 2rem;
        resize: none;
      }

      textarea::placeholder {
        color: color-mix(in srgb, var(--tp-color-text-muted) 72%, transparent);
      }

      .dock[data-placement="terminal"] textarea::placeholder {
        color: color-mix(in srgb, var(--tp-terminal-color-text-muted) 78%, transparent);
      }

      textarea:disabled {
        cursor: not-allowed;
      }

      .hint {
        color: var(--tp-color-text-muted);
        font-size: 0.84rem;
        line-height: 1.45;
      }

      .dock[data-placement="terminal"] .hint {
        display: none;
      }

      .dock[data-placement="terminal"] .actions button {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, transparent);
        color: var(--tp-terminal-color-text);
      }

      .primary {
        border-color: color-mix(in srgb, var(--tp-color-accent) 52%, transparent);
        background: color-mix(in srgb, var(--tp-color-accent) 18%, var(--tp-color-panel-raised));
      }

      .dock[data-placement="terminal"] .primary {
        border-color: color-mix(in srgb, var(--tp-terminal-color-accent) 54%, transparent);
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 16%,
          var(--tp-terminal-color-bg-raised)
        );
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

      .dock[data-placement="terminal"] .badge {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 78%, transparent);
        color: var(--tp-terminal-color-text-muted);
        padding: 0.2rem 0.48rem;
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

      .dock[data-placement="terminal"] .notice {
        order: 2;
      }

      details {
        border-top: 1px solid var(--tp-color-border);
        padding-top: var(--tp-space-3);
      }

      .dock[data-placement="terminal"] details {
        order: 6;
        border-top-color: color-mix(in srgb, var(--tp-terminal-color-border) 58%, transparent);
        padding-top: var(--tp-space-2);
      }

      summary {
        cursor: pointer;
        color: var(--tp-color-text-muted);
        font-weight: 600;
      }

      .dock[data-placement="terminal"] summary {
        color: var(--tp-terminal-color-text-muted);
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

      @media (max-width: 720px) {
        .dock[data-placement="terminal"] {
          padding: var(--tp-space-2);
        }

        .dock[data-placement="terminal"] .dock-footer {
          grid-template-columns: 1fr;
        }
      }
    `,
  ];

  declare quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined;
  declare autoFocusInput: boolean;
  declare placement: TerminalCommandDockPlacement;
  protected declare pending: boolean;
  protected declare actionError: string | null;
  protected declare historyClearConfirmationArmed: boolean;

  #historyCursor: number | null = null;
  #historyDraftBeforeNavigation = "";
  #historyClearConfirmationResetTimer: ReturnType<typeof setTimeout> | null = null;
  #lastAutoFocusedPaneId: string | null = null;

  constructor() {
    super();
    this.quickCommands = defaultTerminalCommandQuickCommands;
    this.autoFocusInput = false;
    this.placement = "panel";
    this.pending = false;
    this.actionError = null;
    this.historyClearConfirmationArmed = false;
  }

  override disconnectedCallback(): void {
    this.clearHistoryClearConfirmationResetTimer();
    super.disconnectedCallback();
  }

  protected override updated(changedProperties: PropertyValues): void {
    if (
      changedProperties.has("snapshot")
      || changedProperties.has("pending")
      || changedProperties.has("autoFocusInput")
    ) {
      this.maybeAutoFocusInput();
    }
  }

  override render() {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    const quickCommands = resolveTerminalCommandQuickCommands(this.quickCommands);
    const inputStatus = resolveTerminalCommandInputStatus(controls);
    const pasteTitle = controls.pasteCapabilityStatus === "known" && !controls.canPasteClipboard
      ? "Paste is not supported by the active backend"
      : "Paste clipboard into the focused pane";
    const saveLayoutTitle = controls.saveCapabilityStatus === "known" && !controls.canSaveLayout
      ? "Save layout is not supported by the active backend"
      : controls.saveCapabilityStatus === "unknown"
        ? "Save layout is disabled until backend capabilities load"
        : "Save the focused session layout";
    const activePaneIdentity = controls.activePaneId
      ? resolveTerminalEntityIdLabel(controls.activePaneId, { prefix: "Pane" })
      : null;
    const historyCountLabel = formatCommandHistoryCount(controls.commandHistory.length);
    const isHistoryClearConfirming =
      this.historyClearConfirmationArmed && controls.commandHistory.length > 0 && !this.pending;
    const clearHistoryTitle = isHistoryClearConfirming
      ? `Confirm clearing ${historyCountLabel}`
      : `Clear ${historyCountLabel}`;

    return html`
      <div
        class="panel dock"
        part="command-dock"
        data-testid="tp-command-dock"
        data-placement=${this.placement}
        data-command-input=${String(controls.canWriteInput)}
        data-input-capability=${controls.inputCapabilityStatus}
        data-save-capability=${controls.saveCapabilityStatus}
        data-save-layout=${String(controls.canSaveLayout)}
      >
        <div class="dock-header">
          ${this.placement === "terminal"
            ? nothing
            : html`
                <div class="panel-header">
                  <div class="panel-eyebrow">Command Input</div>
                  <div class="panel-title">Focused pane command lane</div>
                  <div class="panel-copy">Send shell input to the selected pane without leaving the workspace.</div>
                </div>
              `}

          <div class="dock-status" part="status">
            <span
              class="badge"
              data-testid="tp-command-active-pane"
              data-tone=${controls.activePaneId ? "ready" : "idle"}
              title=${activePaneIdentity?.title ?? ""}
            >
              ${activePaneIdentity?.label ?? "No pane"}
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
                      @click=${() => this.setDraft(command.value, { focusInput: true })}
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
                        ?disabled=${!controls.canWriteInput}
                        @click=${() => this.setDraft(command, { focusInput: true })}
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
              @click=${() => this.sendDraft({ focusInput: true })}
            >
              Send command
            </button>
            <button
              data-testid="tp-paste-clipboard"
              title=${pasteTitle}
              ?disabled=${!controls.canPasteClipboard}
              @click=${() => this.pasteClipboard({ focusInput: true })}
            >
              Paste
            </button>
            <button
              data-testid="tp-send-interrupt"
              ?disabled=${!controls.canWriteInput}
              @click=${() => this.sendShortcut("\u0003", { focusInput: true })}
            >
              Ctrl+C
            </button>
            <button
              data-testid="tp-send-enter"
              ?disabled=${!controls.canWriteInput}
              @click=${() => this.sendShortcut("\r", { focusInput: true })}
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
              title=${saveLayoutTitle}
              ?disabled=${!controls.canSaveLayout}
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
              data-danger="true"
              data-confirming=${String(isHistoryClearConfirming)}
              data-history-count=${String(controls.commandHistory.length)}
              title=${clearHistoryTitle}
              aria-label=${clearHistoryTitle}
              ?disabled=${controls.commandHistory.length === 0 || this.pending}
              @click=${() => this.handleClearCommandHistoryClick()}
            >
              ${isHistoryClearConfirming ? `Confirm clear ${controls.commandHistory.length}` : "Clear history"}
            </button>
          </div>
        </details>
      </div>
    `;
  }

  private setDraft(value: string, options: TerminalCommandInputFocusOptions = {}): void {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activePaneId || !controls.canWriteInput) {
      return;
    }

    this.actionError = null;
    this.clearHistoryClearConfirmation();
    this.resetHistoryNavigation();
    this.kernel?.commands.updateDraft(controls.activePaneId, value);
    if (options.focusInput) {
      this.refocusCommandInputAfterUpdate();
    }
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

  private async sendDraft(options: TerminalCommandInputFocusOptions = {}): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canSend) {
      return;
    }

    await this.dispatchInput(controls.activeSessionId, controls.activePaneId, `${controls.draft}\n`);
    this.recordCommandHistory(controls.draft);
    this.clearHistoryClearConfirmation();
    this.resetHistoryNavigation();
    this.kernel?.commands.clearDraft(controls.activePaneId);
    if (options.focusInput) {
      this.refocusCommandInputAfterUpdate();
    }
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
    this.clearHistoryClearConfirmation();
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

  private maybeAutoFocusInput(): void {
    if (!this.autoFocusInput) {
      this.#lastAutoFocusedPaneId = null;
      return;
    }

    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activePaneId || !controls.canWriteInput) {
      this.#lastAutoFocusedPaneId = null;
      return;
    }

    if (this.#lastAutoFocusedPaneId === controls.activePaneId) {
      return;
    }

    const textarea = this.shadowRoot?.querySelector<HTMLTextAreaElement>('[data-testid="tp-command-input"]');
    if (this.focusCommandInput(textarea)) {
      this.#lastAutoFocusedPaneId = controls.activePaneId;
    }
  }

  private focusCommandInput(
    textarea = this.shadowRoot?.querySelector<HTMLTextAreaElement>('[data-testid="tp-command-input"]'),
  ): boolean {
    if (!textarea || textarea.disabled) {
      return false;
    }

    textarea.focus({ preventScroll: true });
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    return true;
  }

  private refocusCommandInputAfterUpdate(): void {
    void this.updateComplete.then(() => {
      this.focusCommandInput();
    });
  }

  private async pasteClipboard(options: TerminalCommandInputFocusOptions = {}): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canPasteClipboard) {
      return;
    }

    this.pending = true;
    this.actionError = null;
    this.clearHistoryClearConfirmation();

    let pastedText = "";
    try {
      pastedText = await readClipboardText();
    } catch (error) {
      this.pending = false;
      this.actionError = getErrorMessage(error);
      if (options.focusInput) {
        this.refocusCommandInputAfterUpdate();
      }
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
      if (options.focusInput) {
        this.refocusCommandInputAfterUpdate();
      }
      return;
    }

    await this.dispatchPaste(controls.activeSessionId, controls.activePaneId, pastedText);
    if (options.focusInput) {
      this.refocusCommandInputAfterUpdate();
    }
  }

  private async sendShortcut(data: string, options: TerminalCommandInputFocusOptions = {}): Promise<void> {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.activePaneId || !controls.canWriteInput) {
      return;
    }

    this.clearHistoryClearConfirmation();
    await this.dispatchInput(controls.activeSessionId, controls.activePaneId, data);
    if (options.focusInput) {
      this.refocusCommandInputAfterUpdate();
    }
  }

  private async dispatchPaste(sessionId: string, paneId: string, data: string): Promise<void> {
    this.pending = true;
    this.actionError = null;
    this.clearHistoryClearConfirmation();

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
    this.clearHistoryClearConfirmation();

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
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (!controls.activeSessionId || !controls.canSaveLayout) {
      return;
    }

    this.pending = true;
    this.actionError = null;
    this.clearHistoryClearConfirmation();

    try {
      await this.kernel?.commands.dispatchMuxCommand(controls.activeSessionId, { kind: "save_session" });
      await this.kernel?.commands.refreshSavedSessions();
      const savedSessions = this.kernel?.getSnapshot().catalog.savedSessions ?? [];
      this.dispatchEvent(
        new CustomEvent("tp-terminal-layout-saved", {
          bubbles: true,
          composed: true,
          detail: {
            sessionId: controls.activeSessionId,
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
          detail: { sessionId: controls.activeSessionId, error },
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
    this.clearHistoryClearConfirmation();

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

  private handleClearCommandHistoryClick(): void {
    const controls = resolveTerminalCommandDockControlState(this.snapshot, { pending: this.pending });
    if (controls.commandHistory.length === 0 || this.pending) {
      this.clearHistoryClearConfirmation();
      return;
    }

    if (!this.historyClearConfirmationArmed) {
      this.setHistoryClearConfirmation();
      return;
    }

    this.clearCommandHistory();
  }

  private clearCommandHistory(): void {
    this.clearHistoryClearConfirmation();
    this.resetHistoryNavigation();
    this.kernel?.commands.clearCommandHistory();
    this.dispatchEvent(
      new CustomEvent("tp-terminal-command-history-cleared", {
        bubbles: true,
        composed: true,
      }),
    );
  }

  private setHistoryClearConfirmation(): void {
    this.actionError = null;
    this.historyClearConfirmationArmed = true;
    this.clearHistoryClearConfirmationResetTimer();
    this.#historyClearConfirmationResetTimer = setTimeout(() => {
      this.historyClearConfirmationArmed = false;
      this.#historyClearConfirmationResetTimer = null;
    }, TERMINAL_DESTRUCTIVE_CONFIRMATION_RESET_MS);
  }

  private clearHistoryClearConfirmation(): void {
    this.historyClearConfirmationArmed = false;
    this.clearHistoryClearConfirmationResetTimer();
  }

  private clearHistoryClearConfirmationResetTimer(): void {
    if (this.#historyClearConfirmationResetTimer) {
      clearTimeout(this.#historyClearConfirmationResetTimer);
      this.#historyClearConfirmationResetTimer = null;
    }
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace command failed";
}

function formatCommandHistoryCount(count: number): string {
  return `${count} command history ${count === 1 ? "entry" : "entries"}`;
}
