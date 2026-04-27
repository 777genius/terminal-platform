import { css, html, type TemplateResult } from "lit";

import { WorkspaceKernelConsumerElement } from "../context/workspace-kernel-consumer-element.js";
import { terminalElementStyles } from "../styles/terminal-element-styles.js";
import {
  defaultTerminalCommandQuickCommands,
  type TerminalCommandQuickCommand,
} from "./terminal-command-quick-commands.js";
import {
  resolveTerminalWorkspaceLayoutState,
  TERMINAL_WORKSPACE_LAYOUT_PRESETS,
  TERMINAL_WORKSPACE_INSPECTOR_MODES,
  TERMINAL_WORKSPACE_NAVIGATION_MODES,
  type TerminalWorkspaceLayoutPreset,
  type TerminalWorkspaceInspectorMode,
  type TerminalWorkspaceNavigationMode,
} from "./terminal-workspace-layout.js";

export class TerminalWorkspaceElement extends WorkspaceKernelConsumerElement {
  static override properties = {
    ...WorkspaceKernelConsumerElement.properties,
    quickCommands: { attribute: false },
    autoFocusCommandInput: { attribute: "auto-focus-command-input", type: Boolean },
    layoutPreset: { attribute: "layout-preset", type: String },
    inspectorMode: { attribute: "inspector-mode", type: String },
    navigationMode: { attribute: "navigation-mode", type: String },
  };

