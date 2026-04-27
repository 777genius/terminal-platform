import type * as React from "react";
import { describe, expect, it } from "vitest";

import type {
  TerminalCommandComposer,
  TerminalWorkspace,
  TerminalCommandComposerActionId,
  TerminalCommandComposerActionOptions,
  TerminalCommandComposerActionPresentation,
  TerminalCommandComposerActionTone,
  TerminalCommandComposerDraftChangeDetail,
  TerminalCommandComposerHistoryNavigateDetail,
  TerminalCommandComposerShortcutDetail,
  TerminalCommandDockAccessoryMode,
  TerminalCommandDockAccessoryOptions,
  TerminalScreenActionTone,
  TerminalScreenChromeMode,
  TerminalScreenChromeState,
  TerminalWorkspaceChromeState,
  TerminalWorkspaceChromeTone,
  TerminalWorkspaceInspectorMode,
  TerminalWorkspaceInspectorState,
  TerminalWorkspaceLayoutPreset,
  TerminalWorkspaceLayoutState,
  TerminalWorkspaceNavigationMode,
  TerminalWorkspaceNavigationState,
  TerminalWorkspaceSecondaryChromeMode,
} from "./index.js";
import type { TerminalCommandComposerElement } from "@terminal-platform/workspace-elements";

type Assert<T extends true> = T;

type Equal<Actual, Expected> = (<T>() => T extends Actual ? 1 : 2) extends
  <T>() => T extends Expected ? 1 : 2
  ? true
  : false;

type EventParameter<Handler> = NonNullable<Handler> extends (event: infer Event) => void ? Event : never;

type ComposerProps = React.ComponentProps<typeof TerminalCommandComposer>;

type _ComposerRefTargetsElement = Assert<
  Equal<React.ComponentRef<typeof TerminalCommandComposer>, TerminalCommandComposerElement>
>;
type _ComposerDraftProp = Assert<Equal<ComposerProps["draft"], string | undefined>>;
type _ComposerMinRowsProp = Assert<Equal<ComposerProps["minRows"], number | undefined>>;
type _ComposerMaxRowsProp = Assert<Equal<ComposerProps["maxRows"], number | undefined>>;
type _ComposerDraftChangeEvent = Assert<
  Equal<EventParameter<ComposerProps["onCommandDraftChange"]>, CustomEvent<TerminalCommandComposerDraftChangeDetail>>
>;
type _ComposerHistoryNavigateEvent = Assert<
  Equal<
    EventParameter<ComposerProps["onCommandHistoryNavigate"]>,
    CustomEvent<TerminalCommandComposerHistoryNavigateDetail>
  >
>;
type _ComposerShortcutEvent = Assert<
  Equal<EventParameter<ComposerProps["onCommandShortcut"]>, CustomEvent<TerminalCommandComposerShortcutDetail>>
>;
type _ComposerPasteEvent = Assert<Equal<EventParameter<ComposerProps["onCommandPaste"]>, CustomEvent<void>>>;
type _ComposerSubmitEvent = Assert<Equal<EventParameter<ComposerProps["onCommandSubmit"]>, CustomEvent<void>>>;
type WorkspaceProps = React.ComponentProps<typeof TerminalWorkspace>;
type _WorkspacePropsRemainImportable = WorkspaceProps;
type _WorkspaceInspectorModeProp = Assert<
  Equal<WorkspaceProps["inspectorMode"], TerminalWorkspaceInspectorMode | undefined>
>;
type _WorkspaceNavigationModeProp = Assert<
  Equal<WorkspaceProps["navigationMode"], TerminalWorkspaceNavigationMode | undefined>
>;
type _WorkspaceLayoutPresetProp = Assert<
  Equal<WorkspaceProps["layoutPreset"], TerminalWorkspaceLayoutPreset | undefined>
