import { css, html, nothing } from "lit";
import { repeat } from "lit/directives/repeat.js";

import type { MuxCommand } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  canRunTerminalTopologyCommand,
  compactTerminalId,
  resolveTerminalTopologyControlState,
} from "./terminal-topology-controls.js";

export class TerminalTabStripElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    pendingAction: { state: true },
    actionError: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .tab-strip {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: var(--tp-space-2);
        align-items: center;
        min-height: 1.75rem;
        overflow: hidden;
        border-bottom-right-radius: 0;
        border-bottom-left-radius: 0;
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 90%, transparent),
            color-mix(in srgb, var(--tp-terminal-color-bg) 96%, transparent)
          ),
          var(--tp-terminal-color-bg);
        color: var(--tp-terminal-color-text);
        padding: 0.16rem 0.44rem 0;
      }

      .tabs {
        display: flex;
        gap: 0.25rem;
        min-width: 0;
        overflow-x: auto;
        scrollbar-width: none;
      }

      .tabs::-webkit-scrollbar {
        display: none;
      }

      .tab,
      .new-tab {
        min-height: 1.58rem;
        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 76%, transparent);
        border-bottom-right-radius: 0;
        border-bottom-left-radius: 0;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 66%, transparent);
        color: var(--tp-terminal-color-text-muted);
        font-size: 0.78rem;
        line-height: 1.1;
        white-space: nowrap;
      }

      .tab {
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
        max-width: min(15rem, 42vw);
        padding: 0.2rem 0.62rem;
      }

      .tab[aria-pressed="true"] {
        border-color: color-mix(in srgb, var(--tp-terminal-color-accent) 54%, transparent);
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-accent) 14%, var(--tp-terminal-color-bg-raised)),
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, transparent)
          );
        color: var(--tp-terminal-color-text);
      }

      .tab__title {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .tab__meta {
        flex: 0 0 auto;
        color: var(--tp-terminal-color-text-muted);
        font-family: var(--tp-font-family-mono);
        font-size: 0.72rem;
      }

      .new-tab {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        min-width: 1.72rem;
        padding: 0.18rem 0.5rem;
      }

      .empty {
        color: var(--tp-terminal-color-text-muted);
        font-size: 0.82rem;
        padding: 0.35rem 0.5rem 0.55rem;
      }

      .notice {
        grid-column: 1 / -1;
        margin: 0 0 var(--tp-space-2);
        border: 1px solid color-mix(in srgb, var(--tp-color-warning) 45%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-warning) 10%, transparent);
        color: var(--tp-terminal-color-text);
        padding: var(--tp-space-3);
      }

      @media (max-width: 720px) {
        .tab-strip {
          min-height: 1.68rem;
          padding-inline: var(--tp-space-2);
        }

        .tab,
        .new-tab {
          min-height: 1.5rem;
          padding-block: 0.18rem;
        }
      }
    `,
  ];

  protected declare pendingAction: string | null;
  protected declare actionError: string | null;

  constructor() {
    super();
    this.pendingAction = null;
    this.actionError = null;
  }

  override render() {
    const topology = this.snapshot.attachedSession?.topology ?? null;
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    const isPending = this.pendingAction !== null;
    const activeTabIndex = topology?.tabs.findIndex((tab) => tab.tab_id === controls.activeTab?.tab_id) ?? -1;
    const tabCount = topology?.tabs.length ?? 0;

    return html`
      <div
        class="panel tab-strip"
        part="tab-strip"
        data-testid="tp-terminal-tab-strip"
        data-tab-count=${String(tabCount)}
        data-capability-status=${controls.capabilityStatus}
      >
        ${this.actionError
          ? html`
              <div class="notice" part="error">
                <strong>Tab command failed</strong>
                <div>${this.actionError}</div>
              </div>
            `
          : nothing}

        ${topology && topology.tabs.length > 0
          ? html`
              <div class="tabs" part="tabs" aria-label="Terminal tabs">
                ${repeat(
                  topology.tabs,
                  (tab, index) => `${tab.tab_id}:${index}`,
                  (tab, index) => html`
                    <button
                      class="tab"
                      type="button"
                      part="tab"
                      data-testid="tp-terminal-tab"
                      data-tab-id=${tab.tab_id}
                      title=${tab.tab_id}
                      aria-pressed=${String(index === activeTabIndex)}
                      ?disabled=${isPending || !controls.canFocusTab}
                      @click=${() => this.focusTab(tab.tab_id)}
                    >
                      <span class="tab__title">${tab.title ?? compactTerminalId(tab.tab_id)}</span>
                      <span class="tab__meta">${compactTerminalId(tab.tab_id)}</span>
                    </button>
                  `,
                )}
              </div>
            `
          : html`<div class="empty" part="empty">No terminal tabs</div>`}

        <button
          class="new-tab"
          type="button"
          part="new-tab"
          data-testid="tp-terminal-new-tab"
          title="Create terminal tab"
          aria-label="Create terminal tab"
          ?disabled=${isPending || !controls.canCreateTab}
          @click=${() => this.newTab()}
        >
          +
        </button>
      </div>
    `;
  }

  private focusTab(tabId: string): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    if (!controls.canFocusTab || this.snapshot.attachedSession?.topology?.focused_tab === tabId) {
      return;
    }

    this.runTopologyCommand("focus_tab", { kind: "focus_tab", tab_id: tabId });
  }

  private newTab(): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    if (!controls.canCreateTab) {
      return;
    }

    this.runTopologyCommand("new_tab", {
      kind: "new_tab",
      title: `Tab ${controls.tabCount + 1}`,
    });
  }

  private runTopologyCommand(action: string, command: MuxCommand): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    const sessionId = controls.activeSessionId;
    if (!sessionId || this.pendingAction || !canRunTerminalTopologyCommand(controls, command)) {
      return;
    }

    this.pendingAction = action;
    this.actionError = null;

    const commandPromise = this.kernel?.commands.dispatchMuxCommand(sessionId, command);
    if (!commandPromise) {
      this.pendingAction = null;
      return;
    }

    void commandPromise
      .then(async () => {
        await this.kernel?.commands.attachSession(sessionId);
        this.dispatchEvent(
          new CustomEvent("tp-terminal-tab-strip-action-completed", {
            bubbles: true,
            composed: true,
            detail: {
              action,
              sessionId,
            },
          }),
        );
      })
      .catch((error: unknown) => {
        this.actionError = getErrorMessage(error);
        this.dispatchEvent(
          new CustomEvent("tp-terminal-tab-strip-action-failed", {
            bubbles: true,
            composed: true,
            detail: {
              action,
              error,
              sessionId,
            },
          }),
        );
      })
      .finally(() => {
        this.pendingAction = null;
      });
  }
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Terminal tab command failed";
}
