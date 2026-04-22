import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalSavedSessionsElement extends WorkspaceKernelConsumerElement {
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

      .actions {
        display: flex;
        gap: var(--tp-space-2);
        margin-top: var(--tp-space-2);
      }
    `,
  ];

  override render() {
    return html`
      <div class="panel saved" part="saved">
        <div class="panel-header">
          <div class="panel-eyebrow">Saved layouts</div>
          <div class="panel-title">${this.snapshot.catalog.savedSessions.length || "No"} saved sessions</div>
          <div class="panel-copy">Restore a saved layout or clean up entries you no longer need.</div>
        </div>

        ${this.snapshot.catalog.savedSessions.length === 0
          ? html`<div class="empty-state" part="empty">Saved sessions will appear here after you save a layout.</div>`
          : html`
              <ul part="list">
                ${this.snapshot.catalog.savedSessions.map(
                  (session) => html`
                    <li part="item">
                      <div><strong>${session.title ?? session.session_id}</strong></div>
                      <div class="muted">${session.compatibility.status}</div>
                      <div class="actions" part="actions">
                        <button
                          @click=${() => {
                            void this.kernel?.commands.restoreSavedSession(session.session_id).catch(() => {
                              // Command failures are already recorded in kernel diagnostics.
                            });
                          }}
                        >
                          Restore
                        </button>
                        <button
                          @click=${() => {
                            void this.kernel?.commands.deleteSavedSession(session.session_id).catch(() => {
                              // Command failures are already recorded in kernel diagnostics.
                            });
                          }}
                        >
                          Delete
                        </button>
                      </div>
                    </li>
                  `,
                )}
              </ul>
            `}
      </div>
    `;
  }
}
