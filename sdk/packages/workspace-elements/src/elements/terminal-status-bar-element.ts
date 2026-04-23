import { css, html } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalStatusBarElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .status {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: var(--tp-space-3);
        align-items: center;
        padding: var(--tp-space-3) var(--tp-space-4);
      }

      .primary,
      .metrics {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-items: center;
        min-width: 0;
      }

      .title {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-weight: 700;
      }

      .pill {
        display: inline-flex;
        align-items: center;
        min-height: 1.7rem;
        max-width: 100%;
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        padding: 0.24rem 0.55rem;
        background: color-mix(in srgb, var(--tp-color-panel-raised) 76%, transparent);
        color: var(--tp-color-text-muted);
        font-size: 0.78rem;
      }

      .pill[data-tone="ready"] {
        border-color: color-mix(in srgb, var(--tp-color-success) 42%, transparent);
        color: var(--tp-color-success);
      }

      .pill[data-tone="warn"] {
        border-color: color-mix(in srgb, var(--tp-color-warning) 44%, transparent);
        color: var(--tp-color-warning);
      }

      .pill[data-tone="danger"] {
        border-color: color-mix(in srgb, var(--tp-color-danger) 44%, transparent);
        background: var(--tp-color-danger-soft);
        color: var(--tp-color-danger);
      }

      code {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      @media (max-width: 820px) {
        .status {
          grid-template-columns: 1fr;
        }

        .metrics {
          justify-content: flex-start;
        }
      }
    `,
  ];

  override render() {
    const activeSessionId = this.snapshot.selection.activeSessionId;
    const activeSession =
      this.snapshot.catalog.sessions.find((session) => session.session_id === activeSessionId)
      ?? this.snapshot.attachedSession?.session
      ?? null;
    const screen = this.snapshot.attachedSession?.focused_screen ?? null;
    const health = this.snapshot.attachedSession?.health ?? null;
    const connectionTone = connectionToneFor(this.snapshot.connection.state);
    const healthTone = healthToneFor(health?.phase ?? null);

    return html`
      <div class="panel status" part="status-bar" data-testid="tp-status-bar">
        <div class="primary">
          <span class="title" part="title">
            ${activeSession ? activeSession.title ?? activeSession.session_id : "No active session"}
          </span>
          <span class="pill" part="connection" data-tone=${connectionTone}>
            ${connectionLabelFor(this.snapshot.connection.state)}
          </span>
          <span class="pill" part="health" data-tone=${healthTone}>
            ${health ? healthLabelFor(health.phase) : "No health snapshot"}
          </span>
        </div>

        <div class="metrics" part="metrics">
          ${activeSession
            ? html`<span class="pill" part="session"><code>${activeSession.session_id}</code></span>`
            : null}
          ${screen
            ? html`
                <span class="pill" part="pane"><code>${screen.pane_id}</code></span>
                <span class="pill" part="screen-size">${screen.cols}x${screen.rows}</span>
              `
            : null}
          ${this.snapshot.diagnostics.length > 0
            ? html`<span class="pill" part="diagnostics" data-tone="warn">
                ${this.snapshot.diagnostics.length} notices
              </span>`
            : null}
        </div>
      </div>
    `;
  }
}

function connectionLabelFor(state: string): string {
  if (state === "ready") {
    return "Connected";
  }

  if (state === "error") {
    return "Connection issue";
  }

  if (state === "disposed") {
    return "Closed";
  }

  if (state === "bootstrapping") {
    return "Connecting";
  }

  return "Idle";
}

function connectionToneFor(state: string): "ready" | "warn" | "danger" | "muted" {
  if (state === "ready") {
    return "ready";
  }

  if (state === "error" || state === "disposed") {
    return "danger";
  }

  if (state === "bootstrapping") {
    return "warn";
  }

  return "muted";
}

function healthLabelFor(phase: string): string {
  if (phase === "ready") {
    return "Healthy";
  }

  if (phase === "degraded") {
    return "Degraded";
  }

  if (phase === "stale") {
    return "Stale";
  }

  if (phase === "terminated") {
    return "Terminated";
  }

  return phase;
}

function healthToneFor(phase: string | null): "ready" | "warn" | "danger" | "muted" {
  if (phase === "ready") {
    return "ready";
  }

  if (phase === "degraded" || phase === "stale") {
    return "warn";
  }

  if (phase === "terminated") {
    return "danger";
  }

  return "muted";
}
