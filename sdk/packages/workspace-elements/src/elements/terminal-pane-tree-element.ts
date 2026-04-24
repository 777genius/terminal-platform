import { css, html } from "lit";
import type { TemplateResult } from "lit";

import type { MuxCommand, PaneTreeNode, SplitDirection } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  compactTerminalId,
  resolvePaneResizeCommand,
  resolveTerminalTopologyControlState,
  TERMINAL_PANE_MAX_COLS,
  TERMINAL_PANE_MAX_ROWS,
  TERMINAL_PANE_MIN_COLS,
  TERMINAL_PANE_MIN_ROWS,
  type TerminalPaneResizeDelta,
} from "./terminal-topology-controls.js";

export class TerminalPaneTreeElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    pendingAction: { state: true },
    actionError: { state: true },
    renamingTabId: { state: true },
    renameDraft: { state: true },
    armedCloseAction: { state: true },
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

      .rename-form {
        display: grid;
        grid-template-columns: minmax(9rem, 1fr) auto auto;
        gap: var(--tp-space-2);
        align-items: center;
      }

      .rename-form input {
        min-width: 0;
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-sm);
        background: color-mix(in srgb, var(--tp-color-panel-raised) 80%, transparent);
        color: var(--tp-color-text);
        font: inherit;
        padding: 0.42rem 0.6rem;
      }

      .pane-row {
        display: inline-flex;
        max-width: 100%;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-items: center;
      }

      .resize-strip {
        display: flex;
        flex-wrap: wrap;
        gap: var(--tp-space-2);
        align-items: center;
        padding: var(--tp-space-2);
        border: 1px solid color-mix(in srgb, var(--tp-color-border) 72%, transparent);
        border-radius: var(--tp-radius-sm);
        background: color-mix(in srgb, var(--tp-color-panel-raised) 42%, transparent);
      }

      .resize-strip__size {
        display: inline-flex;
        align-items: center;
        min-height: 1.9rem;
        border-radius: var(--tp-radius-sm);
        background: color-mix(in srgb, var(--tp-color-bg) 62%, transparent);
        color: var(--tp-color-text);
        font-family: var(--tp-font-family-mono);
        font-size: 0.82rem;
        padding: 0.2rem 0.55rem;
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

      .node-action {
        min-height: 1.8rem;
        border-radius: var(--tp-radius-sm);
        padding: 0.2rem 0.5rem;
        font-size: 0.78rem;
      }

      .danger {
        border-color: color-mix(in srgb, var(--tp-color-danger) 38%, var(--tp-color-border));
        color: color-mix(in srgb, var(--tp-color-danger) 82%, var(--tp-color-text));
      }

      .danger[data-confirming] {
        background: color-mix(in srgb, var(--tp-color-danger) 16%, transparent);
        color: var(--tp-color-text);
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

        .rename-form {
          grid-template-columns: 1fr;
        }
      }
    `,
  ];

  protected declare pendingAction: string | null;
  protected declare actionError: string | null;
  protected declare renamingTabId: string | null;
  protected declare renameDraft: string;
  protected declare armedCloseAction: CloseAction | null;

  constructor() {
    super();
    this.pendingAction = null;
    this.actionError = null;
    this.renamingTabId = null;
    this.renameDraft = "";
    this.armedCloseAction = null;
  }

  override render() {
    const topology = this.snapshot.attachedSession?.topology;
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    const focusedTab = controls.activeTab;
    const root = focusedTab?.root;
    const focusedPaneId = controls.activePaneId;
    const isPending = this.pendingAction !== null;
    const isRenamingFocusedTab = Boolean(focusedTab && this.renamingTabId === focusedTab.tab_id);
    const closeTabArmed = Boolean(focusedTab && this.isCloseArmed("tab", focusedTab.tab_id));

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
            data-testid="tp-split-right"
            data-split-direction="vertical"
            ?disabled=${isPending || !controls.canSplitPane}
            @click=${() => this.splitPane("vertical")}
          >
            Split right
          </button>
          <button
            type="button"
            data-testid="tp-split-down"
            data-split-direction="horizontal"
            ?disabled=${isPending || !controls.canSplitPane}
            @click=${() => this.splitPane("horizontal")}
          >
            Split down
          </button>
          <button
            type="button"
            data-testid="tp-rename-tab"
            ?disabled=${isPending || !controls.canRenameTab}
            @click=${() => this.toggleRenameTab()}
          >
            Rename tab
          </button>
          <button
            class="danger"
            type="button"
            data-testid="tp-close-tab"
            ?data-confirming=${closeTabArmed}
            ?disabled=${isPending || !controls.canCloseTab}
            @click=${() => this.closeTab()}
          >
            ${closeTabArmed ? "Confirm close tab" : "Close tab"}
          </button>
        </div>

        ${isRenamingFocusedTab
          ? html`
              <div class="rename-form" part="rename-form">
                <input
                  data-testid="tp-rename-tab-input"
                  name="tp-rename-tab-title"
                  aria-label="Tab title"
                  .value=${this.renameDraft}
                  ?disabled=${isPending}
                  @input=${this.handleRenameInput}
                  @keydown=${this.handleRenameKeydown}
                />
                <button
                  type="button"
                  data-testid="tp-rename-tab-save"
                  ?disabled=${isPending || !this.renameDraft.trim()}
                  @click=${() => this.renameTab()}
                >
                  Save
                </button>
                <button type="button" data-testid="tp-rename-tab-cancel" ?disabled=${isPending} @click=${() => this.cancelRename()}>
                  Cancel
                </button>
              </div>
            `
          : null}

        ${controls.activePaneSize
          ? html`
              <div class="resize-strip" part="resize-controls" aria-label="Focused pane size controls">
                <span class="resize-strip__size" part="pane-size" data-testid="tp-pane-size">
                  ${controls.activePaneSize.cols}x${controls.activePaneSize.rows}
                </span>
                <button
                  type="button"
                  data-testid="tp-resize-narrower"
                  ?disabled=${isPending || !controls.canResizePane || controls.activePaneSize.cols <= TERMINAL_PANE_MIN_COLS}
                  @click=${() => this.resizePane({ cols: -8 })}
                >
                  Narrower
                </button>
                <button
                  type="button"
                  data-testid="tp-resize-wider"
                  ?disabled=${isPending || !controls.canResizePane || controls.activePaneSize.cols >= TERMINAL_PANE_MAX_COLS}
                  @click=${() => this.resizePane({ cols: 8 })}
                >
                  Wider
                </button>
                <button
                  type="button"
                  data-testid="tp-resize-shorter"
                  ?disabled=${isPending || !controls.canResizePane || controls.activePaneSize.rows <= TERMINAL_PANE_MIN_ROWS}
                  @click=${() => this.resizePane({ rows: -4 })}
                >
                  Shorter
                </button>
                <button
                  type="button"
                  data-testid="tp-resize-taller"
                  ?disabled=${isPending || !controls.canResizePane || controls.activePaneSize.rows >= TERMINAL_PANE_MAX_ROWS}
                  @click=${() => this.resizePane({ rows: 4 })}
                >
                  Taller
                </button>
              </div>
            `
          : null}

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
                armedCloseAction: this.armedCloseAction,
                canClosePane: controls.canClosePane,
                canFocusPane: controls.canFocusPane,
                focusedPaneId,
                isPending,
                onClose: (paneId) => this.closePane(paneId),
                onFocus: (paneId) => this.focusPane(paneId),
              })
            : html`<div class="empty-state" part="empty">No pane tree yet</div>`}
        </div>
      </div>
    `;
  }

  private newTab(): void {
    const controls = resolveTerminalTopologyControlState(this.snapshot);
    this.clearTransientTopologyUi();
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

    this.clearTransientTopologyUi();
    this.runTopologyCommand(`split_${direction}`, {
      kind: "split_pane",
      pane_id: paneId,
      direction,
    });
  }

  private resizePane(delta: TerminalPaneResizeDelta): void {
    const command = resolvePaneResizeCommand(this.snapshot, delta);
    if (!command) {
      return;
    }

    this.clearTransientTopologyUi();
    this.runTopologyCommand("resize_pane", command);
  }

  private focusTab(tabId: string): void {
    if (this.snapshot.attachedSession?.topology.focused_tab === tabId) {
      return;
    }

    this.clearTransientTopologyUi();
    this.runTopologyCommand("focus_tab", { kind: "focus_tab", tab_id: tabId });
  }

  private focusPane(paneId: string): void {
    if (resolveTerminalTopologyControlState(this.snapshot).activePaneId === paneId) {
      return;
    }

    this.clearTransientTopologyUi();
    this.runTopologyCommand("focus_pane", { kind: "focus_pane", pane_id: paneId });
  }

  private toggleRenameTab(): void {
    const tab = resolveTerminalTopologyControlState(this.snapshot).activeTab;
    if (!tab) {
      return;
    }

    this.armedCloseAction = null;
    if (this.renamingTabId === tab.tab_id) {
      this.cancelRename();
      return;
    }

    this.renamingTabId = tab.tab_id;
    this.renameDraft = tab.title ?? compactTerminalId(tab.tab_id);
  }

  private handleRenameInput(event: Event): void {
    this.renameDraft = (event.currentTarget as HTMLInputElement).value;
  }

  private handleRenameKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      this.renameTab();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      this.cancelRename();
    }
  }

  private renameTab(): void {
    const tab = resolveTerminalTopologyControlState(this.snapshot).activeTab;
    const title = this.renameDraft.trim();
    if (!tab || !title) {
      return;
    }

    this.renamingTabId = null;
    this.renameDraft = "";
    this.armedCloseAction = null;
    this.runTopologyCommand("rename_tab", {
      kind: "rename_tab",
      tab_id: tab.tab_id,
      title,
    });
  }

  private cancelRename(): void {
    this.renamingTabId = null;
    this.renameDraft = "";
  }

  private closePane(paneId: string): void {
    if (!this.isCloseArmed("pane", paneId)) {
      this.renamingTabId = null;
      this.renameDraft = "";
      this.armedCloseAction = { kind: "pane", id: paneId };
      return;
    }

    this.armedCloseAction = null;
    this.runTopologyCommand("close_pane", { kind: "close_pane", pane_id: paneId });
  }

  private closeTab(): void {
    const tab = resolveTerminalTopologyControlState(this.snapshot).activeTab;
    if (!tab) {
      return;
    }

    if (!this.isCloseArmed("tab", tab.tab_id)) {
      this.renamingTabId = null;
      this.renameDraft = "";
      this.armedCloseAction = { kind: "tab", id: tab.tab_id };
      return;
    }

    this.armedCloseAction = null;
    this.runTopologyCommand("close_tab", { kind: "close_tab", tab_id: tab.tab_id });
  }

  private isCloseArmed(kind: CloseAction["kind"], id: string): boolean {
    return this.armedCloseAction?.kind === kind && this.armedCloseAction.id === id;
  }

  private clearTransientTopologyUi(): void {
    this.renamingTabId = null;
    this.renameDraft = "";
    this.armedCloseAction = null;
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

interface CloseAction {
  kind: "pane" | "tab";
  id: string;
}

interface RenderNodeOptions {
  armedCloseAction: CloseAction | null;
  canClosePane: boolean;
  canFocusPane: boolean;
  focusedPaneId: string | null;
  isPending: boolean;
  onClose(paneId: string): void;
  onFocus(paneId: string): void;
}

function renderNode(node: PaneTreeNode, options: RenderNodeOptions): TemplateResult {
  if (node.kind === "leaf") {
    const focused = node.pane_id === options.focusedPaneId;
    const closeArmed = options.armedCloseAction?.kind === "pane" && options.armedCloseAction.id === node.pane_id;
    return html`
      <span class="pane-row" part="pane-row">
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
        <button
          class="node-action danger"
          type="button"
          part="pane-close"
          data-testid="tp-close-pane"
          data-pane-id=${node.pane_id}
          ?data-confirming=${closeArmed}
          ?disabled=${options.isPending || !options.canClosePane}
          @click=${() => options.onClose(node.pane_id)}
        >
          ${closeArmed ? "Confirm close" : "Close"}
        </button>
      </span>
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
