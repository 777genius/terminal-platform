import { css, html } from "lit";
import type { TemplateResult } from "lit";

import type { PaneTreeNode } from "@terminal-platform/runtime-types";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";

export class TerminalPaneTreeElement extends WorkspaceKernelConsumerElement {
  static styles = [
    terminalElementStyles,
    css`
      .tree {
        padding: var(--tp-space-3);
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
        min-height: 1.8rem;
        border: 1px solid var(--tp-color-border);
        border-radius: 999px;
        padding: 0.2rem 0.55rem;
        background: color-mix(in srgb, var(--tp-color-panel-raised) 72%, transparent);
        color: var(--tp-color-text-muted);
        font-size: 0.84rem;
      }

      .node[data-focused="true"] {
        border-color: color-mix(in srgb, var(--tp-color-accent) 46%, transparent);
        background: var(--tp-color-accent-soft);
        color: var(--tp-color-text);
      }
    `,
  ];

  override render() {
    const topology = this.snapshot.attachedSession?.topology;
    const focusedTab = topology?.tabs.find((tab) => tab.tab_id === topology.focused_tab) ?? topology?.tabs[0];
    const root = focusedTab?.root;
    const focusedPaneId = focusedTab?.focused_pane ?? this.snapshot.selection.activePaneId;

    return html`
      <div class="panel tree" part="tree">
        <div class="panel-header">
          <div class="panel-eyebrow">Layout</div>
          <div class="panel-title">Pane structure</div>
          <div class="panel-copy">${focusedTab?.title ?? "Focused tab"} pane routing map.</div>
        </div>
        ${root ? renderNode(root, focusedPaneId) : html`<div class="empty-state" part="empty">No pane tree yet</div>`}
      </div>
    `;
  }
}

function renderNode(node: PaneTreeNode, focusedPaneId: string | null): TemplateResult {
  if (node.kind === "leaf") {
    return html`
      <div class="node" part="leaf" data-focused=${String(node.pane_id === focusedPaneId)}>
        Pane ${node.pane_id}
      </div>
    `;
  }

  return html`
    <div class="node" part="split">Split ${node.direction}</div>
    <ul part="children">
      <li>${renderNode(node.first, focusedPaneId)}</li>
      <li>${renderNode(node.second, focusedPaneId)}</li>
    </ul>
  `;
}
