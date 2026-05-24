import process from "node:process";
import path from "node:path";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import { buildTerminalRuntimeBrowserUrl } from "@features/terminal-runtime-host/contracts";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  resolveDemoDefaultWorkingDirectory,
  resolveDemoDefaultShellProgram,
  startTerminalRuntimeHost,
  type TerminalRuntimeHostHandle,
  type TerminalRuntimeGatewayFaultInjectionPort,
} from "@features/terminal-runtime-host/main";
import { clearBrowserBootstrapConfig, writeBrowserBootstrapConfig } from "./browser-bootstrap-config.js";

const moduleDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(moduleDir, "../../..");
const repoRoot = path.resolve(appRoot, "../..");
const runtimeSlug = process.env.TERMINAL_DEMO_RUNTIME_SLUG ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
const rendererUrl = process.env.TERMINAL_DEMO_RENDERER_URL ?? "http://127.0.0.1:5173";
const bootstrapScope = process.env.TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE ?? "public-and-dist";
const sessionStorePath = process.env.TERMINAL_DEMO_SESSION_STORE_PATH ?? null;
const demoAutoStartSession = process.env.TERMINAL_DEMO_AUTO_START_SESSION === "1";
const failNextWorkspacePaneHistory = process.env.TERMINAL_DEMO_FAIL_NEXT_WORKSPACE_PANE_HISTORY === "1";
const workspacePaneHistoryFaultMarkerPath =
  process.env.TERMINAL_DEMO_FAIL_WORKSPACE_PANE_HISTORY_MARKER_PATH ?? null;
const demoDefaultShellProgram = resolveDemoDefaultShellProgram({
  validateWindowsPaths: true,
});
const demoDefaultWorkingDirectory = resolveDemoDefaultWorkingDirectory({
  cwd: repoRoot,
  validateExists: true,
});

let hostHandle: TerminalRuntimeHostHandle | null = null;
let bootstrapConfig: TerminalRuntimeBootstrapConfig | null = null;
let shuttingDown = false;

async function bootstrap(): Promise<void> {
  await clearBrowserBootstrapConfig({
    appRoot,
    scope: bootstrapScope,
  });

  hostHandle = await startTerminalRuntimeHost({
    runtimeSlug,
    forceRestartReadyDaemon: true,
    initialNativeSession: demoAutoStartSession
      ? {
          title: "SDK Workspace",
          program: demoDefaultShellProgram,
          cwd: demoDefaultWorkingDirectory,
        }
      : null,
    sessionStorePath,
    gatewayFaultInjection: createGatewayFaultInjection({
      failNextWorkspacePaneHistory,
      workspacePaneHistoryFaultMarkerPath,
    }),
  });

  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl: hostHandle.controlPlaneUrl,
    ...(demoDefaultWorkingDirectory ? { demoDefaultWorkingDirectory } : {}),
    demoDefaultShellProgram,
    sessionStreamUrl: hostHandle.sessionStreamUrl,
    runtimeSlug: hostHandle.runtimeSlug,
  };
  bootstrapConfig = config;
  await writeBrowserBootstrapConfig({
    appRoot,
    config,
    scope: bootstrapScope,
  });
  const browserUrl = buildTerminalRuntimeBrowserUrl(rendererUrl, config);

  console.log(`[terminal-demo-browser] runtime ${config.runtimeSlug}`);
  console.log(`[terminal-demo-browser] cwd ${demoDefaultWorkingDirectory ?? "(default)"}`);
  console.log(`[terminal-demo-browser] control ${config.controlPlaneUrl}`);
  console.log(`[terminal-demo-browser] stream ${config.sessionStreamUrl}`);
  console.log(`TERMINAL_DEMO_BROWSER_URL=${browserUrl}`);
}

function createGatewayFaultInjection(options: {
  failNextWorkspacePaneHistory: boolean;
  workspacePaneHistoryFaultMarkerPath: string | null;
}): TerminalRuntimeGatewayFaultInjectionPort | null {
  let shouldFailWorkspacePaneHistory = options.failNextWorkspacePaneHistory;
  if (!shouldFailWorkspacePaneHistory && !options.workspacePaneHistoryFaultMarkerPath) {
    return null;
  }

  return {
    beforeWorkspacePaneHistory() {
      if (shouldFailWorkspacePaneHistory) {
        shouldFailWorkspacePaneHistory = false;
        throw new Error("Simulated workspace pane history failure for degraded persistence smoke");
      }

      if (!options.workspacePaneHistoryFaultMarkerPath) {
        return;
      }

      if (!fs.existsSync(options.workspacePaneHistoryFaultMarkerPath)) {
        return;
      }

      fs.rmSync(options.workspacePaneHistoryFaultMarkerPath, { force: true });
      throw new Error("Simulated workspace pane history failure for degraded persistence smoke");
    },
  };
}

async function shutdown(exitCode = 0): Promise<void> {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  await Promise.allSettled([
    hostHandle?.dispose() ?? Promise.resolve(),
  ]);
  await clearBrowserBootstrapConfig({
    appRoot,
    expectedConfig: bootstrapConfig,
    scope: bootstrapScope,
  }).catch(() => undefined);
  bootstrapConfig = null;
  process.exit(exitCode);
}

process.on("SIGINT", () => {
  void shutdown(0);
});

process.on("SIGTERM", () => {
  void shutdown(0);
});

process.on("unhandledRejection", (error) => {
  console.error(error);
  void shutdown(1);
});

process.on("uncaughtException", (error) => {
  console.error(error);
  void shutdown(1);
});

void bootstrap().catch((error) => {
  console.error(error);
  void shutdown(1);
});
