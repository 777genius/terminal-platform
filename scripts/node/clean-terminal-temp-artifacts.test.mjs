import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  cleanTerminalTempArtifacts,
  collectTerminalTempArtifacts,
  isKnownTerminalTempArtifactName,
} from "./clean-terminal-temp-artifacts.mjs";

test("matches only known Terminal Platform temp artifact names", () => {
  assert.equal(isKnownTerminalTempArtifactName("terminal-capi-package-123"), true);
  assert.equal(isKnownTerminalTempArtifactName("terminal-demo-browser-smoke-profile-123"), true);
  assert.equal(isKnownTerminalTempArtifactName("terminal-node-addon-123"), true);
  assert.equal(isKnownTerminalTempArtifactName("terminal-runtime-store-123"), true);
  assert.equal(isKnownTerminalTempArtifactName("node-compile-cache"), false);
  assert.equal(isKnownTerminalTempArtifactName("my-terminal-notes"), false);
});

test("dry-run reports direct known artifacts without deleting them", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-clean-test-"));

  try {
    await fs.mkdir(path.join(tmpDir, "terminal-capi-package-1"));
    await fs.writeFile(path.join(tmpDir, "terminal-demo-browser-smoke-1.png"), "png");
    await fs.mkdir(path.join(tmpDir, "unrelated-cache"));

    const results = await cleanTerminalTempArtifacts({
      checkOpenFiles: false,
      tmpDir,
    });

    assert.deepEqual(results.map((item) => [item.name, item.status]), [
      ["terminal-capi-package-1", "dry-run"],
      ["terminal-demo-browser-smoke-1.png", "dry-run"],
    ]);
    assert.deepEqual(await listDirectory(tmpDir), [
      "terminal-capi-package-1",
      "terminal-demo-browser-smoke-1.png",
      "unrelated-cache",
    ]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test("apply deletes known artifacts and leaves unrelated paths", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-clean-test-"));

  try {
    await fs.mkdir(path.join(tmpDir, "terminal-node-package-1"));
    await fs.writeFile(path.join(tmpDir, "terminal-runtime-log-1"), "log");
    await fs.mkdir(path.join(tmpDir, "node-compile-cache"));

    const results = await cleanTerminalTempArtifacts({
      apply: true,
      checkOpenFiles: false,
      tmpDir,
    });

    assert.deepEqual(results.map((item) => [item.name, item.status]), [
      ["terminal-node-package-1", "deleted"],
      ["terminal-runtime-log-1", "deleted"],
    ]);
    assert.deepEqual(await listDirectory(tmpDir), ["node-compile-cache"]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test("apply skips artifacts reported as open by lsof", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-clean-test-"));

  try {
    await fs.mkdir(path.join(tmpDir, "terminal-capi-prefix-1"));

    const results = await cleanTerminalTempArtifacts({
      apply: true,
      openFileChecker: () => ({ open: true }),
      tmpDir,
    });

    assert.deepEqual(results.map((item) => [item.name, item.status, item.reason]), [
      ["terminal-capi-prefix-1", "skipped", "open-files"],
    ]);
    assert.deepEqual(await listDirectory(tmpDir), ["terminal-capi-prefix-1"]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

test("collect skips symlinks even when their names match", async () => {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-clean-test-"));

  try {
    await fs.writeFile(path.join(tmpDir, "target-file"), "x");
    await fs.symlink(path.join(tmpDir, "target-file"), path.join(tmpDir, "terminal-demo-link-1"));

    const results = await collectTerminalTempArtifacts({ tmpDir });

    assert.deepEqual(results.map((item) => [item.name, item.status, item.reason]), [
      ["terminal-demo-link-1", "skipped", "symlink"],
    ]);
  } finally {
    await fs.rm(tmpDir, { recursive: true, force: true });
  }
});

async function listDirectory(dir) {
  return (await fs.readdir(dir)).sort();
}
