#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const rendererDist = path.resolve(appRoot, "dist", "renderer");
const indexPath = path.join(rendererDist, "index.html");

const rendererBundleContract = {
  markers: [
    {
      label: "static workspace mode flag",
      marker: "demoStaticWorkspace",
    },
    {
      label: "static preview test id",
      marker: "terminal-demo-static-preview",
    },
    {
      label: "NativeMux preview session id",
      marker: "preview-session-native",
    },
  ],
};

const terminalLayoutSourceContracts = [
  {
    name: "workspace terminal column attaches output to composer",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-workspace-element.ts",
    ),
    includes: [
      {
        label: "terminal column reserves tab chrome, remaining output, and auto-height composer",
        marker: "grid-template-rows: auto minmax(0, 1fr) auto;",
      },
      {
        label: "terminal column exposes a top tab strip",
        marker: "<tp-terminal-tab-strip .kernel=${this.kernel}></tp-terminal-tab-strip>",
      },
      {
        label: "terminal screen removes top-left radius after the tab strip",
        marker: "--tp-terminal-screen-panel-border-top-left-radius: 0;",
      },
      {
        label: "terminal screen removes top-right radius after the tab strip",
        marker: "--tp-terminal-screen-panel-border-top-right-radius: 0;",
      },
      {
        label: "terminal output and command dock are flush",
        marker: "gap: 0;",
      },
      {
        label: "collapsed navigation stretches terminal content before drawer rows",
        marker: '.body[data-navigation-mode="collapsed"] .content',
      },
      {
        label: "terminal screen removes bottom padding before the dock",
        marker: "--tp-terminal-screen-panel-padding-bottom: 0;",
      },
      {
        label: "terminal screen removes bottom-left radius before the dock",
        marker: "--tp-terminal-screen-viewport-border-bottom-left-radius: 0;",
      },
      {
        label: "terminal screen removes bottom-right radius before the dock",
        marker: "--tp-terminal-screen-viewport-border-bottom-right-radius: 0;",
      },
      {
        label: "terminal column remains addressable for e2e checks",
        marker: 'data-testid="tp-workspace-terminal-column"',
      },
      {
        label: "workspace supports a terminal-first collapsed inspector",
        marker: 'data-inspector-mode=${inspectorState.mode}',
      },
      {
        label: "workspace exposes collapsed inspector drawer",
        marker: 'data-testid="tp-workspace-inspector-drawer"',
      },
      {
        label: "terminal screen opts into terminal placement",
        marker: '<tp-terminal-screen .kernel=${this.kernel} placement="terminal"></tp-terminal-screen>',
      },
      {
        label: "terminal command dock opts into terminal placement",
        marker: 'placement="terminal"',
      },
      {
        label: "secondary drawers render real summary action labels",
        marker: "renderSecondarySummary",
      },
      {
        label: "secondary drawer open action is real DOM",
        marker: "secondary-toggle__action-open",
      },
      {
        label: "secondary drawer action is styleable through part contract",
        marker: 'part="secondary-summary-action"',
      },
    ],
    order: [
      {
        label: "terminal tabs render before output and the attached command dock",
        markers: [
          'data-testid="tp-workspace-terminal-column"',
          "<tp-terminal-tab-strip",
          '<tp-terminal-screen .kernel=${this.kernel} placement="terminal"></tp-terminal-screen>',
          "<tp-terminal-command-dock",
        ],
      },
    ],
  },
  {
    name: "terminal workspace layout resolves compact secondary summaries",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-workspace-layout.ts",
    ),
    includes: [
      {
        label: "terminal preset uses compact tools label",
        marker: 'summaryLabel: "Tools"',
      },
      {
        label: "terminal preset uses compact sessions label",
        marker: 'summaryLabel: "Sessions"',
      },
      {
        label: "closed secondary action label is explicit state",
        marker: 'summaryActionClosedLabel: "Open"',
      },
      {
        label: "open secondary action label is explicit state",
        marker: 'summaryActionOpenLabel: "Close"',
      },
    ],
  },
  {
    name: "static demo opts into terminal-first workspace layout",
    relativePath: path.join(
      "apps",
      "terminal-demo",
      "src",
      "renderer",
      "app",
      "TerminalDemoWorkspaceApp.tsx",
    ),
    includes: [
      {
        label: "demo opts into the public terminal-first workspace preset",
        marker: 'layoutPreset="terminal"',
      },
    ],
  },
  {
    name: "command dock puts composer first in terminal placement",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-dock-element.ts",
    ),
    includes: [
      {
        label: "terminal placement has explicit dock ordering",
        marker: "const orderedDockContent = isTerminalPlacement",
      },
      {
        label: "composer participates in terminal-first ordering",
        marker: "composerTemplate,",
      },
      {
        label: "dock does not add a divider against terminal output",
        marker: "border-top-width: 0;",
      },
      {
        label: "dock keeps only bottom terminal radius",
        marker: "border-radius: 0 0 var(--tp-radius-md) var(--tp-radius-md);",
      },
      {
        label: "dock remains addressable for e2e checks",
        marker: 'data-testid="tp-command-dock"',
      },
      {
        label: "dock links composer input to the command input status",
        marker: ".inputDescriptionId=${TERMINAL_COMMAND_INPUT_STATUS_DESCRIPTION_ID}",
      },
      {
        label: "input status exposes a host-owned description id",
        marker: "id=${badge.id === TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input",
      },
      {
        label: "input status announces capability changes politely",
        marker: 'aria-live=${badge.id === TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input ? "polite" : nothing}',
      },
      {
        label: "input status announces complete status labels",
        marker: 'aria-atomic=${badge.id === TERMINAL_COMMAND_DOCK_STATUS_BADGE_IDS.input ? "true" : nothing}',
      },
      {
        label: "session actions come from a presentation resolver",
        marker: "resolveTerminalCommandDockSessionActions",
      },
      {
        label: "session actions expose stable action ids",
        marker: "data-session-action=${action.id}",
      },
      {
        label: "session actions expose label mode for compact adapters",
        marker: "data-session-action-label-mode=${action.labelMode}",
      },
      {
        label: "session actions expose placement for layout adapters",
        marker: "data-session-action-placement=${action.placement}",
      },
      {
        label: "session actions expose action tone for theme adapters",
        marker: "data-session-action-tone=${action.tone}",
      },
      {
        label: "session danger tone drives visual state",
        marker: 'button[data-session-action-tone="danger"]',
      },
      {
        label: "session compact glyph mode drives terminal styling",
        marker: 'button[data-session-action-label-mode="glyph"]',
      },
      {
        label: "composer primary tone drives terminal styling",
        marker: 'button[data-action-tone="primary"]',
      },
      {
        label: "composer compact glyph mode drives terminal styling",
        marker: 'button[data-action-label-mode="glyph"]',
      },
      {
        label: "quick commands expose stable ids",
        marker: "data-quick-command=${command.id}",
      },
      {
        label: "quick commands expose action tone",
        marker: "data-quick-command-tone=${command.tone}",
      },
      {
        label: "quick commands expose resolved accessible labels",
        marker: "aria-label=${command.ariaLabel}",
      },
      {
        label: "recent command chips render stable history ids",
        marker: "data-command-history-entry=${command.id}",
      },
      {
        label: "recent command chips render original history indexes",
        marker: "data-history-index=${command.historyIndex}",
      },
      {
        label: "recent command chips expose resolved accessible labels",
        marker: "aria-label=${command.ariaLabel}",
      },
      {
        label: "compact terminal placement keeps recent history addressable",
        marker: '.dock[data-placement="terminal"] .history-row',
      },
      {
        label: "compact terminal placement condenses recent history label",
        marker: '.dock[data-placement="terminal"] .history-label',
      },
      {
        label: "compact terminal placement keeps quick and recent commands on one accessory row",
        marker: '"quick history"',
      },
      {
        label: "accessory bar exposes recent-history layout state",
        marker: "data-has-command-history=${String(accessoryState.hasRecentCommands)}",
      },
      {
        label: "accessory bar exposes quick-command counts",
        marker: "data-quick-command-count=${accessoryState.quickCommandCount}",
      },
    ],
    order: [
      {
        label: "composer appears before status/header content in terminal placement",
        markers: [
          "const orderedDockContent = isTerminalPlacement",
          "composerTemplate,",
          "errorTemplate,",
          "headerTemplate,",
        ],
      },
    ],
  },
  {
    name: "terminal command composer element exposes action label mode",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-composer-element.ts",
    ),
    includes: [
      {
        label: "composer action label mode is a DOM contract",
        marker: "data-action-label-mode=${action.labelMode}",
      },
      {
        label: "composer action placement remains addressable",
        marker: "data-action-placement=${action.placement}",
      },
    ],
  },
  {
    name: "terminal command composer actions resolve glyph presentation",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-composer-actions.ts",
    ),
    includes: [
      {
        label: "action label mode is a public presentation contract",
        marker: "TerminalCommandComposerActionLabelMode",
      },
      {
        label: "terminal submit action resolves to a compact glyph",
        marker: 'terminal: "\\u25b6"',
      },
      {
        label: "terminal placement opts into glyph presentation",
        marker: 'terminal: "glyph"',
      },
      {
        label: "label mode is resolved outside rendering",
        marker: "resolveTerminalCommandComposerActionLabelMode",
      },
    ],
  },
  {
    name: "command dock accessories resolve presentation metadata",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-dock-accessories.ts",
    ),
    includes: [
      {
        label: "accessory state is resolved outside rendering",
        marker: "TerminalCommandDockAccessoryState",
      },
      {
        label: "quick command count is normalized",
        marker: "const quickCommandCount = normalizeAccessoryCount",
      },
      {
        label: "recent command presence is explicit",
        marker: "hasRecentCommands: recentCommandCount > 0",
      },
    ],
  },
  {
    name: "quick commands resolve presentation metadata",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-quick-commands.ts",
    ),
    includes: [
      {
        label: "quick command tones are exported as a public contract",
        marker: "TERMINAL_COMMAND_QUICK_COMMAND_TONES",
      },
      {
        label: "quick commands resolve to presentation state",
        marker: "TerminalCommandQuickCommandPresentation",
      },
      {
        label: "quick commands normalize stable ids",
        marker: "resolveQuickCommandId",
      },
      {
        label: "quick commands resolve accessible labels",
        marker: "ariaLabel: explicitAriaLabel ?? fallbackLabel",
      },
      {
        label: "quick commands resolve tones independently from rendering",
        marker: "tone: resolveQuickCommandTone",
      },
    ],
  },
  {
    name: "recent commands resolve presentation metadata",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-recent-commands.ts",
    ),
    includes: [
      {
        label: "recent command chips resolve to presentation state",
        marker: "TerminalCommandRecentCommandPresentation",
      },
      {
        label: "recent command ids avoid command value selectors",
        marker: "id: `history-${historyIndex + 1}`",
      },
      {
        label: "recent command entries preserve original history position",
        marker: "historyIndex",
      },
      {
        label: "recent command entries expose accessible labels",
        marker: "ariaLabel: `Use recent command ${command}`",
      },
    ],
  },
  {
    name: "terminal screen actions stay presentation-driven",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-screen-element.ts",
    ),
    includes: [
      {
        label: "screen actions come from a presentation resolver",
        marker: "resolveTerminalScreenActions",
      },
      {
        label: "screen actions expose stable action ids",
        marker: "data-screen-action=${action.id}",
      },
      {
        label: "screen actions expose action tone for theme adapters",
        marker: "data-screen-action-tone=${action.tone}",
      },
      {
        label: "screen actions expose compact label mode",
        marker: "data-screen-action-label-mode=${action.labelMode}",
      },
      {
        label: "screen actions expose placement metadata",
        marker: "data-screen-action-placement=${action.placement}",
      },
      {
        label: "screen action tone drives visual state",
        marker: 'button[data-screen-action-tone="primary"]',
      },
      {
        label: "screen action label mode drives terminal styling",
        marker: 'button[data-screen-action-label-mode="glyph"]',
      },
      {
        label: "screen actions route through action ids",
        marker: "handleScreenActionClick",
      },
      {
        label: "screen search actions come from a presentation resolver",
        marker: "resolveTerminalScreenSearchActions",
      },
      {
        label: "screen search actions expose stable action ids",
        marker: "data-screen-search-action=${action.id}",
      },
      {
        label: "screen search actions expose compact label mode",
        marker: "data-screen-search-action-label-mode=${action.labelMode}",
      },
      {
        label: "screen search actions expose placement metadata",
        marker: "data-screen-search-action-placement=${action.placement}",
      },
      {
        label: "screen search compact glyph mode drives terminal styling",
        marker: 'button[data-screen-search-action-label-mode="glyph"]',
      },
      {
        label: "screen search actions route through action ids",
        marker: "handleSearchActionClick",
      },
      {
        label: "screen search uses native search semantics",
        marker: 'type="search"',
      },
      {
        label: "screen search disables browser autocomplete",
        marker: 'autocomplete="off"',
      },
      {
        label: "screen search disables browser autocapitalize",
        marker: 'autocapitalize="off"',
      },
      {
        label: "screen search disables browser autocorrect",
        marker: 'autocorrect="off"',
      },
      {
        label: "screen search presents search enter semantics",
        marker: 'enterkeyhint="search"',
      },
      {
        label: "screen search uses search keyboard mode",
        marker: 'inputmode="search"',
      },
      {
        label: "screen search disables spellcheck",
        marker: 'spellcheck="false"',
      },
      {
        label: "screen search describes current match count",
        marker: "aria-describedby=${TERMINAL_SCREEN_SEARCH_COUNT_ID}",
      },
      {
        label: "screen search count has a stable description id",
        marker: "id=${TERMINAL_SCREEN_SEARCH_COUNT_ID}",
      },
      {
        label: "screen search count announces complete labels",
        marker: 'aria-atomic="true"',
      },
      {
        label: "screen search match avoids template whitespace",
        marker: ">${segment.value}</mark>",
      },
    ],
  },
  {
    name: "terminal screen search actions stay presentation-driven",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-screen-search-actions.ts",
    ),
    includes: [
      {
        label: "screen search action ids are exported as a public contract",
        marker: "TERMINAL_SCREEN_SEARCH_ACTION_IDS",
      },
      {
        label: "screen search actions resolve through a pure function",
        marker: "export function resolveTerminalScreenSearchActions",
      },
      {
        label: "screen search label mode is a public presentation contract",
        marker: "TerminalScreenSearchActionLabelMode",
      },
      {
        label: "terminal previous match resolves to a compact glyph",
        marker: 'label: compact ? "\\u2191" : "Prev"',
      },
      {
        label: "terminal next match resolves to a compact glyph",
        marker: 'label: compact ? "\\u2193" : "Next"',
      },
      {
        label: "terminal clear query resolves to a compact glyph",
        marker: 'label: compact ? "\\u00d7" : "Clear"',
      },
      {
        label: "label mode is resolved outside rendering",
        marker: "resolveTerminalScreenSearchActionLabelMode",
      },
    ],
  },
  {
    name: "terminal screen action labels stay presentation-driven",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-screen-actions.ts",
    ),
    includes: [
      {
        label: "screen action ids are exported as a public contract",
        marker: "TERMINAL_SCREEN_ACTION_IDS",
      },
      {
        label: "screen actions resolve through a pure function",
        marker: "export function resolveTerminalScreenActions",
      },
      {
        label: "screen action label mode is a public presentation contract",
        marker: "TerminalScreenActionLabelMode",
      },
      {
        label: "terminal placement uses compact pause glyph",
        marker: 'return compact ? "\\u23f8" : "Following";',
      },
      {
        label: "terminal placement uses compact latest glyph",
        marker: 'label: compact ? "\\u2193" : "Scroll latest"',
      },
      {
        label: "copy failure state remains explicit",
        marker: 'return compact ? "!" : "Copy failed";',
      },
      {
        label: "label mode is resolved outside rendering",
        marker: "resolveTerminalScreenActionLabelMode",
      },
    ],
  },
  {
    name: "terminal command dock session actions stay presentation-driven",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-dock-session-actions.ts",
    ),
    includes: [
      {
        label: "session action ids are exported as a public contract",
        marker: "TERMINAL_COMMAND_DOCK_SESSION_ACTION_IDS",
      },
      {
        label: "session actions resolve through a pure function",
        marker: "export function resolveTerminalCommandDockSessionActions",
      },
      {
        label: "session action label mode is a public presentation contract",
        marker: "TerminalCommandDockSessionActionLabelMode",
      },
      {
        label: "terminal placement uses compact save glyph",
        marker: 'label: compact ? "\\u21e9" : "Save layout"',
      },
      {
        label: "terminal placement uses compact refresh glyph",
        marker: 'label: compact ? "\\u21bb" : "Refresh terminal"',
      },
      {
        label: "terminal placement uses compact clear glyph",
        marker: '? "\\u232b"',
      },
      {
        label: "clear history still requires explicit confirmation",
        marker: "historyClearConfirmationArmed",
      },
    ],
  },
  {
    name: "terminal tab strip preserves keyboard focus contract",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-tab-strip-element.ts",
    ),
    includes: [
      {
        label: "tab strip exposes tablist semantics",
        marker: 'role="tablist"',
      },
      {
        label: "tab strip exposes tab semantics",
        marker: 'role="tab"',
      },
      {
        label: "tab strip keeps stable item keys for focus recovery",
        marker: "data-tab-key=${tab.itemKey}",
      },
      {
        label: "tab strip uses roving tab order",
        marker: "tabindex=${String(tab.tabIndex)}",
      },
      {
        label: "tab close controls use explicit tab order",
        marker: "tabindex=${String(tab.closeTabIndex)}",
      },
      {
        label: "tab strip handles keyboard events through an adapter",
        marker: "handleTabKeydown",
      },
      {
        label: "tab strip restores focus after topology updates",
        marker: "focusPendingTabButton",
      },
    ],
  },
  {
    name: "terminal tab strip keyboard intent stays pure",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-tab-strip-keyboard-navigation.ts",
    ),
    includes: [
      {
        label: "keyboard resolver is exported as a pure function",
        marker: "export function resolveTerminalTabStripKeyboardIntent",
      },
      {
        label: "keyboard resolver supports backward navigation",
        marker: 'case "ArrowLeft":',
      },
      {
        label: "keyboard resolver supports forward navigation",
        marker: 'case "ArrowRight":',
      },
      {
        label: "keyboard resolver supports first tab navigation",
        marker: 'case "Home":',
      },
      {
        label: "keyboard resolver supports last tab navigation",
        marker: 'case "End":',
      },
      {
        label: "keyboard resolver supports keyboard close intent",
        marker: 'input.key === "Delete" || input.key === "Backspace"',
      },
    ],
  },
  {
    name: "terminal screen hides editor-style gutters in terminal placement",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-screen-element.ts",
    ),
    includes: [
      {
        label: "screen reserves an explicit row for tools before output",
        marker: "grid-template-rows: auto auto auto minmax(0, 1fr);",
      },
      {
        label: "terminal placement has its own line layout",
        marker: ".screen[data-placement=\"terminal\"] .line",
      },
      {
        label: "terminal placement uses a single output column",
        marker: "grid-template-columns: minmax(0, 1fr);",
      },
      {
        label: "terminal placement targets the gutter",
        marker: ".screen[data-placement=\"terminal\"] .gutter",
      },
      {
        label: "terminal placement hides the gutter visually",
        marker: "display: none;",
      },
      {
        label: "terminal placement hides the gutter from assistive tech",
        marker: 'aria-hidden="true"',
      },
    ],
  },
  {
    name: "static preview advertises ready native backend capabilities",
    relativePath: path.join(
      "apps",
      "terminal-demo",
      "src",
      "renderer",
      "app",
      "terminal-demo-static-workspace.ts",
    ),
    includes: [
      {
        label: "static preview exposes backend capability fixture",
        marker: "createDemoPreviewBackendCapabilities",
      },
      {
        label: "static preview makes focused pane input ready",
        marker: "pane_input_write: true",
      },
      {
        label: "static preview makes paste ready",
        marker: "pane_paste_write: true",
      },
      {
        label: "static preview makes save layout ready",
        marker: "explicit_session_save: true",
      },
      {
        label: "static kernel serves backend capability queries",
        marker: "getBackendCapabilities: async (backend: BackendKind)",
      },
      {
        label: "static kernel models command input locally",
        marker: "dispatchStaticMuxCommand",
      },
      {
        label: "static kernel models advertised terminal tab creation",
        marker: "createStaticPreviewTab",
      },
      {
        label: "static kernel makes simulated output explicit",
        marker: "static preview: simulated output, no native host is attached",
      },
      {
        label: "static kernel simulates common demo commands",
        marker: "On branch demo/static-preview",
      },
      {
        label: "static kernel models save layout locally",
        marker: "createStaticSavedSessionSummary",
      },
    ],
  },
  {
    name: "command composer has compact and multiline layout state",
    relativePath: path.join(
      "sdk",
      "packages",
      "workspace-elements",
      "src",
      "elements",
      "terminal-command-composer-element.ts",
    ),
    includes: [
      {
        label: "composer renders actions from the public action presentation contract",
        marker: "resolveTerminalCommandComposerActions",
      },
      {
        label: "composer exposes current row count",
        marker: "data-row-count",
      },
      {
        label: "composer disables browser autocomplete for shell input",
        marker: 'autocomplete="off"',
      },
      {
        label: "composer disables browser autocapitalize for shell input",
        marker: 'autocapitalize="off"',
      },
      {
        label: "composer disables browser autocorrect for shell input",
        marker: 'autocorrect="off"',
      },
      {
        label: "composer presents terminal submit semantics to mobile keyboards",
        marker: 'enterkeyhint="send"',
      },
      {
        label: "composer disables spellcheck for shell input",
        marker: 'spellcheck="false"',
      },
      {
        label: "composer can describe the input from host status text",
        marker: "aria-describedby=${this.inputDescriptionId || nothing}",
      },
      {
        label: "composer exposes stable action ids for e2e and host wrappers",
        marker: "data-action=${action.id}",
      },
      {
        label: "composer exposes disabled state for theme adapters",
        marker: "data-action-disabled=${String(disabled)}",
      },
      {
        label: "composer exposes action tone for theme adapters",
        marker: "data-action-tone=${action.tone}",
      },
      {
        label: "composer exposes key hints for terminal-style controls",
        marker: "data-key-hint=${action.keyHint ?? nothing}",
      },
      {
        label: "composer only advertises actual UI shortcuts to assistive tech",
        marker: "aria-keyshortcuts=${action.ariaKeyShortcuts ?? nothing}",
      },
      {
        label: "composer exposes multiline state",
        marker: "data-multiline",
      },
      {
        label: "composer supports minimum rows",
        marker: "minRows",
      },
      {
        label: "composer supports maximum rows",
        marker: "maxRows",
      },
      {
        label: "composer accepts a host-owned description id",
        marker: "inputDescriptionId",
      },
      {
        label: "composer synchronizes textarea height after draft changes",
        marker: "syncCommandInputHeight",
      },
    ],
  },
];

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exitCode = 1;
}

