import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalWorkspaceElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .workspace {
        display: grid;
        gap: var(--tp-space-4);
      }

      .body {
        display: grid;
        grid-template-columns: minmax(16rem, 20rem) 1fr;
        gap: var(--tp-space-4);
      }

      .sidebar,
      .content {
        display: grid;
        gap: var(--tp-space-3);
      }

      .diagnostics {
        padding: var(--tp-space-3);
      }

      @media (max-width: 900px) {
        .body {
          grid-template-columns: 1fr;
        }
      }
    `,
  ];

  override render() {
    return html`
      <div class="workspace" part="workspace">
        <tp-terminal-toolbar .kernel=${this.kernel}></tp-terminal-toolbar>
        <div class="body" part="body">
          <div class="sidebar" part="sidebar">
            <tp-terminal-session-list .kernel=${this.kernel}></tp-terminal-session-list>
            <tp-terminal-saved-sessions .kernel=${this.kernel}></tp-terminal-saved-sessions>
          </div>
          <div class="content" part="content">
            <tp-terminal-pane-tree .kernel=${this.kernel}></tp-terminal-pane-tree>
            ${this.snapshot.diagnostics.length > 0
              ? html`
                  <div class="panel diagnostics" part="diagnostics">
                    <ul>
                      ${this.snapshot.diagnostics.map(
                        (item) => html`
                          <li>
                            <strong>${item.code}</strong>
                            <span class="muted"> ${item.message}</span>
                          </li>
                        `,
                      )}
                    </ul>
                  </div>
                `
              : null}
            <tp-terminal-screen .kernel=${this.kernel}></tp-terminal-screen>
          </div>
        </div>
      </div>
    `;
  }
}
