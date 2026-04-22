import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalSessionListElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      ul {
        list-style: none;
        margin: 0;
        padding: 0;
      }

      li + li {
        margin-top: var(--tp-space-2);
      }

      button {
        width: 100%;
        text-align: left;
      }
    `,
  ];

  override render() {
    return html`
      <div class="panel" part="panel">
        <ul part="list">
          ${this.snapshot.catalog.sessions.map(
            (session) => html`
              <li part="item">
                <button
                  part="button"
                  @click=${() => {
                    this.kernel?.commands.setActiveSession(session.session_id);
                    void this.kernel?.commands.attachSession(session.session_id);
                  }}
                >
                  <strong>${session.title ?? session.session_id}</strong>
                  <div class="muted">${session.route.backend}</div>
                </button>
              </li>
            `,
          )}
        </ul>
      </div>
    `;
  }
}
