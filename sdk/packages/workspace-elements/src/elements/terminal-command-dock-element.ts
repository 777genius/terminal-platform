import { css, html, nothing } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

const QUICK_COMMANDS: readonly { label: string; value: string }[] = [
  { label: "pwd", value: "pwd" },
  { label: "ls -la", value: "ls -la" },
  { label: "git status", value: "git status" },
  { label: "hello", value: 'printf "hello from Terminal Platform\\n"' },
];

export class TerminalCommandDockElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
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

      .dock-header,
      .dock-footer {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: var(--tp-space-3);
      }

      .dock-status {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        justify-content: flex-end;
      }

      .chip-row,
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
        .dock-header,
        .dock-footer {
          display: grid;
        }

        .dock-status {
          justify-content: flex-start;
        }
      }
    `,
  ];

  protected declare pending: boolean;
  protected declare actionError: string | null;

  constructor() {
    super();
    this.pending = false;
    this.actionError = null;
  }

  override render() {
    const activeSessionId =
      this.snapshot.selection.activeSessionId ?? this.snapshot.attachedSession?.session.session_id ?? null;
    const activePaneId =
      this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;
    const draft = activePaneId ? (this.snapshot.drafts[activePaneId] ?? "") : "";
    const canSend = Boolean(activeSessionId && activePaneId && draft.trim().length > 0 && !this.pending);
    const canUsePane = Boolean(activeSessionId && activePaneId && !this.pending);
    const statusLabel = this.pending ? "Sending" : activePaneId ? "Ready" : "Pick a pane";

    return html`
      <div class="panel dock" part="command-dock" data-testid="tp-command-dock">
        <div class="dock-header">
          <div class="panel-header">
            <div class="panel-eyebrow">Command Input</div>
            <div class="panel-title">Focused pane command lane</div>
            <div class="panel-copy">Send shell input to the selected pane without leaving the workspace.</div>
          </div>

          <div class="dock-status" part="status">
            <span class="badge" data-tone=${activePaneId ? "ready" : "idle"}>
              ${activePaneId ? `Pane ${activePaneId}` : "No pane"}
            </span>
            <span class="badge" data-tone=${canSend ? "ready" : "idle"}>${statusLabel}</span>
          </div>
        </div>

        <div class="chip-row" part="quick-commands" aria-label="Quick commands">
          ${QUICK_COMMANDS.map(
            (command) => html`
              <button
                class="chip"
                type="button"
                ?disabled=${!activePaneId || this.pending}
                @click=${() => this.setDraft(command.value)}
              >
                ${command.label}
              </button>
            `,
          )}
        </div>

        <label class="composer" part="composer">
          <span class="prompt" aria-hidden="true">&gt;_</span>
          <textarea
            data-testid="tp-command-input"
            .value=${draft}
            ?disabled=${!activePaneId || this.pending}
            placeholder=${activePaneId ? "Type shell input for the focused pane" : "Select a pane first"}
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
            ${activePaneId
              ? "Enter sends the command. Shift+Enter inserts a newline."
              : "Start or select a session, then choose a pane to enable input."}
          </div>

          <div class="actions">
            <button
              class="primary"
              data-testid="tp-send-command"
              ?disabled=${!canSend}
              @click=${() => this.sendDraft()}
            >
              Send command
            </button>
            <button ?disabled=${!canUsePane} @click=${() => this.sendShortcut("\u0003")}>Ctrl+C</button>
            <button ?disabled=${!canUsePane} @click=${() => this.sendShortcut("\r")}>Enter</button>
          </div>
        </div>

        <details part="session-tools">
          <summary>Session tools</summary>
          <div class="actions">
            <button ?disabled=${!activeSessionId || this.pending} @click=${() => this.saveLayout()}>
              Save layout
            </button>
            <button ?disabled=${!activeSessionId || this.pending} @click=${() => this.refreshActiveSession()}>
              Refresh terminal
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
    this.kernel?.commands.updateDraft(paneId, value);
  }

  private handleInput(event: Event): void {
    const target = event.currentTarget as HTMLTextAreaElement;
    this.setDraft(target.value);
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }

    event.preventDefault();
    void this.sendDraft();
  }

  private async sendDraft(): Promise<void> {
    const sessionId = this.snapshot.selection.activeSessionId ?? this.snapshot.attachedSession?.session.session_id ?? null;
    const paneId = this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;
    const draft = paneId ? (this.snapshot.drafts[paneId] ?? "") : "";

    if (!sessionId || !paneId || draft.trim().length === 0 || this.pending) {
      return;
    }

    await this.dispatchInput(sessionId, paneId, `${draft}\n`);
    this.kernel?.commands.clearDraft(paneId);
  }

  private async sendShortcut(data: string): Promise<void> {
    const sessionId = this.snapshot.selection.activeSessionId ?? this.snapshot.attachedSession?.session.session_id ?? null;
    const paneId = this.snapshot.selection.activePaneId ?? this.snapshot.attachedSession?.focused_screen?.pane_id ?? null;

    if (!sessionId || !paneId || this.pending) {
      return;
    }

    await this.dispatchInput(sessionId, paneId, data);
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
    } catch (error) {
      this.actionError = getErrorMessage(error);
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
    } catch (error) {
      this.actionError = getErrorMessage(error);
    } finally {
      this.pending = false;
    }
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace command failed";
}
