import { css, html } from "lit";
import type { PropertyValues, TemplateResult } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalScreenElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    followOutput: { state: true },
  };

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
        min-height: 18rem;
      }

      .screen-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: var(--tp-space-3);
      }

      .screen-header .panel-header {
        margin-bottom: 0;
      }

      .screen-actions {
        display: flex;
        flex-wrap: wrap;
        justify-content: flex-end;
        gap: var(--tp-space-2);
      }

      .screen-actions button {
        white-space: nowrap;
      }

      .screen-actions button[aria-pressed="true"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 52%, transparent);
        color: var(--tp-color-success);
      }

      .viewport {
        margin: 0;
        min-height: 12rem;
        max-height: min(28vh, 26rem);
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

      @media (max-width: 720px) {
        .screen-header {
          display: grid;
        }

        .screen-actions {
          justify-content: flex-start;
        }
      }
    `,
  ];

  protected declare followOutput: boolean;

  #autoScrolling = false;

  constructor() {
    super();
    this.followOutput = true;
  }

  protected override updated(changedProperties: PropertyValues): void {
    if (!changedProperties.has("snapshot") && !changedProperties.has("followOutput")) {
      return;
    }

    if (this.followOutput && this.snapshot.attachedSession?.focused_screen) {
      this.scrollViewportToBottom();
    }
  }

  override render() {
    const screen = this.snapshot.attachedSession?.focused_screen;

    return html`
      <div class="panel screen" part="screen" data-testid="tp-terminal-screen">
        <div class="screen-header">
          <div class="panel-header">
            <div class="panel-eyebrow">Terminal</div>
            <div class="panel-title">${screen?.surface.title ?? "Live output"}</div>
            <div class="panel-copy">Focused pane output. Input from the command dock is routed here.</div>
          </div>
          <div class="screen-actions" part="screen-actions">
            <button
              type="button"
              data-testid="tp-screen-follow"
              aria-pressed=${String(this.followOutput)}
              ?disabled=${!screen}
              @click=${() => this.toggleFollowOutput()}
            >
              ${this.followOutput ? "Following" : "Paused"}
            </button>
            <button
              type="button"
              data-testid="tp-screen-scroll-latest"
              ?disabled=${!screen}
              @click=${() => this.scrollLatest()}
            >
              Scroll latest
            </button>
          </div>
        </div>
        ${screen
          ? html`
              <div class="screen-meta" part="meta">
                <span>${screen.cols} columns</span>
                <span>${screen.rows} rows</span>
                <span>seq ${String(screen.sequence)}</span>
                <span>${screen.source}</span>
              </div>
              <div
                class="viewport"
                part="screen-lines"
                data-testid="tp-screen-viewport"
                @scroll=${(event: Event) => this.handleViewportScroll(event)}
              >
                ${screen.surface.lines.map((line, index) => renderLine(index + 1, line.text))}
              </div>
            `
          : html`<div class="empty-state" part="empty">No active screen yet. Start or attach a session to see output here.</div>`}
      </div>
    `;
  }

  private toggleFollowOutput(): void {
    this.followOutput = !this.followOutput;
    if (this.followOutput) {
      this.scrollViewportToBottom();
    }
  }

  private scrollLatest(): void {
    this.followOutput = true;
    this.scrollViewportToBottom();
  }

  private handleViewportScroll(event: Event): void {
    if (this.#autoScrolling) {
      return;
    }

    const viewport = event.currentTarget as HTMLElement;
    this.followOutput = isViewportAtBottom(viewport);
  }

  private scrollViewportToBottom(): void {
    const viewport = this.shadowRoot?.querySelector<HTMLElement>('[data-testid="tp-screen-viewport"]');
    if (!viewport) {
      return;
    }

    this.#autoScrolling = true;
    viewport.scrollTop = viewport.scrollHeight;
    requestAnimationFrame(() => {
      this.#autoScrolling = false;
    });
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

function isViewportAtBottom(viewport: HTMLElement): boolean {
  return viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 2;
}
