import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { pathToFileURL } from "node:url";

import {
  resolveDaemonBinaryPath,
  resolveDaemonRepoRoot,
} from "../dist/features/terminal-runtime-host/main/infrastructure/DaemonSupervisor.js";

test("daemon supervisor resolves repo root from module location instead of process cwd", () => {
  const repoRoot = path.resolve("tmp", "repo-root");
  const moduleUrl = pathToFileURL(path.join(
    repoRoot,
    "apps",
    "terminal-demo",
    "dist",
    "features",
    "terminal-runtime-host",
    "main",
    "infrastructure",
    "DaemonSupervisor.js",
  )).href;

  assert.equal(resolveDaemonRepoRoot({ env: {}, moduleUrl }), repoRoot);
  assert.equal(
    resolveDaemonBinaryPath({ env: {}, moduleUrl, platform: "win32" }),
    path.join(repoRoot, "target", "debug", "terminal-daemon.exe"),
  );
});

test("daemon supervisor accepts an explicit repo root override for packaged Windows launchers", () => {
  const repoRoot = path.resolve("tmp", "explicit-root");

  assert.equal(
    resolveDaemonRepoRoot({
      env: { TERMINAL_DEMO_REPO_ROOT: ` ${repoRoot} ` },
      moduleUrl: pathToFileURL(path.resolve("elsewhere", "DaemonSupervisor.js")).href,
    }),
    repoRoot,
  );
});
