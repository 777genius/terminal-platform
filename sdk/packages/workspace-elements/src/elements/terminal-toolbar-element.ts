import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalToolbarElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .toolbar {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: var(--tp-space-3);
        padding: var(--tp-space-3);
      }

      .actions {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        justify-content: flex-end;
      }

      .status {
        align-self: start;
      }

      @media (max-width: 900px) {
        .toolbar {
          grid-template-columns: 1fr;
        }

        .actions {
          justify-content: flex-start;
        }
      }
    `,
  ];

  override render() {
    return html`
      <div class="panel toolbar" part="toolbar">
        <div>
          <div class="panel-eyebrow">Advanced tools</div>
          <div class="panel-title">Refresh shells, reconnect the workspace, and clear notices</div>
        </div>

        <div class="actions">
          <button part="bootstrap" @click=${() => this.runCommand(() => this.kernel?.commands.bootstrap())}>
            Reconnect
          </button>
          <button part="refresh" @click=${() => this.runCommand(() => this.kernel?.commands.refreshSessions())}>
            Reload sessions
          </button>
          <button part="saved" @click=${() => this.runCommand(() => this.kernel?.commands.refreshSavedSessions())}>
            Reload saved
          </button>
          <button part="diagnostics" @click=${() => this.kernel?.commands.clearDiagnostics()}>
            Clear alerts
          </button>
          <span class="status muted" part="status">${this.snapshot.connection.state}</span>
        </div>
      </div>
    `;
  }

  private runCommand(command: () => Promise<unknown> | undefined): void {
    void command()?.catch(() => {
      // Command failures are already recorded in kernel diagnostics.
    });
  }
}
