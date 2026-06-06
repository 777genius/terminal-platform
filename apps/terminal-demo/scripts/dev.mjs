#!/usr/bin/env node

import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  runSync,
  spawnElectronPreview,
  spawnViteDevServer,
  stopProcess,
  waitForServer,
} from "./dev-launcher-utils.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const rendererPort = process.env.TERMINAL_DEMO_RENDERER_PORT ?? "5173";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;

runSync("npm", ["run", "stage:sdk"], appRoot);
runSync("npm", ["run", "build:host"], appRoot);

const vite = spawnViteDevServer(appRoot, rendererPort);

let electron = null;
let shuttingDown = false;
let shutdownPromise = null;
const shutdown = async (exitCode = 0) => {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  await Promise.allSettled([
    stopProcess(electron),
    stopProcess(vite),
  ]);
  process.exit(exitCode);
};

const requestShutdown = (exitCode = 0) => {
  shutdownPromise ??= shutdown(exitCode);
};

process.on("SIGINT", () => requestShutdown(0));
process.on("SIGTERM", () => requestShutdown(0));

await waitForServer(rendererUrl, {
  child: vite,
  label: "Renderer dev server",
});

electron = spawnElectronPreview(appRoot, rendererUrl);

electron.on("exit", (code) => {
  requestShutdown(code ?? 0);
});

vite.on("exit", (code) => {
  if (!shuttingDown && code && code !== 0) {
    requestShutdown(code);
  }
});
