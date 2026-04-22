import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalScreenElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .screen {
        padding: var(--tp-space-4);
        background: var(--tp-color-bg);
        min-height: 14rem;
      }

      pre {
        margin: 0;
        white-space: pre-wrap;
      }
    `,
  ];

  override render() {
    const screen = this.snapshot.attachedSession?.focused_screen;

    return html`
      <div class="panel screen" part="screen">
        ${screen
          ? html`
              <div class="muted" part="screen-title">${screen.surface.title ?? "Terminal"}</div>
              <pre part="screen-lines">${screen.surface.lines.map((line) => line.text).join("\n")}</pre>
            `
          : html`<div class="muted" part="empty">No active screen</div>`}
      </div>
    `;
  }
}