async function main() {
  await verifyTerminalLayoutSourceContracts();
  await verifyRendererBundleContract();

  process.stdout.write(`Verified static renderer preview bundle and layout contracts: ${path.relative(appRoot, indexPath)}\n`);
}

async function verifyTerminalLayoutSourceContracts() {
  const failures = [];

  await Promise.all(terminalLayoutSourceContracts.map(async (contract) => {
    const sourcePath = path.join(repoRoot, contract.relativePath);
    let source;
    try {
      source = await fs.readFile(sourcePath, "utf8");
    } catch (error) {
      failures.push(`${contract.name}: cannot read ${contract.relativePath}: ${formatError(error)}`);
      return;
    }

    failures.push(...collectMissingSourceMarkers(contract, source));
    failures.push(...collectSourceOrderFailures(contract, source));
  }));

  if (failures.length > 0) {
    throw new Error(`Static renderer layout contract failed:\n${failures.map((failure) => `- ${failure}`).join("\n")}`);
  }
}

async function verifyRendererBundleContract() {
  const indexHtml = await fs.readFile(indexPath, "utf8");
  const moduleScripts = [...indexHtml.matchAll(/<script\b([^>]*)>/g)]
    .flatMap((match) => {
      const attributes = match[1];
      if (!/\btype="module"(?:\s|$)/.test(attributes)) {
        return [];
      }

      const srcMatch = attributes.match(/\bsrc="([^"]+)"/);
      return srcMatch ? [srcMatch[1]] : [];
    });

  if (moduleScripts.length === 0) {
    throw new Error(`Renderer index does not reference a module script: ${indexPath}`);
  }

  const bundlePaths = moduleScripts.map((scriptPath) => path.resolve(rendererDist, scriptPath.replace(/^\.\//, "")));
  const missingBundles = [];
  for (const bundlePath of bundlePaths) {
    try {
      await fs.access(bundlePath);
    } catch {
      missingBundles.push(bundlePath);
    }
  }

  if (missingBundles.length > 0) {
    throw new Error(`Renderer bundle files are missing: ${missingBundles.join(", ")}`);
  }

  const bundles = await Promise.all(bundlePaths.map(async (bundlePath) => fs.readFile(bundlePath, "utf8")));
  const combinedSource = bundles.join("\n");
  const missingPreviewMarkers = rendererBundleContract.markers.filter(({ marker }) => !combinedSource.includes(marker));

  if (missingPreviewMarkers.length > 0) {
    throw new Error(
      `Static preview contract markers are missing from renderer bundle: ${missingPreviewMarkers
        .map(formatContractMarker)
        .join(", ")}`,
    );
  }
}

function collectMissingSourceMarkers(contract, source) {
  return contract.includes
    .filter(({ marker }) => !source.includes(marker))
    .map((marker) => `${contract.name}: missing ${formatContractMarker(marker)} in ${contract.relativePath}`);
}

function collectSourceOrderFailures(contract, source) {
  return (contract.order ?? []).flatMap(({ label, markers }) => {
    let previousIndex = -1;
    const orderFailures = [];

    for (const marker of markers) {
      const index = source.indexOf(marker, previousIndex + 1);
      if (index === -1) {
        orderFailures.push(`${contract.name}: missing ordered marker "${marker}" for ${label} in ${contract.relativePath}`);
        break;
      }

      if (index <= previousIndex) {
        orderFailures.push(`${contract.name}: markers out of order for ${label} in ${contract.relativePath}`);
        break;
      }

      previousIndex = index;
    }

    return orderFailures;
  });
}

function formatContractMarker({ label, marker }) {
  return `${label} (${JSON.stringify(marker)})`;
}

function formatError(error) {
  return error instanceof Error ? error.message : String(error);
}

main().catch((error) => {
  fail(error instanceof Error ? error.message : String(error));
});
