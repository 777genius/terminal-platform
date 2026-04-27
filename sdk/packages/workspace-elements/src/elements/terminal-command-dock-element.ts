import { css, html, nothing, type PropertyValues } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { readClipboardText } from "./terminal-clipboard.js";
import type { TerminalCommandComposerElement } from "./terminal-command-composer-element.js";
import type {
  TerminalCommandComposerDraftChangeDetail,
  TerminalCommandComposerHistoryNavigateDetail,
  TerminalCommandComposerShortcutDetail,
} from "./terminal-command-composer-events.js";
import { resolveTerminalCommandInputStatus } from "./terminal-command-input-status.js";
import { resolveTerminalCommandDockStatusBadges } from "./terminal-command-dock-status.js";
import {
  createTerminalCommandHistoryNavigationState,
  resolveTerminalCommandHistoryNavigation,
  type TerminalCommandHistoryInputState,
  type TerminalCommandHistoryNavigationDirection,
  type TerminalCommandHistoryNavigationState,
} from "./terminal-command-history-navigation.js";
import { TERMINAL_DESTRUCTIVE_CONFIRMATION_RESET_MS } from "./terminal-destructive-action.js";
import {
  defaultTerminalCommandQuickCommands,
  resolveTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";
import { resolveTerminalCommandDockAccessoryMode } from "./terminal-command-dock-accessories.js";
import {
  TERMINAL_COMMAND_DOCK_TERMINAL_RECENT_COMMAND_LIMIT,
  resolveTerminalCommandDockControlState,
} from "./terminal-command-dock-controls.js";
import {
  TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS,
  resolveTerminalCommandDockSessionActions,
  type TerminalCommandDockSessionActionId,
} from "./terminal-command-dock-session-actions.js";

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

      .dock[data-accessory-mode="bar"] {
        gap: 0.42rem;
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
        align-items: center;
        justify-content: flex-start;
        min-height: 1.35rem;
      }

      .dock-footer {
        display: grid;
        grid-template-columns: 1fr;
        align-items: flex-start;
        gap: var(--tp-space-3);
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

      .dock-accessory-bar {
        display: grid;
        grid-template-columns: auto minmax(0, 1.2fr) minmax(0, 1fr) auto;
        gap: 0.42rem;
        align-items: center;
        min-width: 0;
        border-top: 1px solid color-mix(in srgb, var(--tp-terminal-color-border) 58%, transparent);
        padding-top: 0.45rem;
      }

      .dock-accessory-bar .dock-header,
      .dock-accessory-bar .history-row,
      .dock-accessory-bar .chip-row,
      .dock-accessory-bar .session-actions {
        min-width: 0;
      }

      .dock-accessory-bar .dock-status,
      .dock-accessory-bar .chip-row,
      .dock-accessory-bar .history-actions,
      .dock-accessory-bar .session-actions {
        flex-wrap: nowrap;
        overflow-x: auto;
        scrollbar-width: none;
      }

      .dock-accessory-bar .dock-status::-webkit-scrollbar,
      .dock-accessory-bar .chip-row::-webkit-scrollbar,
      .dock-accessory-bar .history-actions::-webkit-scrollbar,
      .dock-accessory-bar .session-actions::-webkit-scrollbar {
        display: none;
      }

      .dock-accessory-bar .badge {
        white-space: nowrap;
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

      .chip {
        flex: 0 0 auto;
        max-width: min(14rem, 100%);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
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

      .history-label {
        color: var(--tp-color-text-muted);
        font-size: 0.74rem;
        font-weight: 700;
        letter-spacing: 0;
        text-transform: uppercase;
      }

      .history-chip {
        flex: 0 0 auto;
        max-width: min(22rem, 100%);
        min-width: 0;
        overflow: hidden;
        justify-content: flex-start;
        white-space: nowrap;
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

      .dock[data-accessory-mode="bar"] .history-row {
        gap: 0.42rem;
        grid-template-columns: auto minmax(0, 1fr);
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
        grid-template-columns: auto minmax(0, 1fr) auto;
        gap: var(--tp-space-2);
        align-items: stretch;
        border: 1px solid color-mix(in srgb, var(--tp-color-border) 82%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-bg) 74%, transparent);
        padding: var(--tp-space-2);
      }

      .composer-actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-content: flex-start;
        justify-content: flex-end;
        min-width: 0;
      }

      .composer-actions button {
        min-width: 3rem;
        white-space: nowrap;
      }

      .dock[data-placement="terminal"] .composer {
        border-color: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 36%,
          var(--tp-terminal-color-border)
        );
        border-top-width: 0;
        border-radius: 0 0 var(--tp-radius-md) var(--tp-radius-md);
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 86%, transparent),
            transparent
          ),
          color-mix(in srgb, var(--tp-terminal-color-bg) 92%, var(--tp-terminal-color-bg-raised));
        padding: 0.48rem 0.62rem;
      }

      .dock[data-placement="terminal"] .composer-actions {
        gap: 0.35rem;
        flex-wrap: nowrap;
      }

      .dock[data-placement="terminal"] .composer-actions button {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, transparent);
        color: var(--tp-terminal-color-text);
        font-size: 0.8rem;
        min-width: 2.15rem;
        padding: 0.3rem 0.48rem;
      }

      .dock[data-placement="terminal"] .composer-actions .primary {
        border-color: color-mix(in srgb, var(--tp-terminal-color-accent) 54%, transparent);
        background: color-mix(
          in srgb,
          var(--tp-terminal-color-accent) 16%,
          var(--tp-terminal-color-bg-raised)
        );
        min-width: 3rem;
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
        max-height: min(12rem, 34vh);
        overflow-y: auto;
        resize: vertical;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--tp-color-text);
        font: 0.94rem/1.45 var(--tp-font-family-mono);
      }

      .dock[data-placement="terminal"] textarea {
        color: var(--tp-terminal-color-text);
        min-height: 1.65rem;
        max-height: min(9rem, 28vh);
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

      .dock[data-placement="terminal"] details .actions button {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, transparent);
        color: var(--tp-terminal-color-text);
      }

      .dock[data-placement="terminal"] .session-actions {
        justify-content: flex-end;
      }

      .dock[data-placement="terminal"] .session-actions button {
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 72%, transparent);
        border-radius: 0.45rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, transparent);
        color: var(--tp-terminal-color-text);
        padding: 0.3rem 0.48rem;
        white-space: nowrap;
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

      details {
        border-top: 1px solid var(--tp-color-border);
        padding-top: var(--tp-space-3);
      }

      .dock[data-placement="terminal"] details {
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

        .dock-status {
          justify-content: flex-start;
        }
      }

      @media (max-width: 1180px) {
        .dock[data-placement="terminal"] .dock-status,
        .dock[data-placement="terminal"] .chip-row {
          flex-wrap: nowrap;
          overflow-x: auto;
          scrollbar-width: none;
        }

        .dock[data-placement="terminal"] .history-row,
        .dock[data-placement="terminal"] details {
          display: none;
        }

        .dock-accessory-bar {
          grid-template-columns: 1fr;
        }

        .dock-accessory-bar .session-actions {
          display: none;
        }
      }

      @media (max-width: 720px) {
        .dock[data-placement="terminal"] {
          padding: 0 var(--tp-space-2) var(--tp-space-2);
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

  #historyNavigation: TerminalCommandHistoryNavigationState = createTerminalCommandHistoryNavigationState();
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
    const isTerminalPlacement = this.placement === "terminal";
    const controls = resolveTerminalCommandDockControlState(this.snapshot, {
      pending: this.pending,
      recentCommandLimit: isTerminalPlacement ? TERMINAL_COMMAND_DOCK_TERMINAL_RECENT_COMMAND_LIMIT : null,
    });
    const quickCommands = resolveTerminalCommandQuickCommands(this.quickCommands);
    const inputStatus = resolveTerminalCommandInputStatus(controls);
    const statusBadges = resolveTerminalCommandDockStatusBadges(controls, inputStatus, {
      placement: this.placement,
    });
    const pasteTitle = controls.pasteCapabilityStatus === "known" && !controls.canPasteClipboard
      ? "Paste is not supported by the active backend"
      : "Paste clipboard into the focused pane";
    const accessoryMode = resolveTerminalCommandDockAccessoryMode({ placement: this.placement });
    const sessionActions = resolveTerminalCommandDockSessionActions(controls, {
      historyClearConfirmationArmed: this.historyClearConfirmationArmed,
      pending: this.pending,
      placement: this.placement,
    });

    const headerTemplate = html`
      <div class="dock-header">
        ${isTerminalPlacement
          ? nothing
          : html`
              <div class="panel-header">
                <div class="panel-eyebrow">Command Input</div>
                <div class="panel-title">Focused pane command lane</div>
                <div class="panel-copy">Send shell input to the selected pane without leaving the workspace.</div>
              </div>
            `}

        <div class="dock-status" part="status">
          ${statusBadges.map(
            (badge) => html`
              <span
                class="badge"
                data-status-badge=${badge.id}
                data-testid=${badge.testId}
                data-tone=${badge.tone}
                title=${badge.title}
              >
                ${badge.label}
              </span>
            `,
          )}
        </div>
      </div>
    `;

    const quickCommandsTemplate = quickCommands.length > 0
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
      : nothing;

    const commandHistoryTemplate = controls.recentCommands.length > 0
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
      : nothing;

    const composerTemplate = html`
      <tp-terminal-command-composer
        class="composer"
        part="composer"
        .draft=${controls.draft}
        .canWriteInput=${controls.canWriteInput}
        .canSend=${controls.canSend}
        .canPasteClipboard=${controls.canPasteClipboard}
        .placeholder=${inputStatus.placeholder}
        .pasteTitle=${pasteTitle}
        .placement=${this.placement}
        @tp-terminal-command-draft-change=${(event: CustomEvent<TerminalCommandComposerDraftChangeDetail>) =>
          this.handleComposerDraftChange(event)}
        @tp-terminal-command-history-navigate=${(event: CustomEvent<TerminalCommandComposerHistoryNavigateDetail>) =>
          this.handleComposerHistoryNavigate(event)}
        @tp-terminal-command-submit=${() => this.sendDraft({ focusInput: true })}
        @tp-terminal-command-paste=${() => this.pasteClipboard({ focusInput: true })}
        @tp-terminal-command-shortcut=${(event: CustomEvent<TerminalCommandComposerShortcutDetail>) =>
          this.handleComposerShortcut(event)}
      ></tp-terminal-command-composer>
    `;

    const errorTemplate = this.actionError
      ? html`
          <div class="notice" part="error">
            <strong>Command failed</strong>
            <div>${this.actionError}</div>
          </div>
        `
      : nothing;

    const footerTemplate = isTerminalPlacement
      ? nothing
      : html`
          <div class="dock-footer">
            <div class="hint" part="hint">
              ${inputStatus.hint}
            </div>
          </div>
        `;

    const sessionActionsTemplate = html`
      <div class="actions session-actions" part="session-actions" data-testid="tp-session-actions">
            ${sessionActions.map(
              (action) => html`
                <button
                  data-testid=${action.testId}
                  data-session-action=${action.id}
                  data-danger=${action.dangerous ? "true" : nothing}
                  data-confirming=${String(action.confirming)}
                  data-history-count=${action.historyCount == null ? nothing : String(action.historyCount)}
                  title=${action.title}
                  aria-label=${action.ariaLabel}
                  ?disabled=${action.disabled}
                  @click=${() => this.handleSessionActionClick(action.id)}
                >
                  ${action.label}
                </button>
              `,
            )}
          </div>
    `;

    const sessionToolsTemplate = html`
      <details part="session-tools" data-testid="tp-session-tools">
          <summary>Session tools</summary>
          ${sessionActionsTemplate}
        </details>
    `;

    const accessoryBarTemplate = html`
      <div
        class="dock-accessory-bar"
        part="terminal-accessories"
        data-testid="tp-command-accessory-bar"
        data-accessory-mode=${accessoryMode}
        aria-label="Terminal command accessories"
      >
        ${headerTemplate}
        ${quickCommandsTemplate}
        ${commandHistoryTemplate}
        ${sessionActionsTemplate}
      </div>
    `;

    const orderedDockContent = isTerminalPlacement
      ? [
          composerTemplate,
          errorTemplate,
          accessoryBarTemplate,
        ]
      : [
          headerTemplate,
          quickCommandsTemplate,
          commandHistoryTemplate,
          composerTemplate,
          errorTemplate,
          footerTemplate,
          sessionToolsTemplate,
        ];

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
        data-accessory-mode=${accessoryMode}
      >
        ${orderedDockContent}
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

  private handleComposerDraftChange(event: CustomEvent<TerminalCommandComposerDraftChangeDetail>): void {
    this.setDraft(event.detail.value);
  }

  private handleComposerHistoryNavigate(event: CustomEvent<TerminalCommandComposerHistoryNavigateDetail>): void {
    const composer = event.currentTarget as TerminalCommandComposerElement;
    if (this.navigateCommandHistory(event.detail.direction, event.detail.input, composer)) {
      event.preventDefault();
    }
  }

  private handleComposerShortcut(event: CustomEvent<TerminalCommandComposerShortcutDetail>): void {
    void this.sendShortcut(event.detail.data, { focusInput: true });
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

  private navigateCommandHistory(
    direction: TerminalCommandHistoryNavigationDirection,
    input: TerminalCommandHistoryInputState,
    composer: TerminalCommandComposerElement,
  ): boolean {
    const paneId = this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;
    const commandHistory = this.snapshot.commandHistory.entries;
    if (!paneId) {
      return false;
    }

    const result = resolveTerminalCommandHistoryNavigation(
      direction,
      input,
      commandHistory,
      this.#historyNavigation,
    );
    this.#historyNavigation = result.state;

    if (!result.navigated) {
      return false;
    }

    this.applyHistoryDraft(paneId, composer, result.value);
    return true;
  }

  private applyHistoryDraft(paneId: string, composer: TerminalCommandComposerElement, value: string): void {
    this.clearHistoryClearConfirmation();
    composer.applyDraft(value);
    this.kernel?.commands.updateDraft(paneId, value);
  }

  private recordCommandHistory(value: string): void {
    this.kernel?.commands.recordCommandHistory(value);
  }

  private resetHistoryNavigation(): void {
    this.#historyNavigation = createTerminalCommandHistoryNavigationState();
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

    if (this.focusCommandInput()) {
      this.#lastAutoFocusedPaneId = controls.activePaneId;
    }
  }

  private focusCommandInput(
    composer = this.shadowRoot?.querySelector<TerminalCommandComposerElement>("tp-terminal-command-composer"),
  ): boolean {
    return composer?.focusInput() ?? false;
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

  private handleSessionActionClick(actionId: TerminalCommandDockSessionActionId): void {
    switch (actionId) {
      case TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.saveLayout:
        void this.saveLayout();
        return;
      case TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.refreshTerminal:
        void this.refreshActiveSession();
        return;
      case TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.clearCommandHistory:
        this.handleClearCommandHistoryClick();
        return;
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
