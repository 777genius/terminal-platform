import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

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
        gap: 0.18rem;
      }

      button[data-active="true"] {
        border-color: color-mix(in srgb, var(--tp-color-accent) 42%, transparent);
        background: color-mix(in srgb, var(--tp-color-accent) 12%, transparent);
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
                  (session) => html`
                    <li part="item">
                      <button
                        part="button"
                        data-active=${String(this.snapshot.selection.activeSessionId === session.session_id)}
                        @click=${() => {
                          this.kernel?.commands.setActiveSession(session.session_id);
                          void this.kernel?.commands.attachSession(session.session_id).catch(() => {
                            // Command failures are already recorded in kernel diagnostics.
                          });
                        }}
                      >
                        <strong>${session.title ?? session.session_id}</strong>
                        <div class="muted">${session.route.backend}</div>
                      </button>
                    </li>
                  `,
                )}
              </ul>
            `}
      </div>
    `;
  }
}
