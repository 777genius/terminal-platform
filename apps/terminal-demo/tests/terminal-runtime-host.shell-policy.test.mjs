import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  resolveDemoDefaultShellProgram,
  resolveDemoDefaultWorkingDirectory,
} from "../dist/renderer-node-test/features/terminal-runtime-host/main/composition/shell-policy.js";

test("Windows shell policy skips missing implicit ComSpec paths when validation is enabled", () => {
  const missingShell = path.join(os.tmpdir(), `terminal-demo-missing-${process.pid}.exe`);

  assert.equal(
    resolveDemoDefaultShellProgram({
      env: { ComSpec: missingShell },
      platform: "win32",
      validateWindowsPaths: true,
    }),
    "cmd.exe",
  );
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: { ComSpec: process.execPath },
      platform: "win32",
      validateWindowsPaths: true,
    }),
    process.execPath,
  );
});

test("Windows shell policy preserves explicit shell overrides even when validation is enabled", () => {
  assert.equal(
    resolveDemoDefaultShellProgram({
      env: {
        ComSpec: path.join(os.tmpdir(), `terminal-demo-missing-${process.pid}.exe`),
        TERMINAL_DEMO_DEFAULT_SHELL: "pwsh.exe",
      },
      platform: "win32",
      validateWindowsPaths: true,
    }),
    "pwsh.exe",
  );
});

test("working directory policy falls back when an explicit cwd no longer exists", async () => {
  const existingCwd = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-cwd-policy-"));
  try {
    assert.equal(
      resolveDemoDefaultWorkingDirectory({
        cwd: existingCwd,
        env: { TERMINAL_DEMO_DEFAULT_CWD: path.join(existingCwd, "missing") },
        validateExists: true,
      }),
      path.resolve(existingCwd),
    );
  } finally {
    await fs.rm(existingCwd, { recursive: true, force: true });
  }
});
