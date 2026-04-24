import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

const DEFAULT_VISIBLE_SAVED_SESSIONS = 4;
const SAVED_SESSION_PAGE_SIZE = 8;

export class TerminalSavedSessionsElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    visibleSavedSessionCount: { state: true },
    pendingSavedSessionId: { state: true },
    pendingSavedSessionAction: { state: true },
    deleteConfirmationSessionId: { state: true },
    actionError: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .saved {
        padding: var(--tp-space-3);
      }

      ul {
        list-style: none;
        margin: 0;
        padding: 0;
        max-height: 20rem;
        overflow: auto;
      }

      li + li {
        margin-top: var(--tp-space-2);
      }

      li {
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-md);
        padding: var(--tp-space-3);
        background: color-mix(in srgb, var(--tp-color-panel-raised) 58%, transparent);
      }

      .actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        margin-top: var(--tp-space-2);
      }

      button[data-danger="true"] {
        border-color: color-mix(in srgb, var(--tp-color-danger) 42%, transparent);
        color: var(--tp-color-danger);
      }

      button[data-confirming="true"] {
        background: color-mix(in srgb, var(--tp-color-danger) 16%, var(--tp-color-panel-raised));
      }

      .list-footer {
        display: grid;
        gap: var(--tp-space-2);
        margin-top: var(--tp-space-3);
      }

      .list-controls {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
      }

      .summary {
        display: grid;
        gap: 0.2rem;
      }

      .meta {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        color: var(--tp-color-text-muted);
        font-size: 0.8rem;
      }

      .notice {
        border: 1px solid color-mix(in srgb, var(--tp-color-danger) 45%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-danger) 10%, transparent);
        color: var(--tp-color-text);
        margin-bottom: var(--tp-space-3);
        padding: var(--tp-space-3);
      }
    `,
  ];

  protected declare visibleSavedSessionCount: number;
  protected declare pendingSavedSessionId: string | null;
  protected declare pendingSavedSessionAction: "restore" | "delete" | null;
  protected declare deleteConfirmationSessionId: string | null;
  protected declare actionError: string | null;

  #deleteConfirmationResetTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    super();
    this.visibleSavedSessionCount = DEFAULT_VISIBLE_SAVED_SESSIONS;
    this.pendingSavedSessionId = null;
    this.pendingSavedSessionAction = null;
    this.deleteConfirmationSessionId = null;
    this.actionError = null;
  }

  override disconnectedCallback(): void {
    this.clearDeleteConfirmationResetTimer();
    super.disconnectedCallback();
  }

  override render() {
    const savedSessions = this.snapshot.catalog.savedSessions;
    const visibleCount = Math.min(this.visibleSavedSessionCount, savedSessions.length);
    const visibleSessions = savedSessions.slice(0, visibleCount);
    const hiddenCount = savedSessions.length - visibleSessions.length;

    return html`
      <div class="panel saved" part="saved" data-testid="tp-saved-sessions">
        <div class="panel-header">
          <div class="panel-eyebrow">Saved layouts</div>
          <div class="panel-title">${savedSessions.length || "No"} saved sessions</div>
          <div class="panel-copy">
            Restore a saved layout or clean up entries you no longer need. Large histories are paged to keep
            the workspace responsive.
          </div>
        </div>

        ${this.actionError
          ? html`
              <div class="notice" part="error" data-testid="tp-saved-session-error">
                <strong>Saved layout action failed</strong>
                <div>${this.actionError}</div>
              </div>
            `
          : null}

        ${savedSessions.length === 0
          ? html`<div class="empty-state" part="empty">Saved sessions will appear here after you save a layout.</div>`
          : html`
              <ul part="list" data-testid="tp-saved-session-list">
                ${visibleSessions.map(
                  (session) => {
                    const title = session.title ?? session.session_id;
                    const isPending = this.pendingSavedSessionId === session.session_id;
                    const isRestoring = isPending && this.pendingSavedSessionAction === "restore";
                    const isDeleting = isPending && this.pendingSavedSessionAction === "delete";
                    const isConfirmingDelete = this.deleteConfirmationSessionId === session.session_id;
                    const anyPending = Boolean(this.pendingSavedSessionId);

                    return html`
                      <li part="item" data-testid="tp-saved-session-item" data-session-id=${session.session_id}>
                        <div class="summary">
                          <strong>${title}</strong>
                          <div class="meta">
                            <span>${session.compatibility.status}</span>
                            <span>${session.tab_count} tabs</span>
                            <span>${session.pane_count} panes</span>
                            ${isConfirmingDelete ? html`<span>confirm delete</span>` : null}
                          </div>
                        </div>
                        <div class="actions" part="actions">
                          <button
                            data-testid="tp-restore-saved-session"
                            ?disabled=${anyPending || !session.compatibility.can_restore}
                            aria-label=${`Restore saved layout ${title}`}
                            @click=${() => {
                              void this.restoreSavedSession(session.session_id);
                            }}
                          >
                            ${isRestoring ? "Restoring" : "Restore"}
                          </button>
                          <button
                            data-testid="tp-delete-saved-session"
                            data-danger="true"
                            data-confirming=${String(isConfirmingDelete)}
                            ?disabled=${anyPending}
                            aria-label=${isConfirmingDelete
                              ? `Confirm delete saved layout ${title}`
                              : `Delete saved layout ${title}`}
                            @click=${() => {
                              void this.handleDeleteClick(session.session_id);
                            }}
                          >
                            ${isDeleting ? "Deleting" : isConfirmingDelete ? "Confirm delete" : "Delete"}
                          </button>
                        </div>
                      </li>
                    `;
                  },
                )}
              </ul>

              <div class="list-footer" part="list-footer">
                <div class="meta" part="list-summary">
                  <span>Showing ${visibleSessions.length} of ${savedSessions.length}</span>
                  ${hiddenCount > 0 ? html`<span>${hiddenCount} hidden</span>` : null}
                </div>

                <div class="list-controls">
                  ${hiddenCount > 0
                    ? html`
                        <button part="show-more" @click=${() => this.showMoreSavedSessions()}>
                          Show ${Math.min(SAVED_SESSION_PAGE_SIZE, hiddenCount)} more
                        </button>
                      `
                    : null}
                  ${visibleSessions.length > DEFAULT_VISIBLE_SAVED_SESSIONS
                    ? html`
                        <button part="collapse" @click=${() => this.collapseSavedSessions()}>
                          Collapse
                        </button>
                      `
                    : null}
                </div>
              </div>
            `}
      </div>
    `;
  }

  private showMoreSavedSessions(): void {
    this.visibleSavedSessionCount = Math.min(
      this.snapshot.catalog.savedSessions.length,
      this.visibleSavedSessionCount + SAVED_SESSION_PAGE_SIZE,
    );
  }

  private collapseSavedSessions(): void {
    this.visibleSavedSessionCount = DEFAULT_VISIBLE_SAVED_SESSIONS;
  }

  private async restoreSavedSession(sessionId: string): Promise<void> {
    if (this.pendingSavedSessionId) {
      return;
    }

    this.clearDeleteConfirmation();
    this.setPendingAction(sessionId, "restore");

    try {
      await this.kernel?.commands.restoreSavedSession(sessionId);
      this.dispatchEvent(
        new CustomEvent("tp-saved-session-restored", {
          bubbles: true,
          composed: true,
          detail: { sessionId },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-saved-session-restore-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, error },
        }),
      );
    } finally {
      this.clearPendingAction();
    }
  }

  private async handleDeleteClick(sessionId: string): Promise<void> {
    if (this.pendingSavedSessionId) {
      return;
    }

    if (this.deleteConfirmationSessionId !== sessionId) {
      this.setDeleteConfirmation(sessionId);
      return;
    }

    await this.deleteSavedSession(sessionId);
  }

  private async deleteSavedSession(sessionId: string): Promise<void> {
    this.setPendingAction(sessionId, "delete");

    try {
      await this.kernel?.commands.deleteSavedSession(sessionId);
      this.dispatchEvent(
        new CustomEvent("tp-saved-session-deleted", {
          bubbles: true,
          composed: true,
          detail: { sessionId },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-saved-session-delete-failed", {
          bubbles: true,
          composed: true,
          detail: { sessionId, error },
        }),
      );
    } finally {
      this.clearDeleteConfirmation();
      this.clearPendingAction();
    }
  }

  private setPendingAction(sessionId: string, action: "restore" | "delete"): void {
    this.actionError = null;
    this.pendingSavedSessionId = sessionId;
    this.pendingSavedSessionAction = action;
  }

  private clearPendingAction(): void {
    this.pendingSavedSessionId = null;
    this.pendingSavedSessionAction = null;
  }

  private setDeleteConfirmation(sessionId: string): void {
    this.actionError = null;
    this.deleteConfirmationSessionId = sessionId;
    this.clearDeleteConfirmationResetTimer();
    this.#deleteConfirmationResetTimer = setTimeout(() => {
      if (this.deleteConfirmationSessionId === sessionId) {
        this.deleteConfirmationSessionId = null;
      }
      this.#deleteConfirmationResetTimer = null;
    }, 4000);
  }

  private clearDeleteConfirmation(): void {
    this.deleteConfirmationSessionId = null;
    this.clearDeleteConfirmationResetTimer();
  }

  private clearDeleteConfirmationResetTimer(): void {
    if (this.#deleteConfirmationResetTimer) {
      clearTimeout(this.#deleteConfirmationResetTimer);
      this.#deleteConfirmationResetTimer = null;
    }
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Saved layout action failed";
}