  static styles = [
    terminalElementStyles,
    css`
      .workspace {
        --tp-workspace-sidebar-default-width: clamp(14rem, 21vw, 19rem);
        --tp-workspace-inspector-default-width: clamp(16rem, 24vw, 24rem);
        --tp-workspace-gap: var(--tp-space-3);
        --tp-workspace-terminal-column-min-height: clamp(30rem, 68vh, 48rem);
        --tp-shadow-panel: var(--tp-workspace-panel-shadow, none);
        --tp-workspace-sidebar-width: var(
          --tp-workspace-sidebar-target-width,
          var(--tp-workspace-sidebar-default-width)
        );
        --tp-workspace-inspector-width: var(
          --tp-workspace-inspector-target-width,
          var(--tp-workspace-inspector-default-width)
        );

        display: grid;
        grid-template-rows: auto minmax(0, 1fr);
        gap: var(--tp-workspace-gap);
        height: 100%;
        min-height: 0;
      }

      .workspace[data-chrome-tone="terminal"] {
        --tp-workspace-gap: 0.55rem;
      }

      .workspace[data-chrome-tone="terminal"] tp-terminal-status-bar {
        --tp-color-bg: var(--tp-terminal-color-bg);
        --tp-color-bg-inset: var(--tp-terminal-color-bg);
        --tp-color-panel: var(--tp-terminal-color-bg-raised);
        --tp-color-panel-raised: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 84%, white 4%);
        --tp-color-border: color-mix(in srgb, var(--tp-terminal-color-border) 82%, transparent);
        --tp-color-text: var(--tp-terminal-color-text);
        --tp-color-text-muted: var(--tp-terminal-color-text-muted);
        --tp-shadow-panel: none;
      }

      .body {
        display: grid;
        grid-template-columns: minmax(14rem, var(--tp-workspace-sidebar-width)) minmax(0, 1fr);
        gap: var(--tp-workspace-gap);
        height: 100%;
        min-height: 0;
        align-items: stretch;
      }

      .body[data-navigation-mode="collapsed"],
      .body[data-navigation-mode="hidden"] {
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: minmax(0, 1fr) auto;
      }

      .sidebar,
      .content {
        display: grid;
        gap: var(--tp-space-3);
        min-height: 0;
        align-content: start;
        min-width: 0;
      }

      .sidebar {
        position: sticky;
        top: 0;
      }

      .content {
        container-type: inline-size;
        height: 100%;
      }

      .body[data-navigation-mode="collapsed"] .content,
      .body[data-navigation-mode="hidden"] .content {
        align-content: stretch;
      }

      .operations-deck {
        display: grid;
        grid-template-columns: minmax(0, 1fr) minmax(16rem, var(--tp-workspace-inspector-width));
        gap: var(--tp-space-3);
        align-items: stretch;
        height: 100%;
        min-height: 0;
      }

      .operations-deck[data-inspector-mode="collapsed"],
      .operations-deck[data-inspector-mode="hidden"] {
        grid-template-columns: minmax(0, 1fr);
        grid-template-rows: minmax(0, 1fr) auto;
      }

      .terminal-column,
      .inspector-column {
        display: grid;
        gap: var(--tp-space-3);
        height: 100%;
        min-width: 0;
        min-height: 0;
      }

      .terminal-column {
        grid-template-rows: auto minmax(0, 1fr) auto;
        align-content: stretch;
        gap: 0;
        min-height: var(--tp-workspace-terminal-column-min-height);
        overflow: hidden;
        --tp-terminal-screen-panel-border-top-left-radius: 0;
        --tp-terminal-screen-panel-border-top-right-radius: 0;
        --tp-terminal-screen-panel-border-bottom-left-radius: 0;
        --tp-terminal-screen-panel-border-bottom-right-radius: 0;
        --tp-terminal-screen-panel-padding-bottom: 0;
        --tp-terminal-screen-panel-shadow: none;
        --tp-terminal-screen-viewport-border-bottom-left-radius: 0;
        --tp-terminal-screen-viewport-border-bottom-right-radius: 0;
      }

      .inspector-column {
        position: sticky;
        top: 0;
        align-content: start;
        max-height: calc(100vh - var(--tp-space-4));
        overflow: auto;
        padding-right: 0.15rem;
        scrollbar-gutter: stable;
      }

      .inspector-drawer {
        min-width: 0;
      }

      .navigation-drawer {
        min-width: 0;
      }

      .navigation-drawer__content {
        display: grid;
        gap: var(--tp-space-3);
        max-height: min(28rem, 42vh);
        min-width: 0;
        overflow: auto;
        padding: var(--tp-space-3);
        scrollbar-gutter: stable;
      }

      .navigation-drawer__content .sidebar {
        grid-template-columns: repeat(auto-fit, minmax(min(100%, 14rem), 1fr));
        position: static;
      }

      .inspector-drawer summary,
      .navigation-drawer summary {
        align-items: center;
        display: flex;
        justify-content: space-between;
      }

      .inspector-drawer summary::after,
      .navigation-drawer summary::after {
        color: var(--tp-color-text-muted);
        content: "Open";
        font-size: 0.76rem;
        font-weight: 500;
      }

      .secondary-toggle[data-secondary-chrome="terminal"] summary::after {
        border: 1px solid color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.42rem;
        background: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, transparent);
        color: var(--tp-terminal-color-text-muted);
        padding: 0.18rem 0.44rem;
      }

      .inspector-drawer[open] summary::after,
      .navigation-drawer[open] summary::after {
        content: "Close";
      }

      .inspector-drawer__content {
        display: grid;
        gap: var(--tp-space-3);
        max-height: min(34rem, 46vh);
        min-width: 0;
        overflow: auto;
        padding: var(--tp-space-3);
        scrollbar-gutter: stable;
      }

      .inspector-drawer__content .inspector-column {
        position: static;
        max-height: none;
        overflow: visible;
        padding-right: 0;
      }

      .command-region {
        min-width: 0;
      }

      .command-region tp-terminal-command-dock {
        display: block;
      }

      tp-terminal-screen,
      tp-terminal-tab-strip,
      tp-terminal-pane-tree,
      tp-terminal-command-dock,
      tp-terminal-toolbar,
      tp-terminal-session-list,
      tp-terminal-saved-sessions {
        display: block;
        min-width: 0;
      }

      .terminal-column tp-terminal-screen {
        height: 100%;
        min-height: 0;
      }

      .diagnostics {
        padding: var(--tp-space-3);
      }

      .advanced-stack {
        display: grid;
        gap: var(--tp-space-3);
      }

      .secondary-toggle {
        border: 1px solid var(--tp-color-border);
        border-radius: var(--tp-radius-md);
        background: var(--tp-color-panel);
        overflow: hidden;
      }

      .secondary-toggle[data-secondary-chrome="terminal"] {
        --tp-color-bg: var(--tp-terminal-color-bg);
        --tp-color-bg-inset: var(--tp-terminal-color-bg);
        --tp-color-panel: var(--tp-terminal-color-bg-raised);
        --tp-color-panel-raised: color-mix(in srgb, var(--tp-terminal-color-bg-raised) 82%, white 4%);
        --tp-color-border: color-mix(in srgb, var(--tp-terminal-color-border) 82%, transparent);
        --tp-color-text: var(--tp-terminal-color-text);
        --tp-color-text-muted: var(--tp-terminal-color-text-muted);
        --tp-shadow-panel: none;

        border-color: color-mix(in srgb, var(--tp-terminal-color-border) 78%, transparent);
        border-radius: 0.5rem;
        background:
          linear-gradient(
            180deg,
            color-mix(in srgb, var(--tp-terminal-color-bg-raised) 86%, transparent),
            var(--tp-terminal-color-bg)
          ),
          var(--tp-terminal-color-bg);
        color: var(--tp-terminal-color-text);
      }

      .secondary-toggle summary {
        cursor: pointer;
        list-style: none;
        padding: var(--tp-space-3);
        font-weight: 600;
      }

      .secondary-toggle[data-secondary-chrome="terminal"] summary {
        min-height: 2.18rem;
        align-items: center;
        color: var(--tp-terminal-color-text);
        padding: 0.5rem 0.7rem;
      }

      .secondary-toggle summary::-webkit-details-marker {
        display: none;
      }

      .secondary-toggle[open] summary {
        border-bottom: 1px solid var(--tp-color-border);
      }

      .secondary-toggle[data-secondary-chrome="terminal"][open] summary {
        border-bottom-color: color-mix(in srgb, var(--tp-terminal-color-border) 74%, transparent);
      }

      .secondary-toggle .advanced-stack {
        padding: var(--tp-space-3);
      }

      .secondary-toggle[data-secondary-chrome="terminal"] .advanced-stack,
      .secondary-toggle[data-secondary-chrome="terminal"] .navigation-drawer__content,
      .secondary-toggle[data-secondary-chrome="terminal"] .inspector-drawer__content {
        padding: 0.62rem;
      }

      .workspace-tools {
        display: grid;
        gap: var(--tp-space-3);
      }

      @media (max-width: 1180px) {
        .operations-deck {
          grid-template-columns: 1fr;
        }

        .inspector-column {
          position: static;
          max-height: none;
          overflow: visible;
          padding-right: 0;
        }
      }

      @container (max-width: 50rem) {
        .operations-deck {
          grid-template-columns: 1fr;
        }

        .inspector-column {
          position: static;
          max-height: none;
          overflow: visible;
          padding-right: 0;
        }
      }

      @media (max-width: 900px) {
        .body {
          grid-template-columns: 1fr;
        }

        .content {
          order: 1;
        }

        .sidebar {
          order: 2;
          position: static;
        }
      }
    `,
  ];

