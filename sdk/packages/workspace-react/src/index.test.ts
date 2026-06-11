import type * as React from "react";
import { describe, expect, it } from "vitest";

import type {
  TerminalCommandComposer,
  TerminalScreen,
  TerminalWorkspace,
  TerminalCommandComposerActionId,
  TerminalCommandComposerActionLabelMode,
  TerminalCommandComposerActionOptions,
  TerminalCommandComposerActionPresentation,
  TerminalCommandComposerActionTone,
  TerminalCommandComposerDraftChangeDetail,
  TerminalCommandComposerHistoryNavigateDetail,
  TerminalCommandComposerReactEventHandlers,
  TerminalCommandComposerShortcutDetail,
  TerminalCommandDockAccessoryMode,
  TerminalCommandDockAccessoryOptions,
  TerminalCommandDockAccessoryState,
  TerminalCommandDockAccessoryStateOptions,
  TerminalCommandDockSessionActionId,
  TerminalCommandDockSessionActionLabelMode,
  TerminalCommandDockSessionActionOptions,
  TerminalCommandDockSessionActionPlacement,
  TerminalCommandDockSessionActionPresentation,
  TerminalCommandDockSessionActionTone,
  TerminalScreenActionId,
  TerminalScreenActionLabelMode,
  TerminalScreenActionOptions,
  TerminalScreenActionPlacement,
  TerminalScreenActionPresentation,
  TerminalScreenActionTone,
  TerminalScreenCopiedDetail,
  TerminalScreenCopyFailedDetail,
  TerminalScreenSearchActionId,
  TerminalScreenSearchActionLabelMode,
  TerminalScreenSearchActionOptions,
  TerminalScreenSearchActionPlacement,
  TerminalScreenSearchActionPresentation,
  TerminalScreenSearchActionTone,
  TerminalScreenChromeMode,
  TerminalScreenChromeState,
  TerminalScreenInputFailedDetail,
  TerminalScreenInputSubmittedDetail,
  TerminalScreenPasteFailedDetail,
  TerminalScreenPasteSubmittedDetail,
  TerminalScreenReactEventHandlers,
  TerminalWorkspaceChromeState,
  TerminalWorkspaceChromeTone,
  TerminalWorkspaceInspectorMode,
  TerminalWorkspaceInspectorState,
  TerminalWorkspaceLayoutPreset,
  TerminalWorkspaceLayoutState,
  TerminalWorkspaceNavigationMode,
  TerminalWorkspaceNavigationState,
  TerminalWorkspacePartName,
  TerminalWorkspaceSecondaryChromeMode,
  TerminalWorkspaceSlotName,
} from "./index.js";
import type { TerminalCommandComposerElement, TerminalScreenElement } from "@terminal-platform/workspace-elements";

type Assert<T extends true> = T;

type Equal<Actual, Expected> = (<T>() => T extends Actual ? 1 : 2) extends
  <T>() => T extends Expected ? 1 : 2
  ? true
  : false;

type EventParameter<Handler> = NonNullable<Handler> extends (event: infer Event) => void ? Event : never;

type ComposerProps = React.ComponentProps<typeof TerminalCommandComposer>;
type ScreenProps = React.ComponentProps<typeof TerminalScreen>;

type _ComposerRefTargetsElement = Assert<
  Equal<React.ComponentRef<typeof TerminalCommandComposer>, TerminalCommandComposerElement>
>;
type _ComposerDraftProp = Assert<Equal<ComposerProps["draft"], string | undefined>>;
type _ComposerMinRowsProp = Assert<Equal<ComposerProps["minRows"], number | undefined>>;
type _ComposerMaxRowsProp = Assert<Equal<ComposerProps["maxRows"], number | undefined>>;
type _ComposerInputDescriptionIdProp = Assert<Equal<ComposerProps["inputDescriptionId"], string | undefined>>;
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
type _ScreenRefTargetsElement = Assert<
  Equal<React.ComponentRef<typeof TerminalScreen>, TerminalScreenElement>
