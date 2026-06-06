import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { withTerminalDemoSmokeLock } from "../scripts/smoke-lock.mjs";

test("browser smoke lock serializes concurrent smoke runners", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-smoke-lock-test-"));
  const lockDir = path.join(tempRoot, "lock");
  const events = [];
  let allowFirstToFinish = () => {};
  const firstCanFinish = new Promise((resolve) => {
    allowFirstToFinish = resolve;
  });
  let markFirstStarted = () => {};
  const firstStarted = new Promise((resolve) => {
    markFirstStarted = resolve;
  });

  try {
    const first = withTerminalDemoSmokeLock(
      "first",
      async () => {
        events.push("first:start");
        markFirstStarted();
        await firstCanFinish;
        events.push("first:end");
      },
      { lockDir, timeoutMs: 1_000 },
    );
    await firstStarted;

    const second = withTerminalDemoSmokeLock(
      "second",
      async () => {
        events.push("second:start");
      },
      { lockDir, timeoutMs: 2_000 },
    );
    await sleep(75);
    assert.deepEqual(events, ["first:start"]);
    allowFirstToFinish();
    await Promise.all([first, second]);

    assert.deepEqual(events, ["first:start", "first:end", "second:start"]);
  } finally {
    await fs.rm(tempRoot, { recursive: true, force: true });
  }
});

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
