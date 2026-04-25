import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  defaultTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";

export class TerminalWorkspaceElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    quickCommands: { attribute: false },
  };

  static styles = [
    terminalElementStyles,
    css`
      .workspace {
        --tp-workspace-sidebar-default-width: clamp(14rem, 21vw, 19rem);
        --tp-workspace-inspector-default-width: clamp(16rem, 24vw, 24rem);
        --tp-workspace-sidebar-width: var(
          --tp-workspace-sidebar-target-width,
          var(--tp-workspace-sidebar-default-width)
        );
        --tp-workspace-inspector-width: var(
          --tp-workspace-inspector-target-width,
          var(--tp-workspace-inspector-default-width)
        );

        display: grid;
        gap: var(--tp-space-4);
        min-height: 0;
      }

      .body {
        display: grid;
        grid-template-columns: minmax(14rem, var(--tp-workspace-sidebar-width)) minmax(0, 1fr);
        gap: var(--tp-space-4);
        min-height: 0;
        align-items: start;
      }

      .sidebar,
      .content {
        display: grid;
        gap: var(--tp-space-3);
        min-height: 0;
        align-content: start;
        min-width: 0;
      }

      .sidebar {
        position: sticky;
        top: 0;
      }

      .content {
        container-type: inline-size;
      }

      .operations-deck {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(16rem, var(--tp-workspace-inspector-width));
        gap: var(--tp-space-3);
        align-items: start;
        min-height: 0;
      }

      .terminal-column,
      .inspector-column {
        display: grid;
        gap: var(--tp-space-3);
        min-width: 0;
        min-height: 0;
      }

      .terminal-column {
        grid-template-rows: auto auto;
        align-content: start;
      }

      .inspector-column {
        position: sticky;
        top: 0;
        align-content: start;
        max-height: calc(100vh - var(--tp-space-4));
        overflow: auto;
        padding-right: 0.15rem;
        scrollbar-gutter: stable;
      }

      .command-region {
        min-width: 0;
      }

      tp-terminal-screen,
      tp-terminal-pane-tree,
      tp-terminal-command-dock,
      tp-terminal-toolbar,
      tp-terminal-session-list,
      tp-terminal-saved-sessions {
        display: block;
        min-width: 0;
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

      @media (max-width: 1180px) {
        .operations-deck {
          grid-template-columns: 1fr;
        }

        .inspector-column {
          position: static;
          max-height: none;
          overflow: visible;
          padding-right: 0;
        }
      }

      @container (max-width: 50rem) {
        .operations-deck {
          grid-template-columns: 1fr;
        }

        .inspector-column {
          position: static;
          max-height: none;
          overflow: visible;
          padding-right: 0;
        }
      }

      @media (max-width: 900px) {
        .body {
          grid-template-columns: 1fr;
        }

        .content {
          order: 1;
        }

        .sidebar {
          order: 2;
          position: static;
        }
      }
    `,
  ];

  declare quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined;

  constructor() {
    super();
    this.quickCommands = defaultTerminalCommandQuickCommands;
  }

  override render() {
    return html`
      <div class="workspace" part="workspace">
        <tp-terminal-status-bar .kernel=${this.kernel}></tp-terminal-status-bar>

        <div
          class="body"
          part="body"
          data-testid="tp-workspace-layout"
          data-layout="operations-deck"
        >
          <div class="sidebar" part="sidebar">
            <tp-terminal-session-list .kernel=${this.kernel}></tp-terminal-session-list>
            <tp-terminal-saved-sessions .kernel=${this.kernel}></tp-terminal-saved-sessions>
          </div>
          <div class="content" part="content">
            <div class="operations-deck" part="operations-deck" data-testid="tp-workspace-operations-deck">
              <div class="terminal-column" part="terminal-column" data-testid="tp-workspace-terminal-column">
                <tp-terminal-screen .kernel=${this.kernel}></tp-terminal-screen>
                <div class="command-region" part="command-region" data-testid="tp-workspace-command-region">
                  <tp-terminal-command-dock
                    .kernel=${this.kernel}
                    .quickCommands=${this.quickCommands}
                  ></tp-terminal-command-dock>
                </div>
              </div>

              <div class="inspector-column" part="inspector-column" data-testid="tp-workspace-inspector-column">
                <tp-terminal-pane-tree .kernel=${this.kernel}></tp-terminal-pane-tree>

                <details class="secondary-toggle workspace-tools">
                  <summary>Workspace tools</summary>
                  <div class="advanced-stack">
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
        </div>
      </div>
    `;
  }
}
