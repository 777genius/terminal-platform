import { css, html } from "lit";
import type { TemplateResult } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalScreenElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .screen {
        display: grid;
        gap: var(--tp-space-3);
        padding: var(--tp-space-4);
        background:
          linear-gradient(180deg, color-mix(in srgb, var(--tp-color-bg-inset) 92%, transparent), var(--tp-color-bg)),
          var(--tp-color-bg);
        min-height: 22rem;
      }

      .viewport {
        margin: 0;
        min-height: 16rem;
        max-height: min(56vh, 42rem);
        overflow: auto;
        border: 1px solid color-mix(in srgb, var(--tp-color-border) 70%, transparent);
        border-radius: var(--tp-radius-lg);
        background: #05070b;
        padding: var(--tp-space-3);
        font-family: var(--tp-font-family-mono);
        font-size: 0.9rem;
        line-height: 1.48;
        scrollbar-gutter: stable;
      }

      .line {
        display: grid;
        grid-template-columns: 3.25rem minmax(0, 1fr);
        gap: var(--tp-space-2);
        min-height: 1.35rem;
      }

      .gutter {
        color: color-mix(in srgb, var(--tp-color-text-muted) 48%, transparent);
        text-align: right;
        user-select: none;
      }

      .text {
        white-space: pre-wrap;
        overflow-wrap: anywhere;
      }

      .screen-meta {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        color: var(--tp-color-text-muted);
        font-size: 0.82rem;
      }

      .screen-meta span {
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        padding: 0.2rem 0.5rem;
        background: color-mix(in srgb, var(--tp-color-panel-raised) 60%, transparent);
      }
    `,
  ];

  override render() {
    const screen = this.snapshot.attachedSession?.focused_screen;

    return html`
      <div class="panel screen" part="screen" data-testid="tp-terminal-screen">
        <div class="panel-header">
          <div class="panel-eyebrow">Terminal</div>
          <div class="panel-title">${screen?.surface.title ?? "Live output"}</div>
          <div class="panel-copy">Focused pane output. Input from the command dock is routed here.</div>
        </div>
        ${screen
          ? html`
              <div class="screen-meta" part="meta">
                <span>${screen.cols} columns</span>
                <span>${screen.rows} rows</span>
                <span>seq ${String(screen.sequence)}</span>
                <span>${screen.source}</span>
              </div>
              <div class="viewport" part="screen-lines">
                ${screen.surface.lines.map((line, index) => renderLine(index + 1, line.text))}
              </div>
            `
          : html`<div class="empty-state" part="empty">No active screen yet. Start or attach a session to see output here.</div>`}
      </div>
    `;
  }
}

function renderLine(index: number, text: string): TemplateResult {
  return html`
    <div class="line" part="screen-line">
      <span class="gutter" part="line-number">${index}</span>
      <span class="text" part="line-text">${text.length > 0 ? text : " "}</span>
    </div>
  `;
}