>;
type _ComposerActionContractTypesRemainImportable =
  | TerminalCommandComposerActionId
  | TerminalCommandComposerActionOptions
  | TerminalCommandComposerActionPresentation
  | TerminalCommandComposerActionTone
  | TerminalCommandDockAccessoryMode
  | TerminalCommandDockAccessoryOptions
  | TerminalScreenActionTone
  | TerminalScreenChromeMode
  | TerminalScreenChromeState
  | TerminalWorkspaceChromeState
  | TerminalWorkspaceChromeTone
  | TerminalWorkspaceInspectorMode
  | TerminalWorkspaceInspectorState
  | TerminalWorkspaceLayoutPreset
  | TerminalWorkspaceLayoutState
  | TerminalWorkspaceNavigationMode
  | TerminalWorkspaceNavigationState
  | TerminalWorkspaceSecondaryChromeMode;

describe("workspace react public api", () => {
  it("exports the command composer wrapper and composer utilities", async () => {
    installCustomElementRuntimeShim();

    const workspaceReact = await import("./index.js");

    expect(workspaceReact.TerminalCommandComposer.displayName).toBe("TerminalCommandComposer");
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit).toBe("submit");
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.id).join("|")).toBe(
      "submit|paste|interrupt|enter",
    );
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.tone).join("|")).toBe(
      "primary|secondary|secondary|secondary",
    );
    expect(workspaceReact.resolveTerminalCommandComposerActions()[0]?.keyHint).toBe("Enter");
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_EVENTS.submit).toBe("tp-terminal-command-submit");
    expect(workspaceReact.TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar).toBe("bar");
    expect(workspaceReact.resolveTerminalCommandDockAccessoryMode({ placement: "terminal" })).toBe("bar");
    expect(workspaceReact.TERMINAL_SCREEN_CHROME_MODES.compact).toBe("compact");
    expect(typeof workspaceReact.resolveTerminalScreenChromeState).toBe("function");
    expect(workspaceReact.resolveTerminalCommandComposerRows("echo one\necho two")).toBe(2);
    expect(workspaceReact.TERMINAL_WORKSPACE_CHROME_TONES.terminal).toBe("terminal");
    expect(workspaceReact.TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed).toBe("collapsed");
    expect(workspaceReact.TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal).toBe("terminal");
    expect(workspaceReact.TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed).toBe("collapsed");
    expect(workspaceReact.TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES.terminal).toBe("terminal");
    expect(workspaceReact.resolveTerminalWorkspaceChromeState("terminal")).toMatchObject({
      tone: "terminal",
      secondaryChrome: "terminal",
    });
    expect(workspaceReact.resolveTerminalWorkspaceLayoutState({ layoutPreset: "terminal" }).navigation.mode).toBe(
      "collapsed",
    );
    expect(workspaceReact.resolveTerminalWorkspaceInspectorState("hidden").renderInspector).toBe(false);
    expect(workspaceReact.resolveTerminalWorkspaceNavigationState("hidden").renderNavigation).toBe(false);
  });
});

function assertComposerActionContractTypesAreImportable(_value: _ComposerActionContractTypesRemainImportable): void {}

assertComposerActionContractTypesAreImportable(null as never);

function installCustomElementRuntimeShim(): void {
  if (!("HTMLElement" in globalThis)) {
    Object.defineProperty(globalThis, "HTMLElement", {
      configurable: true,
      value: class HTMLElement {},
    });
  }

  if (!("CustomEvent" in globalThis)) {
    Object.defineProperty(globalThis, "CustomEvent", {
      configurable: true,
      value: class CustomEvent<T = unknown> extends Event {
        detail: T;

        constructor(type: string, init: CustomEventInit<T> = {}) {
          super(type, init);
          this.detail = init.detail as T;
        }
      },
    });
  }

  if (!("customElements" in globalThis)) {
    const registry = new Map<string, CustomElementConstructor>();

    Object.defineProperty(globalThis, "customElements", {
      configurable: true,
      value: {
        define(tagName: string, constructor: CustomElementConstructor) {
          registry.set(tagName, constructor);
        },
        get(tagName: string) {
          return registry.get(tagName);
        },
      },
    });
  }
}
