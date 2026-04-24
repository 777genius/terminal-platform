import { css, html } from "lit";

import { terminalPlatformThemeManifests } from "@terminal-platform/design-tokens";
import { terminalPlatformTerminalFontScales } from "@terminal-platform/workspace-core";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalToolbarElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .toolbar {
        display: grid;
        grid-template-columns: 1fr;
        gap: var(--tp-space-3);
        padding: var(--tp-space-3);
      }

      .actions {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(8.5rem, 1fr));
        gap: var(--tp-space-2);
        align-items: stretch;
      }

      .actions button,
      .status {
        min-width: 0;
      }

      .status {
        align-self: center;
        justify-self: start;
      }

      .preference-group {
        display: grid;
        gap: var(--tp-space-2);
      }

      .preference-label {
        color: var(--tp-color-text-muted);
        font-size: 0.78rem;
      }

      .theme-options,
      .display-options {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-items: center;
      }

      .theme-options button[aria-pressed="true"],
      .display-options button[aria-pressed="true"] {
        border-color: color-mix(in srgb, var(--tp-color-accent) 58%, transparent);
        background: color-mix(in srgb, var(--tp-color-accent) 16%, var(--tp-color-panel-raised));
      }

      .toolbar-body {
        display: grid;
        gap: var(--tp-space-3);
      }
    `,
  ];

  override render() {
    const terminalDisplay = this.snapshot.terminalDisplay;

    return html`
      <div class="panel toolbar" part="toolbar">
        <div class="toolbar-body">
          <div>
            <div class="panel-eyebrow">Advanced tools</div>
            <div class="panel-title">Refresh shells, reconnect the workspace, and clear notices</div>
          </div>

          <div class="preference-group">
            <div class="preference-label">Theme</div>
            <div class="theme-options" part="theme-options" aria-label="Theme">
              ${terminalPlatformThemeManifests.map(
                (theme) => html`
                  <button
                    type="button"
                    part="theme-option"
                    data-testid="tp-theme-option"
                    data-theme-id=${theme.id}
                    aria-pressed=${String(this.snapshot.theme.themeId === theme.id)}
                    @click=${() => this.kernel?.commands.setTheme(theme.id)}
                  >
                    ${theme.displayName.replace("Terminal Platform ", "")}
                  </button>
                `,
              )}
            </div>
          </div>

          <div class="preference-group">
            <div class="preference-label">Terminal display</div>
            <div class="display-options" part="display-options" aria-label="Terminal display">
              ${terminalPlatformTerminalFontScales.map(
                (fontScale) => html`
                  <button
                    type="button"
                    part="font-scale-option"
                    data-testid="tp-font-scale-option"
                    data-font-scale=${fontScale}
                    aria-pressed=${String(terminalDisplay.fontScale === fontScale)}
                    @click=${() => this.kernel?.commands.setTerminalFontScale(fontScale)}
                  >
                    ${titleCase(fontScale)}
                  </button>
                `,
              )}
              <button
                type="button"
                part="line-wrap-option"
                data-testid="tp-line-wrap-option"
                aria-pressed=${String(terminalDisplay.lineWrap)}
                @click=${() => this.kernel?.commands.setTerminalLineWrap(!terminalDisplay.lineWrap)}
              >
                Wrap
              </button>
            </div>
          </div>
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

function titleCase(value: string): string {
  return `${value.slice(0, 1).toUpperCase()}${value.slice(1)}`;
}
