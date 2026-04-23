import { TerminalPaneTreeElement } from "./elements/terminal-pane-tree-element.js";
import { TerminalSavedSessionsElement } from "./elements/terminal-saved-sessions-element.js";
import { TerminalScreenElement } from "./elements/terminal-screen-element.js";
import { TerminalSessionListElement } from "./elements/terminal-session-list-element.js";
import { TerminalStatusBarElement } from "./elements/terminal-status-bar-element.js";
import { TerminalToolbarElement } from "./elements/terminal-toolbar-element.js";
import { TerminalWorkspaceElement } from "./elements/terminal-workspace-element.js";

export function defineTerminalPlatformElements(registry?: CustomElementRegistry): void {
  const resolvedRegistry = registry ?? globalThis.customElements;
  if (!resolvedRegistry) {
    return;
  }

  defineIfNeeded(resolvedRegistry, "tp-terminal-workspace", TerminalWorkspaceElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-session-list", TerminalSessionListElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-status-bar", TerminalStatusBarElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-toolbar", TerminalToolbarElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-screen", TerminalScreenElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-pane-tree", TerminalPaneTreeElement);
  defineIfNeeded(resolvedRegistry, "tp-terminal-saved-sessions", TerminalSavedSessionsElement);
}

function defineIfNeeded(
  registry: CustomElementRegistry,
  tagName: string,
  ctor: CustomElementConstructor,
): void {
  if (!registry.get(tagName)) {
    registry.define(tagName, ctor);
  }
}
