import test from "node:test";
import assert from "node:assert/strict";

import {
  buildViteDevServerArgs,
  resolveSpawnCommand,
} from "../scripts/dev-launcher-utils.mjs";

test("dev launcher keeps the renderer on the requested port", () => {
  assert.deepEqual(
    buildViteDevServerArgs("vite.js", "5173"),
    [
      "vite.js",
      "--force",
      "--host",
      "127.0.0.1",
      "--port",
      "5173",
      "--strictPort",
    ],
  );
});

test("dev launcher resolves npm through node on Windows when npm_execpath is available", () => {
  if (process.platform !== "win32") {
    return;
  }

  const npmExecPath = "C:\\Program Files\\nodejs\\node_modules\\npm\\bin\\npm-cli.js";
  assert.deepEqual(
    resolveSpawnCommand("npm", ["run", "build"], { npm_execpath: npmExecPath }),
    {
      args: [npmExecPath, "run", "build"],
      command: process.execPath,
      shell: false,
    },
  );
});
