export type TerminalPlatformPackageImporter = (
  specifier: string,
) => Promise<Record<string, unknown>>;

export interface TerminalPlatformPackedConsumerSmokeEntry {
  readonly specifier: string;
  readonly expectedRuntimeExports?: readonly string[];
  readonly expectNoRuntimeExports?: boolean;
}

export interface TerminalPlatformPackedConsumerSmokeFailure {
  readonly specifier: string;
  readonly message: string;
}

export interface TerminalPlatformPackedConsumerSmokeResult {
  readonly ok: boolean;
  readonly checked: number;
  readonly failures: readonly TerminalPlatformPackedConsumerSmokeFailure[];
}

export interface RunTerminalPlatformPackedConsumerSmokeOptions {
  readonly entries?: readonly TerminalPlatformPackedConsumerSmokeEntry[];
}

export const TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC = [
  {
    specifier: "@terminal-platform/foundation",
    expectedRuntimeExports: [
      "AsyncLane",
      "BasePlatformError",
      "GenerationToken",
      "ResourceScope",
      "createExternalStore",
      "noopTelemetrySink",
      "toDisposable",
    ],
  },
  {
    specifier: "@terminal-platform/runtime-types",
    expectedRuntimeExports: ["RUNTIME_TYPES_SCHEMA_VERSION"],
  },
  {
    specifier: "@terminal-platform/design-tokens",
    expectedRuntimeExports: [
      "TERMINAL_PLATFORM_THEME_ATTRIBUTE",
      "terminalPlatformThemeCssText",
      "terminalPlatformThemeManifests",
    ],
  },
  {
    specifier: "@terminal-platform/design-tokens/css",
    expectedRuntimeExports: ["terminalPlatformThemeCssText"],
  },
  {
    specifier: "@terminal-platform/design-tokens/themes",
    expectedRuntimeExports: ["terminalPlatformThemeManifests"],
  },
  {
    specifier: "@terminal-platform/workspace-contracts",
    expectedRuntimeExports: [
      "WORKSPACE_CONTRACTS_SCHEMA_VERSION",
      "WorkspaceError",
      "toWorkspaceError",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-contracts/commands",
    expectNoRuntimeExports: true,
  },
  {
    specifier: "@terminal-platform/workspace-contracts/errors",
    expectedRuntimeExports: ["WorkspaceError", "toWorkspaceError"],
  },
  {
    specifier: "@terminal-platform/workspace-contracts/observations",
    expectNoRuntimeExports: true,
  },
  {
    specifier: "@terminal-platform/workspace-contracts/ports",
    expectNoRuntimeExports: true,
  },
  {
    specifier: "@terminal-platform/workspace-core",
    expectedRuntimeExports: [
      "DEFAULT_COMMAND_HISTORY_LIMIT",
      "createWorkspaceHost",
      "createWorkspaceKernel",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-core/bootstrap",
    expectedRuntimeExports: ["createWorkspaceHost"],
  },
  {
    specifier: "@terminal-platform/workspace-core/testing",
    expectedRuntimeExports: [
      "createWorkspaceTestClock",
      "flushWorkspaceMicrotasks",
      "recordWorkspaceSnapshots",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-adapter-websocket",
    expectedRuntimeExports: [
      "createWorkspaceWebSocketTransport",
      "decodeWorkspaceWebSocketPayload",
      "encodeWorkspaceWebSocketPayload",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-adapter-websocket/protocol",
    expectNoRuntimeExports: true,
  },
  {
    specifier: "@terminal-platform/workspace-adapter-preload",
    expectedRuntimeExports: ["createWorkspacePreloadTransport"],
  },
  {
    specifier: "@terminal-platform/workspace-adapter-memory",
    expectedRuntimeExports: [
      "createDefaultMemoryWorkspaceFixture",
      "createMemoryWorkspaceTransport",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-elements",
    expectedRuntimeExports: [
      "TerminalWorkspaceElement",
      "defineTerminalPlatformElements",
      "resolveTerminalWorkspaceLayoutState",
    ],
  },
  {
    specifier: "@terminal-platform/workspace-elements/define",
    expectedRuntimeExports: ["defineTerminalPlatformElements"],
  },
  {
    specifier: "@terminal-platform/workspace-elements/styles",
    expectedRuntimeExports: ["terminalElementStyles"],
  },
  {
    specifier: "@terminal-platform/workspace-react",
    expectedRuntimeExports: [
      "TerminalWorkspace",
      "TerminalCommandComposer",
      "useWorkspaceSnapshot",
    ],
  },
  {
    specifier: "@terminal-platform/testing",
    expectedRuntimeExports: [
      "TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC",
      "assertTerminalPlatformPackedConsumerSmoke",
      "createWorkspaceTestHarness",
      "runTerminalPlatformPackedConsumerSmoke",
    ],
  },
] as const satisfies readonly TerminalPlatformPackedConsumerSmokeEntry[];

export async function runTerminalPlatformPackedConsumerSmoke(
  importer: TerminalPlatformPackageImporter,
  options: RunTerminalPlatformPackedConsumerSmokeOptions = {},
): Promise<TerminalPlatformPackedConsumerSmokeResult> {
  const entries = options.entries ?? TERMINAL_PLATFORM_PACKED_CONSUMER_SMOKE_SPEC;
  const failures: TerminalPlatformPackedConsumerSmokeFailure[] = [];

  for (const entry of entries) {
    try {
      const moduleExports = await importer(entry.specifier);
      failures.push(...validateSmokeEntry(entry, moduleExports));
    } catch (error) {
      failures.push({
        specifier: entry.specifier,
        message: `failed to import: ${errorMessage(error)}`,
      });
    }
  }

  return {
    ok: failures.length === 0,
    checked: entries.length,
    failures,
  };
}

export async function assertTerminalPlatformPackedConsumerSmoke(
  importer: TerminalPlatformPackageImporter,
  options: RunTerminalPlatformPackedConsumerSmokeOptions = {},
): Promise<TerminalPlatformPackedConsumerSmokeResult> {
  const result = await runTerminalPlatformPackedConsumerSmoke(importer, options);

  if (!result.ok) {
    throw new Error(formatPackedConsumerSmokeFailures(result.failures));
  }

  return result;
}

function validateSmokeEntry(
  entry: TerminalPlatformPackedConsumerSmokeEntry,
  moduleExports: Record<string, unknown>,
): TerminalPlatformPackedConsumerSmokeFailure[] {
  const failures: TerminalPlatformPackedConsumerSmokeFailure[] = [];
  const runtimeExportNames = Object.keys(moduleExports);

  if (entry.expectNoRuntimeExports && runtimeExportNames.length > 0) {
    failures.push({
      specifier: entry.specifier,
      message: `expected no runtime exports, found ${runtimeExportNames.join(", ")}`,
    });
  }

  for (const expectedExport of entry.expectedRuntimeExports ?? []) {
    if (!(expectedExport in moduleExports)) {
      failures.push({
        specifier: entry.specifier,
        message: `missing runtime export "${expectedExport}"`,
      });
    }
  }

  return failures;
}

function formatPackedConsumerSmokeFailures(
  failures: readonly TerminalPlatformPackedConsumerSmokeFailure[],
): string {
  return [
    "Terminal Platform packed consumer smoke failed:",
    ...failures.map((failure) => `- ${failure.specifier}: ${failure.message}`),
  ].join("\n");
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}
