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
const shutdown = () => {
  stopProcess(electron);
  stopProcess(vite);
};

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
process.on("exit", shutdown);

await waitForServer(rendererUrl, {
  child: vite,
  label: "Renderer dev server",
});

electron = spawnElectronPreview(appRoot, rendererUrl);

electron.on("exit", (code) => {
  shutdown();
  process.exit(code ?? 0);
});

vite.on("exit", (code) => {
  if (code && code !== 0) {
    process.exit(code);
  }
});
