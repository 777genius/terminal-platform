import { css, html } from "lit";
import type { BackendKind, DiscoveredSession } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { resolveTerminalEntityIdLabel } from "./terminal-identity.js";

const foreignBackendOrder: readonly BackendKind[] = ["tmux", "zellij"];

export class TerminalSessionListElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .panel {
        padding: var(--tp-space-3);
      }

      ul {
        list-style: none;
        margin: 0;
        padding: 0;
        max-height: 18rem;
        overflow: auto;
      }

      li + li {
        margin-top: var(--tp-space-2);
      }

      button {
        width: 100%;
        text-align: left;
        display: grid;
        gap: 0.32rem;
        padding: var(--tp-space-3);
      }

      button[data-active="true"] {
        border-color: color-mix(in srgb, var(--tp-color-accent) 42%, transparent);
        background: color-mix(in srgb, var(--tp-color-accent) 12%, transparent);
      }

      .row {
        display: flex;
        justify-content: space-between;
        gap: var(--tp-space-2);
        min-width: 0;
      }

      .title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .backend {
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        padding: 0.1rem 0.45rem;
        font-size: 0.72rem;
        color: var(--tp-color-text-muted);
      }

      .backend--foreign {
        color: var(--tp-color-accent);
      }

      .section-divider {
        margin: var(--tp-space-3) 0 var(--tp-space-2);
        border-top: 1px solid var(--tp-color-border);
        padding-top: var(--tp-space-3);
      }

      .panel-action {
        width: auto;
        min-height: 1.9rem;
        padding: 0.32rem 0.62rem;
        justify-self: start;
        text-align: center;
      }

      .discovered-title {
        min-width: 0;
      }

      code {
        display: block;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--tp-color-text-muted);
        font-size: 0.78rem;
      }
    `,
  ];

  override render() {
    const foreignBackends = this.foreignBackends();
    const discoveredGroups = foreignBackends.map((backend) => ({
      backend,
      sessions: this.snapshot.catalog.discoveredSessions[backend] ?? [],
    }));
    const discoveredCount = discoveredGroups.reduce((count, group) => count + group.sessions.length, 0);

    return html`
      <div class="panel" part="panel">
        <div class="panel-header">
          <div class="panel-eyebrow">Shells</div>
          <div class="panel-title">${this.snapshot.catalog.sessions.length || "No"} running shells</div>
          <div class="panel-copy">Choose where the terminal view and command dock should point.</div>
        </div>

        ${this.snapshot.catalog.sessions.length === 0
          ? html`<div class="empty-state" part="empty">Start a shell from the left rail and it will appear here.</div>`
          : html`
              <ul part="list">
                ${this.snapshot.catalog.sessions.map(
                  (session) => {
                    const identity = resolveTerminalEntityIdLabel(session.session_id, { prefix: "Session" });
                    return html`
                      <li part="item" data-testid="tp-session-list-item" data-session-id=${session.session_id}>
                        <button
                          part="button"
                          data-active=${String(this.snapshot.selection.activeSessionId === session.session_id)}
                          title=${session.session_id}
                          @click=${() => {
                            this.kernel?.commands.setActiveSession(session.session_id);
                            void this.kernel?.commands.attachSession(session.session_id).catch(() => {
                              // Command failures are already recorded in kernel diagnostics.
                            });
                          }}
                        >
                          <span class="row">
                            <strong class="title">${session.title ?? identity.label}</strong>
                            <span class="backend">${session.route.backend}</span>
                          </span>
                          <code data-testid="tp-session-id" title=${identity.title}>${identity.label}</code>
                        </button>
                      </li>
                    `;
                  },
                )}
              </ul>
            `}

        ${foreignBackends.length > 0
          ? html`
              <div class="section-divider" part="foreign-section" data-testid="tp-foreign-backends">
                <div class="panel-header">
                  <div class="panel-eyebrow">Foreign backends</div>
                  <div class="panel-title">${discoveredCount || "No"} importable sessions</div>
                  <button
                    class="panel-action"
                    part="foreign-refresh"
                    data-testid="tp-foreign-refresh"
                    title="Refresh importable tmux and Zellij sessions"
                    @click=${() => void this.refreshForeignBackends()}
                  >
                    Refresh
                  </button>
                </div>

                ${discoveredGroups.map((group) => this.renderDiscoveredGroup(group.backend, group.sessions))}
              </div>
            `
          : null}
      </div>
    `;
  }

  private renderDiscoveredGroup(backend: BackendKind, sessions: readonly DiscoveredSession[]) {
    return html`
      <div part="foreign-group" data-testid="tp-foreign-backend-group" data-backend=${backend}>
        <div class="panel-eyebrow">${backend}</div>
        ${sessions.length === 0
          ? html`<div class="empty-state" part="foreign-empty">No ${backend} sessions discovered.</div>`
          : html`
              <ul part="foreign-list">
                ${sessions.map((session) => this.renderDiscoveredSession(session))}
              </ul>
            `}
      </div>
    `;
  }

  private renderDiscoveredSession(session: DiscoveredSession) {
    const title = session.title ?? "Untitled foreign session";
    const reference = session.route.external?.value ?? session.route.backend;

    return html`
      <li
        part="foreign-item"
        data-testid="tp-discovered-session"
        data-backend=${session.route.backend}
        data-session-title=${title}
      >
        <button
          part="foreign-import-button"
          data-testid="tp-discovered-session-import"
          data-backend=${session.route.backend}
          title=${`Import ${title}`}
          @click=${() => void this.importDiscoveredSession(session)}
        >
          <span class="row">
            <strong class="title discovered-title">${title}</strong>
            <span class="backend backend--foreign">${session.route.backend}</span>
          </span>
          <code title=${reference}>${reference}</code>
        </button>
      </li>
    `;
  }

  private foreignBackends(): BackendKind[] {
    const advertised = new Set(this.snapshot.connection.handshake?.available_backends ?? []);
    return foreignBackendOrder.filter((backend) => advertised.has(backend));
  }

  private async refreshForeignBackends(): Promise<void> {
    await Promise.all(
      this.foreignBackends().map(async (backend) => {
        await this.kernel?.commands.discoverSessions(backend);
      }),
    ).catch(() => {
      // Discovery failures are recorded by the workspace kernel diagnostics.
    });
  }

  private async importDiscoveredSession(session: DiscoveredSession): Promise<void> {
    await this.kernel?.commands.importSession(session.route, session.title).catch(() => {
      // Import failures are recorded by the workspace kernel diagnostics.
    });
  }
}
