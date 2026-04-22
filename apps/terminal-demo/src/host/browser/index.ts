import process from "node:process";
import type { TerminalDemoBootstrapConfig } from "@features/terminal-workspace/contracts";
import { buildTerminalDemoBrowserUrl } from "@features/terminal-workspace/contracts";
import {
  DEFAULT_TERMINAL_WORKSPACE_RUNTIME_SLUG,
  startTerminalWorkspaceHost,
  type TerminalWorkspaceHostHandle,
} from "@features/terminal-workspace/main";

const runtimeSlug = process.env.TERMINAL_DEMO_RUNTIME_SLUG ?? DEFAULT_TERMINAL_WORKSPACE_RUNTIME_SLUG;
const rendererUrl = process.env.TERMINAL_DEMO_RENDERER_URL ?? "http://127.0.0.1:5173";

let hostHandle: TerminalWorkspaceHostHandle | null = null;
let shuttingDown = false;

async function bootstrap(): Promise<void> {
  hostHandle = await startTerminalWorkspaceHost({ runtimeSlug });

  const config: TerminalDemoBootstrapConfig = {
    controlPlaneUrl: hostHandle.controlPlaneUrl,
    sessionStreamUrl: hostHandle.sessionStreamUrl,
    runtimeSlug: hostHandle.runtimeSlug,
  };
  const browserUrl = buildTerminalDemoBrowserUrl(rendererUrl, config);

  console.log(`[terminal-demo-browser] runtime ${config.runtimeSlug}`);
  console.log(`[terminal-demo-browser] control ${config.controlPlaneUrl}`);
  console.log(`[terminal-demo-browser] stream ${config.sessionStreamUrl}`);
  console.log(`TERMINAL_DEMO_BROWSER_URL=${browserUrl}`);
}

async function shutdown(exitCode = 0): Promise<void> {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  await Promise.allSettled([
    hostHandle?.dispose() ?? Promise.resolve(),
  ]);
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
