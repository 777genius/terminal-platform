import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { clearStaleLockIfNeeded } from "./with-file-lock.mjs";

test("keeps fresh lock files whose metadata is not written yet", async () => {
  const rootDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-lock-test-"));
  const lockFile = path.join(rootDir, "stage-sdk.lock");

  try {
    await fs.writeFile(lockFile, "");

    await clearStaleLockIfNeeded(lockFile, 60_000);

    assert.equal(await pathExists(lockFile), true);
  } finally {
    await fs.rm(rootDir, { recursive: true, force: true });
  }
});

test("removes stale lock files when their owner metadata is unreadable", async () => {
  const rootDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-lock-test-"));
  const lockFile = path.join(rootDir, "stage-sdk.lock");

  try {
    await fs.writeFile(lockFile, "");
    const oldTime = new Date(Date.now() - 120_000);
    await fs.utimes(lockFile, oldTime, oldTime);

    await clearStaleLockIfNeeded(lockFile, 1_000);

    assert.equal(await pathExists(lockFile), false);
  } finally {
    await fs.rm(rootDir, { recursive: true, force: true });
  }
});

test("keeps live process locks even when their timestamp is old", async () => {
  const rootDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-lock-test-"));
  const lockFile = path.join(rootDir, "stage-sdk.lock");

  try {
    await fs.writeFile(
      lockFile,
      `${JSON.stringify({
        pid: process.pid,
        createdAt: new Date(Date.now() - 120_000).toISOString(),
      })}\n`,
    );

    await clearStaleLockIfNeeded(lockFile, 1_000);

    assert.equal(await pathExists(lockFile), true);
  } finally {
    await fs.rm(rootDir, { recursive: true, force: true });
  }
});

async function pathExists(filePath) {
  try {
    await fs.stat(filePath);
    return true;
  } catch {
    return false;
  }
}
