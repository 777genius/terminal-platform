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
        <div class="panel-header">
          <div class="panel-eyebrow">Terminal</div>
          <div class="panel-title">${screen?.surface.title ?? "Live output"}</div>
          <div class="panel-copy">This is the focused pane. Commands from the dock are sent here.</div>
        </div>
        ${screen
          ? html`
              <pre part="screen-lines">${screen.surface.lines.map((line) => line.text).join("\n")}</pre>
            `
          : html`<div class="empty-state" part="empty">No active screen yet. Start or attach a session to see output here.</div>`}
      </div>
    `;
  }
}
