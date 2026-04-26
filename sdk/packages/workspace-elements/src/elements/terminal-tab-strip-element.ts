import { css, html, nothing } from "lit";
import { repeat } from "lit/directives/repeat.js";

import type { MuxCommand } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import { TERMINAL_DESTRUCTIVE_CONFIRMATION_RESET_MS } from "./terminal-destructive-action.js";
import {
  canRunTerminalTopologyCommand,
  resolveTerminalTopologyControlState,
} from "./terminal-topology-controls.js";
import {
  resolveTerminalTabStripControlState,
  type TerminalTabStripItemControlState,
} from "./terminal-tab-strip-controls.js";

export class TerminalTabStripElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    pendingAction: { state: true },
    actionError: { state: true },
    armedCloseTabKey: { state: true },
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
      .tab__main,
      .tab__close,
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
        display: inline-grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: center;
        max-width: min(15rem, 42vw);
        border: 1px solid color-mix(in srgb, var(--tp-terminal-color-border) 76%, transparent);
        padding: 0;
        overflow: hidden;
      }

      .tab[data-active="true"] {
        border-color: color-mix(in srgb, var(--tp-terminal-color-accent) 54%, transparent);
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-accent) 14%, var(--tp-terminal-color-bg-raised)),
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, transparent)
          );
        color: var(--tp-terminal-color-text);
      }

      .tab:focus-within {
        outline: 2px solid color-mix(in srgb, var(--tp-terminal-color-accent) 64%, transparent);
        outline-offset: -2px;
      }

      .tab__main,
      .tab__close {
        border: 0;
        border-radius: 0;
        background: transparent;
        min-width: 0;
      }

      .tab__main {
        display: inline-flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.2rem 0.34rem 0.2rem 0.62rem;
        color: inherit;
      }

      .tab__close {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 1.45rem;
        padding: 0.2rem 0.36rem;
        color: var(--tp-terminal-color-text-muted);
      }

      .tab__close:hover:not(:disabled),
      .tab__close[data-confirming="true"] {
        color: var(--tp-color-danger);
        background: color-mix(in srgb, var(--tp-color-danger) 14%, transparent);
      }

      .tab__close:disabled {
        opacity: 0.38;
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
        .tab__main,
        .tab__close,
        .new-tab {
          min-height: 1.5rem;
        }

        .tab__main,
        .tab__close,
        .new-tab {
          padding-block: 0.18rem;
        }
      }
    `,
  ];

  protected declare pendingAction: string | null;
  protected declare actionError: string | null;
  protected declare armedCloseTabKey: string | null;

  #closeConfirmationResetTimer: ReturnType<typeof setTimeout> | null = null;
  #closeConfirmationGeneration = 0;

  constructor() {
    super();
    this.pendingAction = null;
    this.actionError = null;
    this.armedCloseTabKey = null;
  }

  override disconnectedCallback(): void {
    this.clearCloseConfirmationResetTimer();
    super.disconnectedCallback();
  }

  override render() {
    const controls = resolveTerminalTabStripControlState(this.snapshot, {
      armedCloseTabKey: this.armedCloseTabKey,
      pending: this.pendingAction !== null,
    });

    return html`
      <div
        class="panel tab-strip"
        part="tab-strip"
        data-testid="tp-terminal-tab-strip"
        data-tab-count=${String(controls.tabCount)}
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

        ${controls.tabs.length > 0
          ? html`
              <div class="tabs" part="tabs" role="tablist" aria-label="Terminal tabs">
                ${repeat(
                  controls.tabs,
                  (tab) => tab.itemKey,
                  (tab) => html`
                    <div
                      class="tab"
                      part="tab"
                      data-active=${String(tab.active)}
                      data-confirming=${String(tab.closeArmed)}
                    >
                      <button
                        class="tab__main"
                        type="button"
                        part="tab-main"
                        role="tab"
                        data-testid="tp-terminal-tab"
                        data-tab-id=${tab.tabId}
                        title=${tab.title}
                        aria-pressed=${String(tab.active)}
                        aria-selected=${String(tab.active)}
                        ?disabled=${!tab.canFocus}
                        @click=${() => this.focusTab(tab.tabId)}
                      >
                        <span class="tab__title">${tab.label}</span>
                        <span class="tab__meta">${tab.metaLabel}</span>
                      </button>
                      <button
                        class="tab__close"
                        type="button"
                        part="tab-close"
                        data-testid="tp-terminal-tab-close"
                        data-tab-id=${tab.tabId}
                        data-danger="true"
                        data-confirming=${String(tab.closeArmed)}
                        title=${tab.closeTitle}
                        aria-label=${tab.closeTitle}
                        ?disabled=${!tab.canClose}
                        @click=${(event: Event) => this.closeTab(tab, event)}
                      >
                        x
                      </button>
                    </div>
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
          ?disabled=${!controls.canCreateTab}
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

    this.clearCloseConfirmation();
    this.runTopologyCommand("focus_tab", { kind: "focus_tab", tab_id: tabId });
  }

  private newTab(): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    if (!controls.canCreateTab) {
      return;
    }

    this.clearCloseConfirmation();
    this.runTopologyCommand("new_tab", {
      kind: "new_tab",
      title: `Tab ${controls.tabCount + 1}`,
    });
  }

  private closeTab(tab: TerminalTabStripItemControlState, event: Event): void {
    event.stopPropagation();
    const controls = resolveTerminalTabStripControlState(this.snapshot, {
      armedCloseTabKey: this.armedCloseTabKey,
      pending: this.pendingAction !== null,
    });
    const currentTab = controls.tabs.find((item) => item.itemKey === tab.itemKey);
    if (!currentTab?.canClose) {
      return;
    }

    if (!currentTab.closeArmed) {
      this.setCloseConfirmation(tab.itemKey);
      return;
    }

    this.clearCloseConfirmation();
    this.runTopologyCommand("close_tab", { kind: "close_tab", tab_id: currentTab.tabId });
  }

  private setCloseConfirmation(tabKey: string): void {
    this.actionError = null;
    this.armedCloseTabKey = tabKey;
    this.clearCloseConfirmationResetTimer();
    const generation = ++this.#closeConfirmationGeneration;
    this.#closeConfirmationResetTimer = setTimeout(() => {
      if (generation === this.#closeConfirmationGeneration && this.armedCloseTabKey === tabKey) {
        this.armedCloseTabKey = null;
      }
      this.#closeConfirmationResetTimer = null;
    }, TERMINAL_DESTRUCTIVE_CONFIRMATION_RESET_MS);
  }

  private clearCloseConfirmation(): void {
    this.#closeConfirmationGeneration += 1;
    this.armedCloseTabKey = null;
    this.clearCloseConfirmationResetTimer();
  }

  private clearCloseConfirmationResetTimer(): void {
    if (this.#closeConfirmationResetTimer) {
      clearTimeout(this.#closeConfirmationResetTimer);
      this.#closeConfirmationResetTimer = null;
    }
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