>;
type _ScreenCopiedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenCopied"]>, CustomEvent<TerminalScreenCopiedDetail>>
>;
type _ScreenCopyFailedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenCopyFailed"]>, CustomEvent<TerminalScreenCopyFailedDetail>>
>;
type _ScreenInputSubmittedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenInputSubmitted"]>, CustomEvent<TerminalScreenInputSubmittedDetail>>
>;
type _ScreenInputFailedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenInputFailed"]>, CustomEvent<TerminalScreenInputFailedDetail>>
>;
type _ScreenPasteSubmittedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenPasteSubmitted"]>, CustomEvent<TerminalScreenPasteSubmittedDetail>>
>;
type _ScreenPasteFailedEvent = Assert<
  Equal<EventParameter<ScreenProps["onScreenPasteFailed"]>, CustomEvent<TerminalScreenPasteFailedDetail>>
>;
type _ComposerReactHandlerType = Assert<
  Equal<
    NonNullable<TerminalCommandComposerReactEventHandlers["onCommandDraftChange"]>,
    (event: CustomEvent<TerminalCommandComposerDraftChangeDetail>) => void
  >
>;
type _ScreenReactHandlerType = Assert<
  Equal<
    NonNullable<TerminalScreenReactEventHandlers["onScreenInputSubmitted"]>,
    (event: CustomEvent<TerminalScreenInputSubmittedDetail>) => void
  >
>;
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
  | TerminalCommandComposerActionLabelMode
  | TerminalCommandComposerActionOptions
  | TerminalCommandComposerActionPresentation
  | TerminalCommandComposerActionTone
  | TerminalCommandDockAccessoryMode
  | TerminalCommandDockAccessoryOptions
  | TerminalCommandDockAccessoryState
  | TerminalCommandDockAccessoryStateOptions
  | TerminalCommandDockSessionActionId
  | TerminalCommandDockSessionActionLabelMode
  | TerminalCommandDockSessionActionOptions
  | TerminalCommandDockSessionActionPlacement
  | TerminalCommandDockSessionActionPresentation
  | TerminalCommandDockSessionActionTone
  | TerminalScreenActionId
  | TerminalScreenActionLabelMode
  | TerminalScreenActionOptions
  | TerminalScreenActionPlacement
  | TerminalScreenActionPresentation
  | TerminalScreenActionTone
  | TerminalScreenSearchActionId
  | TerminalScreenSearchActionLabelMode
  | TerminalScreenSearchActionOptions
  | TerminalScreenSearchActionPlacement
  | TerminalScreenSearchActionPresentation
  | TerminalScreenSearchActionTone
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
  | TerminalWorkspacePartName
  | TerminalWorkspaceSecondaryChromeMode
  | TerminalWorkspaceSlotName;