  declare quickCommands: readonly TerminalCommandQuickCommand[] | null | undefined;
  declare autoFocusCommandInput: boolean;
  declare layoutPreset: TerminalWorkspaceLayoutPreset;
  declare inspectorMode: TerminalWorkspaceInspectorMode;
  declare navigationMode: TerminalWorkspaceNavigationMode;

  constructor() {
    super();
    this.quickCommands = defaultTerminalCommandQuickCommands;
    this.autoFocusCommandInput = false;
    this.layoutPreset = TERMINAL_WORKSPACE_LAYOUT_PRESETS.classic;
    this.inspectorMode = TERMINAL_WORKSPACE_INSPECTOR_MODES.inline;
    this.navigationMode = TERMINAL_WORKSPACE_NAVIGATION_MODES.inline;
  }

  override render() {
    const layoutState = resolveTerminalWorkspaceLayoutState({
      layoutPreset: this.layoutPreset,
      inspectorMode: this.inspectorMode,
      navigationMode: this.navigationMode,
    });
    const inspectorState = layoutState.inspector;
    const navigationState = layoutState.navigation;
    const chromeState = layoutState.chrome;

    return html`
      <div class="workspace" part="workspace" data-chrome-tone=${chromeState.tone}>
        <tp-terminal-status-bar .kernel=${this.kernel}></tp-terminal-status-bar>

        <div
          class="body"
          part="body"
          data-testid="tp-workspace-layout"
          data-layout="operations-deck"
          data-layout-preset=${layoutState.preset}
          data-navigation-mode=${navigationState.mode}
          data-secondary-chrome=${chromeState.secondaryChrome}
        >
          ${navigationState.renderInlineNavigation
            ? html`
                <div class="sidebar" part="sidebar" data-testid="tp-workspace-sidebar">
                  ${this.renderNavigationContent()}
                </div>
              `
            : null}
          <div class="content" part="content">
            <div
              class="operations-deck"
              part="operations-deck"
              data-testid="tp-workspace-operations-deck"
              data-inspector-mode=${inspectorState.mode}
            >
              <div class="terminal-column" part="terminal-column" data-testid="tp-workspace-terminal-column">
                <tp-terminal-tab-strip .kernel=${this.kernel}></tp-terminal-tab-strip>
                <tp-terminal-screen .kernel=${this.kernel} placement="terminal"></tp-terminal-screen>
                <div class="command-region" part="command-region" data-testid="tp-workspace-command-region">
                  <tp-terminal-command-dock
                    .kernel=${this.kernel}
                    .quickCommands=${this.quickCommands}
                    .autoFocusInput=${this.autoFocusCommandInput}
                    placement="terminal"
                  ></tp-terminal-command-dock>
                </div>
              </div>

              ${inspectorState.renderInlineInspector
                ? html`
                    <div class="inspector-column" part="inspector-column" data-testid="tp-workspace-inspector-column">
                      ${this.renderInspectorContent()}
                    </div>
                  `
                : null}

              ${inspectorState.renderCollapsedInspector
                ? html`
                    <details
                      class="secondary-toggle inspector-drawer"
                      part="inspector-drawer"
                      data-testid="tp-workspace-inspector-drawer"
                      data-secondary-chrome=${chromeState.secondaryChrome}
                    >
                      <summary>${inspectorState.summaryLabel}</summary>
                      <div class="inspector-drawer__content">
                        <div class="inspector-column" part="inspector-column" data-testid="tp-workspace-inspector-column">
                          ${this.renderInspectorContent()}
                        </div>
                      </div>
                    </details>
                  `
                : null}
            </div>
          </div>

          ${navigationState.renderCollapsedNavigation
            ? html`
                <details
                  class="secondary-toggle navigation-drawer"
                  part="navigation-drawer"
                  data-testid="tp-workspace-navigation-drawer"
                  data-secondary-chrome=${chromeState.secondaryChrome}
                >
                  <summary>${navigationState.summaryLabel}</summary>
                  <div class="navigation-drawer__content">
                    <div class="sidebar" part="sidebar" data-testid="tp-workspace-sidebar">
                      ${this.renderNavigationContent()}
                    </div>
                  </div>
                </details>
              `
            : null}
        </div>
      </div>
    `;
  }

