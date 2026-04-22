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
        padding-left: 1rem;
      }
    `,
  ];

  override render() {
    const root = this.snapshot.attachedSession?.topology.tabs[0]?.root;

    return html`
      <div class="panel tree" part="tree">
        <div class="panel-header">
          <div class="panel-eyebrow">Layout</div>
          <div class="panel-title">Pane structure</div>
          <div class="panel-copy">See how the current session is split before sending input to a pane.</div>
        </div>
        ${root ? renderNode(root) : html`<div class="muted" part="empty">No pane tree yet</div>`}
      </div>
    `;
  }
}

function renderNode(node: PaneTreeNode): TemplateResult {
  if (node.kind === "leaf") {
    return html`<div part="leaf">Pane ${node.pane_id}</div>`;
  }

  return html`
    <div part="split">Split ${node.direction}</div>
    <ul part="children">
      <li>${renderNode(node.first)}</li>
      <li>${renderNode(node.second)}</li>
    </ul>
  `;
}
