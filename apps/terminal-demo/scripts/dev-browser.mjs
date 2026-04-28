#!/usr/bin/env node

import { spawn } from "node:child_process";
import fs from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { runSync, spawnViteDevServer, stopProcess, waitForServer } from "./dev-launcher-utils.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const rendererPort = process.env.TERMINAL_DEMO_RENDERER_PORT ?? "5173";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;
const sessionStore = resolveBrowserSessionStore();
const autoStartSession = process.env.TERMINAL_DEMO_AUTO_START_SESSION ?? "1";

runSync("npm", ["run", "stage:sdk"], appRoot);
runSync("npm", ["run", "build:host"], appRoot);

const vite = spawnViteDevServer(appRoot, rendererPort);

let browserHost = null;
let shuttingDown = false;
let shutdownPromise = null;
const shutdown = async (exitCode = 0) => {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  await Promise.allSettled([
    stopProcess(browserHost),
    stopProcess(vite),
  ]);
  cleanupBrowserSessionStore(sessionStore);
  process.exit(exitCode);
};

const requestShutdown = (exitCode = 0) => {
  shutdownPromise ??= shutdown(exitCode);
};

process.on("SIGINT", () => requestShutdown(0));
process.on("SIGTERM", () => requestShutdown(0));
process.on("exit", () => cleanupBrowserSessionStore(sessionStore));

await waitForServer(rendererUrl, {
  child: vite,
  label: "Renderer dev server",
});

browserHost = spawn(process.execPath, ["./dist/host/browser/index.js"], {
  cwd: appRoot,
  env: {
    ...process.env,
    TERMINAL_DEMO_AUTO_START_SESSION: autoStartSession,
    TERMINAL_DEMO_RENDERER_URL: rendererUrl,
    ...(sessionStore.path ? { TERMINAL_DEMO_SESSION_STORE_PATH: sessionStore.path } : {}),
  },
  stdio: "inherit",
  windowsHide: true,
});

console.log(`[terminal-demo-browser] session store ${sessionStore.label}`);
console.log(`[terminal-demo-browser] auto start session ${autoStartSession === "1" ? "enabled" : "disabled"}`);

browserHost.on("exit", (code) => {
  requestShutdown(code ?? 0);
});

vite.on("exit", (code) => {
  if (!shuttingDown && code && code !== 0) {
    requestShutdown(code);
  }
});

function resolveBrowserSessionStore() {
  const explicitPath = process.env.TERMINAL_DEMO_SESSION_STORE_PATH?.trim();
  if (explicitPath) {
    return {
      cleanup: false,
      label: `${explicitPath} (explicit)`,
      path: explicitPath,
    };
  }

  if (process.env.TERMINAL_DEMO_BROWSER_PERSIST_SESSION_STORE === "1") {
    return {
      cleanup: false,
      label: "default persistent store",
      path: null,
    };
  }

  const storePath = path.join(
    tmpdir(),
    `terminal-demo-browser-dev-store-${process.pid}-${Date.now()}.sqlite3`,
  );

  return {
    cleanup: true,
    label: `${storePath} (temporary)`,
    path: storePath,
  };
}

function cleanupBrowserSessionStore(sessionStoreInfo) {
  if (!sessionStoreInfo.cleanup || !sessionStoreInfo.path) {
    return;
  }

  for (const suffix of ["", "-shm", "-wal"]) {
    try {
      fs.rmSync(`${sessionStoreInfo.path}${suffix}`, {
        force: true,
        recursive: true,
        maxRetries: process.platform === "win32" ? 8 : 0,
        retryDelay: process.platform === "win32" ? 250 : 0,
      });
    } catch (error) {
      if (process.platform === "win32" && ["EBUSY", "ENOTEMPTY", "EPERM"].includes(error?.code)) {
        process.stderr.write(
          `[terminal-demo-browser] skipped locked session store cleanup ${sessionStoreInfo.path}${suffix}: ${error.message}\n`,
        );
        continue;
      }

      throw error;
    }
  }
}
