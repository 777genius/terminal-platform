import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  replaceDirectoryAtomically,
  withSiblingStagingDirectory,
} from "./replace-directory-atomically.mjs";

test("withSiblingStagingDirectory removes staging directories after failures", async () => {
  const rootDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-staging-test-"));
  const targetDir = path.join(rootDir, "raw");

  try {
    await assert.rejects(
      withSiblingStagingDirectory(targetDir, "generate", async (stagedDir) => {
        await fs.mkdir(stagedDir, { recursive: true });
        await fs.writeFile(path.join(stagedDir, "partial.ts"), "export {};\n");
        throw new Error("simulated generator failure");
      }),
      /simulated generator failure/,
    );

    assert.deepEqual(await listDirectory(rootDir), []);
  } finally {
    await fs.rm(rootDir, { recursive: true, force: true });
  }
});

test("withSiblingStagingDirectory does not remove atomically replaced targets", async () => {
  const rootDir = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-platform-staging-test-"));
  const targetDir = path.join(rootDir, "raw");

  try {
    await withSiblingStagingDirectory(targetDir, "generate", async (stagedDir) => {
      await fs.writeFile(path.join(stagedDir, "NodeSession.ts"), "export type NodeSession = {};\n");
      await replaceDirectoryAtomically(targetDir, stagedDir);
    });

    assert.deepEqual(await listDirectory(rootDir), ["raw"]);
    assert.equal(
      await fs.readFile(path.join(targetDir, "NodeSession.ts"), "utf8"),
      "export type NodeSession = {};\n",
    );
  } finally {
    await fs.rm(rootDir, { recursive: true, force: true });
  }
});

async function listDirectory(dir) {
  return (await fs.readdir(dir)).sort();
}
