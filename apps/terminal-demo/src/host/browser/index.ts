import process from "node:process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import {
  buildTerminalRuntimeBrowserUrl,
  TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH,
} from "@features/terminal-runtime-host/contracts";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  startTerminalRuntimeHost,
  type TerminalRuntimeHostHandle,
} from "@features/terminal-runtime-host/main";

const runtimeSlug = process.env.TERMINAL_DEMO_RUNTIME_SLUG ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
const rendererUrl = process.env.TERMINAL_DEMO_RENDERER_URL ?? "http://127.0.0.1:5173";
const bootstrapScope = process.env.TERMINAL_DEMO_BROWSER_BOOTSTRAP_SCOPE ?? "public-and-dist";
const sessionStorePath = process.env.TERMINAL_DEMO_SESSION_STORE_PATH ?? null;

let hostHandle: TerminalRuntimeHostHandle | null = null;
let shuttingDown = false;

async function bootstrap(): Promise<void> {
  hostHandle = await startTerminalRuntimeHost({
    runtimeSlug,
    forceRestartReadyDaemon: true,
    sessionStorePath,
  });

  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl: hostHandle.controlPlaneUrl,
    sessionStreamUrl: hostHandle.sessionStreamUrl,
    runtimeSlug: hostHandle.runtimeSlug,
  };
  await writeBrowserBootstrapConfig(config);
  const browserUrl = buildTerminalRuntimeBrowserUrl(rendererUrl, config);

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

async function writeBrowserBootstrapConfig(config: TerminalRuntimeBootstrapConfig): Promise<void> {
  const moduleDir = path.dirname(fileURLToPath(import.meta.url));
  const appRoot = path.resolve(moduleDir, "../../..");
  const relativeTarget = TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH.replace(/^\/+/, "");
  const targets = resolveBootstrapTargets(appRoot, relativeTarget);
  const payload = `${JSON.stringify(config, null, 2)}\n`;

  await Promise.all(targets.map(async (targetPath) => {
    await fs.mkdir(path.dirname(targetPath), { recursive: true });
    await fs.writeFile(targetPath, payload, "utf8");
  }));
}

function resolveBootstrapTargets(appRoot: string, relativeTarget: string): string[] {
  if (bootstrapScope === "dist-only") {
    return [path.join(appRoot, "dist", "renderer", relativeTarget)];
  }

  if (bootstrapScope === "public-only") {
    return [path.join(appRoot, "public", relativeTarget)];
  }

  return [
    path.join(appRoot, "public", relativeTarget),
    path.join(appRoot, "dist", "renderer", relativeTarget),
  ];
}
