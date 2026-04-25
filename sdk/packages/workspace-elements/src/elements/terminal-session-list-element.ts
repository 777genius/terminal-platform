import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { resolveTerminalEntityIdLabel } from "./terminal-identity.js";

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
      </div>
    `;
  }
}
