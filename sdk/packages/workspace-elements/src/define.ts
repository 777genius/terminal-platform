import { TerminalPaneTreeElement } from "./elements/terminal-pane-tree-element.js";
import { TerminalSavedSessionsElement } from "./elements/terminal-saved-sessions-element.js";
import { TerminalScreenElement } from "./elements/terminal-screen-element.js";
import { TerminalSessionListElement } from "./elements/terminal-session-list-element.js";
import { TerminalToolbarElement } from "./elements/terminal-toolbar-element.js";
import { TerminalWorkspaceElement } from "./elements/terminal-workspace-element.js";

export function defineTerminalPlatformElements(registry: CustomElementRegistry = customElements): void {
  defineIfNeeded(registry, "tp-terminal-workspace", TerminalWorkspaceElement);
  defineIfNeeded(registry, "tp-terminal-session-list", TerminalSessionListElement);
  defineIfNeeded(registry, "tp-terminal-toolbar", TerminalToolbarElement);
  defineIfNeeded(registry, "tp-terminal-screen", TerminalScreenElement);
  defineIfNeeded(registry, "tp-terminal-pane-tree", TerminalPaneTreeElement);
  defineIfNeeded(registry, "tp-terminal-saved-sessions", TerminalSavedSessionsElement);
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
