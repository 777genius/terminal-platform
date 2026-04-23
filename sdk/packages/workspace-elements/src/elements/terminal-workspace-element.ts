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
        grid-template-columns: minmax(15rem, 19rem) minmax(0, 1fr);
        gap: var(--tp-space-4);
        min-height: 0;
      }

      .sidebar,
      .content {
        display: grid;
        gap: var(--tp-space-3);
        min-height: 0;
        align-content: start;
      }

      .sidebar {
        position: sticky;
        top: 0;
      }

      .diagnostics {
        padding: var(--tp-space-3);
      }

      .advanced-stack {
        display: grid;
        gap: var(--tp-space-3);
      }

      .secondary-toggle {
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-md);
        background: var(--tp-color-panel);
        overflow: hidden;
      }

      .secondary-toggle summary {
        cursor: pointer;
        list-style: none;
        padding: var(--tp-space-3);
        font-weight: 600;
      }

      .secondary-toggle summary::-webkit-details-marker {
        display: none;
      }

      .secondary-toggle[open] summary {
        border-bottom: 1px solid var(--tp-color-border);
      }

      .secondary-toggle .advanced-stack {
        padding: var(--tp-space-3);
      }

      .workspace-tools {
        display: grid;
        gap: var(--tp-space-3);
      }

      @media (max-width: 900px) {
        .body {
          grid-template-columns: 1fr;
        }

        .sidebar {
          position: static;
        }
      }
    `,
  ];

  override render() {
    return html`
      <div class="workspace" part="workspace">
        <tp-terminal-status-bar .kernel=${this.kernel}></tp-terminal-status-bar>
        <tp-terminal-command-dock .kernel=${this.kernel}></tp-terminal-command-dock>

        <div class="body" part="body">
          <div class="sidebar" part="sidebar">
            <tp-terminal-session-list .kernel=${this.kernel}></tp-terminal-session-list>
            <tp-terminal-saved-sessions .kernel=${this.kernel}></tp-terminal-saved-sessions>
          </div>
          <div class="content" part="content">
            <tp-terminal-screen .kernel=${this.kernel}></tp-terminal-screen>

            <details class="secondary-toggle workspace-tools">
              <summary>Workspace tools</summary>
              <div class="advanced-stack">
                <tp-terminal-pane-tree .kernel=${this.kernel}></tp-terminal-pane-tree>
                <tp-terminal-toolbar .kernel=${this.kernel}></tp-terminal-toolbar>
              </div>
            </details>

            <div class="advanced-stack" part="diagnostics-stack">
              ${this.snapshot.diagnostics.length > 0
                ? html`
                    <details class="secondary-toggle" open>
                      <summary>Workspace notices - ${this.snapshot.diagnostics.length}</summary>
                      <div class="panel diagnostics" part="diagnostics">
                        <div class="panel-header">
                          <div class="panel-eyebrow">Alerts</div>
                          <div class="panel-title">Workspace diagnostics</div>
                          <div class="panel-copy">These notices come from the transport or runtime layer.</div>
                        </div>
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
                    </details>
                  `
                : null}
            </div>
          </div>
        </div>
      </div>
    `;
  }
}
