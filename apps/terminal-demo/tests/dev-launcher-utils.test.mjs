import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import path from "node:path";

import {
  buildBrowserBootstrapConfigPaths,
  buildWindowsTaskkillArgs,
  buildViteDevServerArgs,
  resolveGracefulStopSignal,
  resolveSpawnCommand,
  waitForServer,
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

test("dev launcher resolves browser bootstrap cleanup paths", () => {
  const appRoot = process.platform === "win32"
    ? "C:\\Users\\User\\PROJECT_IT\\terminal-platform\\apps\\terminal-demo"
    : "/workspace/terminal-platform/apps/terminal-demo";

  assert.deepEqual(
    buildBrowserBootstrapConfigPaths(appRoot),
    [
      path.join(appRoot, "public", "terminal-runtime-bootstrap.json"),
      path.join(appRoot, "dist", "renderer", "terminal-runtime-bootstrap.json"),
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

test("dev launcher requests a catchable signal before force-killing Windows process trees", () => {
  assert.equal(resolveGracefulStopSignal("win32"), "SIGINT");
  assert.equal(resolveGracefulStopSignal("linux"), "SIGTERM");
  assert.deepEqual(buildWindowsTaskkillArgs(1234), ["/PID", "1234", "/T", "/F"]);
});

test("dev launcher rejects readiness from an already-running server when the child exits", async () => {
  const child = new EventEmitter();
  child.exitCode = null;
  child.signalCode = null;
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => {
    setTimeout(() => {
      child.exitCode = 1;
      child.emit("exit", 1, null);
    }, 0);

    return { ok: true };
  };

  try {
    await assert.rejects(
      () =>
        waitForServer("http://127.0.0.1:5173", {
          child,
          label: "Renderer dev server",
          readinessSettleMs: 25,
          timeoutMs: 1_000,
        }),
      /Renderer dev server exited after .* became ready - exit code 1/u,
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});