  private renderNavigationContent(): TemplateResult {
    return html`
      <tp-terminal-session-list .kernel=${this.kernel}></tp-terminal-session-list>
      <tp-terminal-saved-sessions .kernel=${this.kernel}></tp-terminal-saved-sessions>
    `;
  }

  private renderInspectorContent(): TemplateResult {
    return html`
      <tp-terminal-pane-tree .kernel=${this.kernel}></tp-terminal-pane-tree>

      <details class="secondary-toggle workspace-tools">
        <summary>Workspace tools</summary>
        <div class="advanced-stack">
          <tp-terminal-toolbar .kernel=${this.kernel}></tp-terminal-toolbar>
        </div>
      </details>

      <div class="advanced-stack" part="diagnostics-stack">
        ${this.renderDiagnostics()}
      </div>
    `;
  }

  private renderDiagnostics(): TemplateResult | null {
    return this.snapshot.diagnostics.length > 0
      ? html`
          <details class="secondary-toggle" open>
            <summary>Workspace notices - ${this.snapshot.diagnostics.length}</summary>
            <div class="panel diagnostics" part="diagnostics">
              <div class="panel-header">
                <div class="panel-eyebrow">Alerts</div>
                <div class="panel-title">Workspace diagnostics</div>
                <div class="panel-copy">These notices come from the transport or runtime layer.</div>
              </div>
              <ul>
                ${this.snapshot.diagnostics.map(
                  (item) => html`
                    <li>
                      <strong>${item.code}</strong>
                      <span class="muted"> ${item.message}</span>
                    </li>
                  `,
                )}
              </ul>
            </div>
          </details>
        `
      : null;
  }
}
