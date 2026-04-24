import { css, html } from "lit";
import type { TemplateResult } from "lit";

import type { MuxCommand, PaneTreeNode, SplitDirection } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  compactTerminalId,
  resolveTerminalTopologyControlState,
} from "./terminal-topology-controls.js";

export class TerminalPaneTreeElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    pendingAction: { state: true },
    actionError: { state: true },
  };

  static styles = [
    terminalElementStyles,
    css`
      .tree {
        display: grid;
        gap: var(--tp-space-3);
        padding: var(--tp-space-3);
      }

      .topology-summary,
      .tab-strip,
      .action-strip {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-items: center;
      }

      .topology-summary {
        justify-content: space-between;
      }

      .tab-strip {
        padding-block: 0.25rem;
      }

      .tab {
        max-width: 13rem;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
      }

      .tab[aria-pressed="true"],
      .node[aria-pressed="true"] {
        border-color: color-mix(in srgb, var(--tp-color-accent) 58%, transparent);
        background: var(--tp-color-accent-soft);
        color: var(--tp-color-text);
      }

      ul {
        list-style: none;
        margin: 0;
        padding-left: var(--tp-space-3);
        border-left: 1px solid var(--tp-color-border);
      }

      li + li {
        margin-top: var(--tp-space-2);
      }

      .node {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        min-height: 1.8rem;
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-sm);
        padding: 0.2rem 0.55rem;
        background: color-mix(in srgb, var(--tp-color-panel-raised) 72%, transparent);
        color: var(--tp-color-text-muted);
        font-size: 0.84rem;
      }

      button.node {
        font-family: var(--tp-font-family-mono);
      }

      .node__meta {
        color: var(--tp-color-text-muted);
        font-family: var(--tp-font-family-ui);
        font-size: 0.74rem;
      }

      .tree-body {
        min-width: 0;
        overflow-x: auto;
      }

      .notice {
        border: 1px solid color-mix(in srgb, var(--tp-color-warning) 45%, transparent);
        border-radius: var(--tp-radius-md);
        background: color-mix(in srgb, var(--tp-color-warning) 10%, transparent);
        color: var(--tp-color-text);
        padding: var(--tp-space-3);
      }

      @media (max-width: 720px) {
        .topology-summary {
          display: grid;
          justify-content: stretch;
        }

        .action-strip button {
          flex: 1 1 8rem;
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
    const topology = this.snapshot.attachedSession?.topology;
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    const focusedTab = controls.activeTab;
    const root = focusedTab?.root;
    const focusedPaneId = controls.activePaneId;
    const isPending = this.pendingAction !== null;

    return html`
      <div class="panel tree" part="tree" data-testid="tp-pane-tree">
        <div class="topology-summary">
          <div class="panel-header">
            <div class="panel-eyebrow">Layout</div>
            <div class="panel-title">Pane structure</div>
            <div class="panel-copy">
              ${focusedTab?.title ?? "Focused tab"} routing map.
              ${controls.capabilityStatus === "known" ? "Backend capabilities loaded." : "Backend capabilities pending."}
            </div>
          </div>
          <div class="muted" part="summary">${controls.tabCount} tabs / ${controls.paneCount} panes</div>
        </div>

        ${topology && topology.tabs.length > 0
          ? html`
              <div class="tab-strip" part="tab-strip" aria-label="Session tabs">
                ${topology.tabs.map(
                  (tab) => html`
                    <button
                      class="tab"
                      type="button"
                      part="tab"
                      data-testid="tp-topology-tab"
                      data-tab-id=${tab.tab_id}
                      title=${tab.tab_id}
                      aria-pressed=${String(tab.tab_id === topology.focused_tab)}
                      ?disabled=${isPending || !controls.canFocusTab}
                      @click=${() => this.focusTab(tab.tab_id)}
                    >
                      ${tab.title ?? compactTerminalId(tab.tab_id)}
                    </button>
                  `,
                )}
              </div>
            `
          : null}

        <div class="action-strip" part="topology-actions" aria-label="Topology actions">
          <button
            type="button"
            data-testid="tp-new-tab"
            ?disabled=${isPending || !controls.canCreateTab}
            @click=${() => this.newTab()}
          >
            New tab
          </button>
          <button
            type="button"
            data-testid="tp-split-horizontal"
            ?disabled=${isPending || !controls.canSplitPane}
            @click=${() => this.splitPane("horizontal")}
          >
            Split right
          </button>
          <button
            type="button"
            data-testid="tp-split-vertical"
            ?disabled=${isPending || !controls.canSplitPane}
            @click=${() => this.splitPane("vertical")}
          >
            Split down
          </button>
        </div>

        ${this.actionError
          ? html`
              <div class="notice" part="error">
                <strong>Layout command failed</strong>
                <div>${this.actionError}</div>
              </div>
            `
          : null}

        <div class="tree-body" part="tree-body">
          ${root
            ? renderNode(root, {
                canFocusPane: controls.canFocusPane,
                focusedPaneId,
                isPending,
                onFocus: (paneId) => this.focusPane(paneId),
              })
            : html`<div class="empty-state" part="empty">No pane tree yet</div>`}
        </div>
      </div>
    `;
  }

  private newTab(): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    this.runTopologyCommand("new_tab", {
      kind: "new_tab",
      title: `Tab ${controls.tabCount + 1}`,
    });
  }

  private splitPane(direction: SplitDirection): void {
    const paneId = resolveTerminalTopologyControlState(this.snapshot).activePaneId;
    if (!paneId) {
      return;
    }

    this.runTopologyCommand(`split_${direction}`, {
      kind: "split_pane",
      pane_id: paneId,
      direction,
    });
  }

  private focusTab(tabId: string): void {
    if (this.snapshot.attachedSession?.topology.focused_tab === tabId) {
      return;
    }

    this.runTopologyCommand("focus_tab", { kind: "focus_tab", tab_id: tabId });
  }

  private focusPane(paneId: string): void {
    if (resolveTerminalTopologyControlState(this.snapshot).activePaneId === paneId) {
      return;
    }

    this.runTopologyCommand("focus_pane", { kind: "focus_pane", pane_id: paneId });
  }

  private runTopologyCommand(action: string, command: MuxCommand): void {
    const sessionId = resolveTerminalTopologyControlState(this.snapshot).activeSessionId;
    if (!sessionId || this.pendingAction) {
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
          new CustomEvent("tp-terminal-topology-action-completed", {
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
          new CustomEvent("tp-terminal-topology-action-failed", {
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

interface RenderNodeOptions {
  canFocusPane: boolean;
  focusedPaneId: string | null;
  isPending: boolean;
  onFocus(paneId: string): void;
}

function renderNode(node: PaneTreeNode, options: RenderNodeOptions): TemplateResult {
  if (node.kind === "leaf") {
    const focused = node.pane_id === options.focusedPaneId;
    return html`
      <button
        class="node"
        type="button"
        part="leaf"
        data-testid="tp-pane-node"
        data-pane-id=${node.pane_id}
        title=${node.pane_id}
        aria-pressed=${String(focused)}
        ?disabled=${options.isPending || !options.canFocusPane}
        @click=${() => options.onFocus(node.pane_id)}
      >
        Pane ${compactTerminalId(node.pane_id)}
        ${focused ? html`<span class="node__meta">focused</span>` : null}
      </button>
    `;
  }

  return html`
    <div class="node" part="split">Split ${node.direction}</div>
    <ul part="children">
      <li>${renderNode(node.first, options)}</li>
      <li>${renderNode(node.second, options)}</li>
    </ul>
  `;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace topology command failed";
}
