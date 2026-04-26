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
        label: "terminal column reserves remaining height for output and auto height for composer",
        marker: "grid-template-rows: minmax(0, 1fr) auto;",
      },
      {
        label: "terminal output and command dock are flush",
        marker: "gap: 0;",
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
        label: "terminal screen opts into terminal placement",
        marker: '<tp-terminal-screen .kernel=${this.kernel} placement="terminal"></tp-terminal-screen>',
      },
      {
        label: "terminal command dock opts into terminal placement",
        marker: 'placement="terminal"',
      },
    ],
    order: [
      {
        label: "terminal output renders before the attached command dock",
        markers: [
          'data-testid="tp-workspace-terminal-column"',
          '<tp-terminal-screen .kernel=${this.kernel} placement="terminal"></tp-terminal-screen>',
          "<tp-terminal-command-dock",
        ],
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
        marker: "border-radius: 0 0 0.6rem 0.6rem;",
      },
      {
        label: "dock remains addressable for e2e checks",
        marker: 'data-testid="tp-command-dock"',
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
        label: "static kernel renders accepted preview input",
        marker: "preview runtime accepted input without native host",
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
        label: "composer exposes stable action ids for e2e and host wrappers",
        marker: "data-action=${action.id}",
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