describe("workspace react public api", () => {
  it("exports the command composer wrapper and composer utilities", async () => {
    installCustomElementRuntimeShim();

    const workspaceReact = await import("./index.js");

    expect(workspaceReact.TerminalCommandComposer.displayName).toBe("TerminalCommandComposer");
    expect(workspaceReact.TerminalScreen.displayName).toBe("TerminalScreen");
    expect(workspaceReact.terminalCommandComposerReactEvents.onCommandSubmit).toBe("tp-terminal-command-submit");
    expect(workspaceReact.terminalScreenReactEvents.onScreenInputSubmitted).toBe(
      "tp-terminal-screen-input-submitted",
    );
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTION_IDS.submit).toBe("submit");
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.id).join("|")).toBe(
      "submit|paste|interrupt|enter",
    );
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_ACTIONS.map((action) => action.tone).join("|")).toBe(
      "primary|secondary|secondary|secondary",
    );
    expect(workspaceReact.resolveTerminalCommandComposerActions({ placement: "terminal" })
      .map((action) => action.labelMode)
      .join("|")).toBe("glyph|glyph|glyph|glyph");
    expect(workspaceReact.resolveTerminalCommandComposerActions()[0]?.keyHint).toBe("Enter");
    expect(workspaceReact.TERMINAL_COMMAND_COMPOSER_EVENTS.submit).toBe("tp-terminal-command-submit");
    expect(workspaceReact.TERMINAL_SCREEN_EVENTS.pasteFailed).toBe("tp-terminal-screen-paste-failed");
    expect(workspaceReact.TERMINAL_SCREEN_ACTION_IDS.followOutput).toBe("follow-output");
    expect(workspaceReact.resolveTerminalScreenActions({ placement: "terminal", followOutput: true })
      .map((action) => action.labelMode)
      .join("|")).toBe("glyph|glyph|glyph");
    expect(workspaceReact.TERMINAL_SCREEN_SEARCH_ACTION_IDS.nextMatch).toBe("next-match");
    expect(workspaceReact.resolveTerminalScreenSearchActions({
      matchCount: 1,
      placement: "terminal",
      query: "ok",
    }).map((action) => action.labelMode).join("|")).toBe("glyph|glyph|glyph");
    expect(workspaceReact.TERMINAL_COMMAND_DOCK_ACCESSORY_MODES.bar).toBe("bar");
    expect(workspaceReact.TERMINAL_COMMAND_INPUT_STATUS_DESCRIPTION_ID).toBe("tp-command-input-status");
    expect(workspaceReact.resolveTerminalCommandDockAccessoryMode({ placement: "terminal" })).toBe("bar");
    expect(workspaceReact.TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS.saveLayout).toBe("save-layout");
    expect(workspaceReact.resolveTerminalCommandDockSessionActions({
      activePaneId: "pane-1",
      activeSessionId: "session-1",
      canPasteClipboard: true,
      canSaveLayout: true,
      canSend: true,
      canUsePane: true,
      canWriteInput: true,
      commandHistory: ["pwd"],
      draft: "pwd",
      inputCapabilityStatus: "known",
      pasteCapabilityStatus: "known",
      recentCommandEntries: [{
        ariaLabel: "Use recent command pwd",
        historyIndex: 0,
        id: "history-1",
        index: 0,
        label: "pwd",
        title: "pwd",
        value: "pwd",
      }],
      recentCommands: ["pwd"],
      saveCapabilityStatus: "known",
    }, { placement: "terminal" }).map((action) => action.labelMode).join("|")).toBe("glyph|glyph|glyph");
    expect(workspaceReact.resolveTerminalCommandDockAccessoryState({
      placement: "terminal",
      quickCommandCount: 5,
      recentCommandCount: 0,
    })).toMatchObject({
      mode: "bar",
      hasQuickCommands: true,
      hasRecentCommands: false,
    });
    expect(workspaceReact.TERMINAL_SCREEN_CHROME_MODES.compact).toBe("compact");
    expect(typeof workspaceReact.resolveTerminalScreenChromeState).toBe("function");
    expect(workspaceReact.resolveTerminalCommandComposerRows("echo one\necho two")).toBe(2);
    expect(workspaceReact.TERMINAL_WORKSPACE_CHROME_TONES.terminal).toBe("terminal");
    expect(workspaceReact.TERMINAL_WORKSPACE_INSPECTOR_MODES.collapsed).toBe("collapsed");
    expect(workspaceReact.TERMINAL_WORKSPACE_LAYOUT_PRESETS.terminal).toBe("terminal");
    expect(workspaceReact.TERMINAL_WORKSPACE_NAVIGATION_MODES.collapsed).toBe("collapsed");
    expect(workspaceReact.TERMINAL_WORKSPACE_PARTS.commandRegion).toBe("command-region");
    expect(workspaceReact.TERMINAL_WORKSPACE_SECONDARY_CHROME_MODES.terminal).toBe("terminal");
    expect(workspaceReact.TERMINAL_WORKSPACE_SLOTS.commandDock).toBe("command-dock");
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
