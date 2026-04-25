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

runSync("npm", ["run", "stage:sdk"], appRoot);
runSync("npm", ["run", "build:host"], appRoot);

const vite = spawnViteDevServer(appRoot, rendererPort);

let browserHost = null;
const shutdown = () => {
  stopProcess(browserHost);
  stopProcess(vite);
  cleanupBrowserSessionStore(sessionStore);
};

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
process.on("exit", shutdown);

await waitForServer(rendererUrl, {
  child: vite,
  label: "Renderer dev server",
});

browserHost = spawn("node", ["./dist/host/browser/index.js"], {
  cwd: appRoot,
  env: {
    ...process.env,
    TERMINAL_DEMO_RENDERER_URL: rendererUrl,
    ...(sessionStore.path ? { TERMINAL_DEMO_SESSION_STORE_PATH: sessionStore.path } : {}),
  },
  stdio: "inherit",
});

console.log(`[terminal-demo-browser] session store ${sessionStore.label}`);

browserHost.on("exit", (code) => {
  shutdown();
  process.exit(code ?? 0);
});

vite.on("exit", (code) => {
  if (code && code !== 0) {
    process.exit(code);
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
    fs.rmSync(`${sessionStoreInfo.path}${suffix}`, { force: true });
  }
}
