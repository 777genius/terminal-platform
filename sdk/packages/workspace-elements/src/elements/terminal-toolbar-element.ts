import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalToolbarElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .toolbar {
        display: flex;
        align-items: center;
        gap: var(--tp-space-2);
        padding: var(--tp-space-3);
      }

      .status {
        margin-left: auto;
      }
    `,
  ];

  override render() {
    return html`
      <div class="panel toolbar" part="toolbar">
        <button part="bootstrap" @click=${() => this.runCommand(() => this.kernel?.commands.bootstrap())}>
          Bootstrap
        </button>
        <button part="refresh" @click=${() => this.runCommand(() => this.kernel?.commands.refreshSessions())}>
          Refresh Sessions
        </button>
        <button part="saved" @click=${() => this.runCommand(() => this.kernel?.commands.refreshSavedSessions())}>
          Refresh Saved
        </button>
        <button part="diagnostics" @click=${() => this.kernel?.commands.clearDiagnostics()}>
          Clear Diagnostics
        </button>
        <span class="status muted" part="status">${this.snapshot.connection.state}</span>
      </div>
    `;
  }

  private runCommand(command: () => Promise<unknown> | undefined): void {
    void command()?.catch(() => {
      // Command failures are already recorded in kernel diagnostics.
    });
  }
}
