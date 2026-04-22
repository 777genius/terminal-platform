#!/usr/bin/env node

import { spawn, spawnSync } from "node:child_process";
import process from "node:process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const rendererPort = process.env.TERMINAL_DEMO_RENDERER_PORT ?? "5173";
const rendererUrl = `http://127.0.0.1:${rendererPort}`;

runSync("npm", ["run", "stage:sdk"], appRoot);
runSync("npm", ["run", "build:host"], appRoot);

const vite = spawn("npx", ["vite", "--host", "127.0.0.1", "--port", rendererPort], {
  cwd: appRoot,
  env: process.env,
  stdio: "inherit",
});

let electron = null;
const shutdown = () => {
  if (electron && !electron.killed) {
    electron.kill("SIGTERM");
  }
  if (!vite.killed) {
    vite.kill("SIGTERM");
  }
};

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
process.on("exit", shutdown);

await waitForServer(rendererUrl);

electron = spawn(
  "npx",
  ["electron", "./dist/host/main/index.js"],
  {
    cwd: appRoot,
    env: {
      ...process.env,
      TERMINAL_DEMO_RENDERER_URL: rendererUrl,
    },
    stdio: "inherit",
  },
);

electron.on("exit", (code) => {
  shutdown();
  process.exit(code ?? 0);
});

vite.on("exit", (code) => {
  if (code && code !== 0) {
    process.exit(code);
  }
});

function runSync(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

async function waitForServer(url) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < 20_000) {
    try {
      const response = await fetch(url, { method: "GET" });
      if (response.ok) {
        return;
      }
    } catch {
      // Server is still starting.
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Timed out waiting for renderer dev server at ${url}`);
}
