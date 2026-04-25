import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalSavedSessionsControlState,
  TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT,
} from "./terminal-saved-sessions-controls.js";

const DEFAULT_VISIBLE_SAVED_SESSIONS = TERMINAL_SAVED_SESSIONS_DEFAULT_VISIBLE_COUNT;
const SAVED_SESSION_PAGE_SIZE = 8;
const SAVED_SESSION_CONFIRMATION_RESET_MS = 4000;

export class TerminalSavedSessionsElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    visibleSavedSessionCount: { state: true },
    pendingSavedSessionId: { state: true },
    pendingSavedSessionAction: { state: true },
    pendingBulkAction: { state: true },
    deleteConfirmationSessionId: { state: true },
    pruneConfirmationArmed: { state: true },
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

      .list-controls button[data-danger="true"] {
        margin-left: auto;
      }

      .list-controls button[data-danger="true"][data-confirming="true"] {
        background: color-mix(in srgb, var(--tp-color-danger) 16%, var(--tp-color-panel-raised));
      }

      .summary {
        display: grid;
        gap: 0.2rem;
      }

      .semantics {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        margin-top: var(--tp-space-2);
      }

      .semantic-note {
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-sm);
        color: var(--tp-color-text-muted);
        font-size: 0.72rem;
        line-height: 1;
        padding: 0.25rem 0.4rem;
      }

      .semantic-note[data-tone="ok"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 34%, transparent);
        color: color-mix(in srgb, var(--tp-color-success) 76%, var(--tp-color-text));
      }

      .semantic-note[data-tone="warning"] {
        border-color: color-mix(in srgb, var(--tp-color-warning) 42%, transparent);
        color: color-mix(in srgb, var(--tp-color-warning) 74%, var(--tp-color-text));
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
  protected declare pendingBulkAction: "prune" | null;
  protected declare deleteConfirmationSessionId: string | null;
  protected declare pruneConfirmationArmed: boolean;
  protected declare actionError: string | null;

  #deleteConfirmationResetTimer: ReturnType<typeof setTimeout> | null = null;
  #pruneConfirmationResetTimer: ReturnType<typeof setTimeout> | null = null;

  constructor() {
    super();
    this.visibleSavedSessionCount = DEFAULT_VISIBLE_SAVED_SESSIONS;
    this.pendingSavedSessionId = null;
    this.pendingSavedSessionAction = null;
    this.pendingBulkAction = null;
    this.deleteConfirmationSessionId = null;
    this.pruneConfirmationArmed = false;
    this.actionError = null;
  }

  override disconnectedCallback(): void {
    this.clearDeleteConfirmationResetTimer();
    this.clearPruneConfirmationResetTimer();
    super.disconnectedCallback();
  }

  override render() {
    const controls = this.resolveControls();

    return html`
      <div
        class="panel saved"
        part="saved"
        data-testid="tp-saved-sessions"
        data-saved-count=${String(controls.savedSessionCount)}
        data-visible-count=${String(controls.visibleCount)}
        data-hidden-count=${String(controls.hiddenCount)}
        data-pending=${String(controls.anyPending)}
      >
        <div class="panel-header">
          <div class="panel-eyebrow">Saved layouts</div>
          <div class="panel-title">${controls.savedSessionCount || "No"} saved sessions</div>
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

        ${controls.savedSessionCount === 0
          ? html`<div class="empty-state" part="empty">Saved sessions will appear here after you save a layout.</div>`
          : html`
              <ul part="list" data-testid="tp-saved-session-list">
                ${controls.items.map(
                  (item) => {
                    return html`
                      <li part="item" data-testid="tp-saved-session-item" data-session-id=${item.session.session_id}>
                        <div class="summary">
                          <strong>${item.title}</strong>
                          <div class="meta">
                            <span title=${item.compatibilityLabel}>${item.session.compatibility.status}</span>
                            <span>${item.session.tab_count} tabs</span>
                            <span>${item.session.pane_count} panes</span>
                            ${item.isConfirmingDelete ? html`<span>confirm delete</span>` : null}
                          </div>
                          <div class="semantics" part="restore-semantics">
                            ${item.restoreSemanticsNotes.map(
                              (note) => html`
                                <span
                                  class="semantic-note"
                                  data-testid="tp-saved-session-restore-semantics"
                                  data-semantics-code=${note.code}
                                  data-tone=${note.tone}
                                  title=${note.detail}
                                >
                                  ${note.label}
                                </span>
                              `,
                            )}
                          </div>
                        </div>
                        <div class="actions" part="actions">
                          <button
                            data-testid="tp-restore-saved-session"
                            data-can-restore=${String(item.canRestore)}
                            data-restore-status=${item.restoreStatus}
                            data-compatibility-status=${item.session.compatibility.status}
                            title=${item.restoreTitle}
                            ?disabled=${!item.canRestore}
                            aria-label=${`Restore saved layout ${item.title}`}
                            @click=${() => {
                              void this.restoreSavedSession(item.session.session_id);
                            }}
                          >
                            ${item.isRestoring ? "Restoring" : "Restore"}
                          </button>
                          <button
                            data-testid="tp-delete-saved-session"
                            data-danger="true"
                            data-confirming=${String(item.isConfirmingDelete)}
                            ?disabled=${!item.canDelete}
                            aria-label=${item.isConfirmingDelete
                              ? `Confirm delete saved layout ${item.title}`
                              : `Delete saved layout ${item.title}`}
                            @click=${() => {
                              void this.handleDeleteClick(item.session.session_id);
                            }}
                          >
                            ${item.isDeleting ? "Deleting" : item.isConfirmingDelete ? "Confirm delete" : "Delete"}
                          </button>
                        </div>
                      </li>
                    `;
                  },
                )}
              </ul>

              <div class="list-footer" part="list-footer">
                <div class="meta" part="list-summary">
                  <span>Showing ${controls.visibleCount} of ${controls.savedSessionCount}</span>
                  ${controls.hiddenCount > 0 ? html`<span>${controls.hiddenCount} hidden</span>` : null}
                </div>

                <div class="list-controls">
                  ${controls.canShowMore
                    ? html`
                        <button part="show-more" @click=${() => this.showMoreSavedSessions()}>
                          Show ${Math.min(SAVED_SESSION_PAGE_SIZE, controls.hiddenCount)} more
                        </button>
                      `
                    : null}
                  ${controls.canCollapse
                    ? html`
                        <button part="collapse" @click=${() => this.collapseSavedSessions()}>
                          Collapse
                        </button>
                      `
                    : null}
                  ${controls.hiddenCount > 0
                    ? html`
                        <button
                          part="prune-hidden"
                          data-testid="tp-prune-hidden-saved-sessions"
                          data-danger="true"
                          data-confirming=${String(controls.pruneConfirmationArmed)}
                          ?disabled=${!controls.canPruneHidden}
                          @click=${() => {
                            void this.handlePruneHiddenClick();
                          }}
                        >
                          ${controls.isPruning
                            ? "Pruning"
                            : controls.pruneConfirmationArmed
                              ? `Confirm prune ${controls.hiddenCount}`
                              : "Prune hidden"}
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
    this.clearPruneConfirmation();
    this.visibleSavedSessionCount = Math.min(
      this.snapshot.catalog.savedSessions.length,
      this.visibleSavedSessionCount + SAVED_SESSION_PAGE_SIZE,
    );
  }

  private collapseSavedSessions(): void {
    this.clearPruneConfirmation();
    this.visibleSavedSessionCount = DEFAULT_VISIBLE_SAVED_SESSIONS;
  }

  private async restoreSavedSession(sessionId: string): Promise<void> {
    const target = findRestorableSavedSession(this.snapshot, this.controlOptions(), sessionId);
    if (!target) {
      return;
    }

    this.clearDeleteConfirmation();
    this.clearPruneConfirmation();
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
    const controls = this.resolveControls();
    const target = controls.items.find((item) => item.session.session_id === sessionId);
    if (!target?.canDelete || !hasSavedSession(this.snapshot, sessionId)) {
      return;
    }

    this.clearPruneConfirmation();
    if (this.deleteConfirmationSessionId !== sessionId) {
      this.setDeleteConfirmation(sessionId);
      return;
    }

    await this.deleteSavedSession(sessionId);
  }

  private async handlePruneHiddenClick(): Promise<void> {
    const controls = this.resolveControls();
    if (!controls.canPruneHidden) {
      this.clearPruneConfirmation();
      return;
    }

    this.clearDeleteConfirmation();
    if (!this.pruneConfirmationArmed) {
      this.setPruneConfirmation();
      return;
    }

    await this.pruneHiddenSavedSessions(controls.pruneKeepLatest);
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

  private setPendingBulkAction(action: "prune" | null): void {
    this.actionError = null;
    this.pendingBulkAction = action;
  }

  private async pruneHiddenSavedSessions(keepLatest: number): Promise<void> {
    const beforeCount = this.snapshot.catalog.savedSessions.length;
    this.setPendingBulkAction("prune");

    try {
      const result = await this.kernel?.commands.pruneSavedSessions(keepLatest);
      const afterCount = this.kernel?.getSnapshot().catalog.savedSessions.length ?? Math.min(beforeCount, keepLatest);
      this.visibleSavedSessionCount = Math.min(this.visibleSavedSessionCount, afterCount);
      this.dispatchEvent(
        new CustomEvent("tp-saved-sessions-pruned", {
          bubbles: true,
          composed: true,
          detail: {
            keepLatest,
            deletedCount: result?.deleted_count ?? Math.max(0, beforeCount - afterCount),
            keptCount: result?.kept_count ?? afterCount,
          },
        }),
      );
    } catch (error) {
      this.actionError = getErrorMessage(error);
      this.dispatchEvent(
        new CustomEvent("tp-saved-sessions-prune-failed", {
          bubbles: true,
          composed: true,
          detail: { keepLatest, error },
        }),
      );
    } finally {
      this.clearPruneConfirmation();
      this.setPendingBulkAction(null);
    }
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
    }, SAVED_SESSION_CONFIRMATION_RESET_MS);
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

  private setPruneConfirmation(): void {
    this.actionError = null;
    this.pruneConfirmationArmed = true;
    this.clearPruneConfirmationResetTimer();
    this.#pruneConfirmationResetTimer = setTimeout(() => {
      this.pruneConfirmationArmed = false;
      this.#pruneConfirmationResetTimer = null;
    }, SAVED_SESSION_CONFIRMATION_RESET_MS);
  }

  private clearPruneConfirmation(): void {
    this.pruneConfirmationArmed = false;
    this.clearPruneConfirmationResetTimer();
  }

  private clearPruneConfirmationResetTimer(): void {
    if (this.#pruneConfirmationResetTimer) {
      clearTimeout(this.#pruneConfirmationResetTimer);
      this.#pruneConfirmationResetTimer = null;
    }
  }

  private resolveControls() {
    return resolveTerminalSavedSessionsControlState(this.snapshot, this.controlOptions());
  }

  private controlOptions() {
    return {
      visibleSavedSessionCount: this.visibleSavedSessionCount,
      pendingSavedSessionId: this.pendingSavedSessionId,
      pendingSavedSessionAction: this.pendingSavedSessionAction,
      pendingBulkAction: this.pendingBulkAction,
      deleteConfirmationSessionId: this.deleteConfirmationSessionId,
      pruneConfirmationArmed: this.pruneConfirmationArmed,
    };
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Saved layout action failed";
}
