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
        <ul part="list">
          ${this.snapshot.catalog.savedSessions.map(
            (session) => html`
              <li part="item">
                <div><strong>${session.title ?? session.session_id}</strong></div>
                <div class="muted">${session.compatibility.status}</div>
                <div class="actions" part="actions">
                  <button @click=${() => void this.kernel?.commands.restoreSavedSession(session.session_id)}>
                    Restore
                  </button>
                  <button @click=${() => void this.kernel?.commands.deleteSavedSession(session.session_id)}>
                    Delete
                  </button>
                </div>
              </li>
            `,
          )}
        </ul>
      </div>
    `;
  }
}
