import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

import {
  resolveDaemonBinaryPath,
  resolveDaemonRepoRoot,
} from "../dist/features/terminal-runtime-host/main/infrastructure/DaemonSupervisor.js";
import {
  cleanupStaleWindowsRuntimeTempDirs,
  collectStaleWindowsRuntimeTempDirs,
} from "../dist/features/terminal-runtime-host/main/infrastructure/windows-runtime-temp.js";

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

test("daemon supervisor collects only old runtime temp dirs owned by dead Windows PIDs", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-runtime-temp-test-"));
  const nowMs = Date.UTC(2026, 0, 1, 12, 0, 0);

  try {
    await createRuntimeTempDir(tmpDir, "terminal-demo-daemon-runtime-111111-AbCd12", nowMs - 3_600_000);
    await createRuntimeTempDir(tmpDir, "terminal-demo-sdk-runtime-222222-EfGh34", nowMs - 3_600_000);
    await createRuntimeTempDir(tmpDir, "terminal-demo-daemon-runtime-333333-IjKl56", nowMs - 3_600_000);
    await createRuntimeTempDir(tmpDir, "terminal-demo-sdk-runtime-444444-MnOp78", nowMs - 10_000);
    await fs.mkdir(path.join(tmpDir, "terminal-demo-daemon-runtime-not-a-pid-AbCd12"));
    await fs.mkdir(path.join(tmpDir, "unrelated"));

    const staleDirs = await collectStaleWindowsRuntimeTempDirs({
      currentPid: 333333,
      isPidRunning: (pid) => pid === 222222,
      minAgeMs: 60_000,
      nowMs,
      platform: "win32",
      tmpDir,
    });

    assert.deepEqual(staleDirs.map((dir) => [dir.kind, dir.name, dir.ownerPid]), [
      ["daemon", "terminal-demo-daemon-runtime-111111-AbCd12", 111111],
    ]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test("daemon supervisor deletes stale Windows runtime temp dirs without touching active owners", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-runtime-temp-test-"));
  const nowMs = Date.UTC(2026, 0, 1, 12, 0, 0);

  try {
    await createRuntimeTempDir(tmpDir, "terminal-demo-daemon-runtime-111111-AbCd12", nowMs - 3_600_000);
    await createRuntimeTempDir(tmpDir, "terminal-demo-sdk-runtime-222222-EfGh34", nowMs - 3_600_000);
    await fs.mkdir(path.join(tmpDir, "unrelated"));

    const results = await cleanupStaleWindowsRuntimeTempDirs({
      isPidRunning: (pid) => pid === 222222,
      minAgeMs: 60_000,
      nowMs,
      platform: "win32",
      tmpDir,
    });

    assert.deepEqual(results.map((item) => [item.name, item.status]), [
      ["terminal-demo-daemon-runtime-111111-AbCd12", "deleted"],
    ]);
    assert.deepEqual(await listDirectory(tmpDir), [
      "terminal-demo-sdk-runtime-222222-EfGh34",
      "unrelated",
    ]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

async function createRuntimeTempDir(tmpDir, name, mtimeMs) {
  const dir = path.join(tmpDir, name);
  await fs.mkdir(dir);
  await fs.writeFile(path.join(dir, "runtime.bin"), "x");
  const time = new Date(mtimeMs);
  await fs.utimes(dir, time, time);
  return dir;
}

async function listDirectory(dir) {
  return (await fs.readdir(dir)).sort();
}
