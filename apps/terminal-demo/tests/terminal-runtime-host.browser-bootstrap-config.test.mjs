import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  buildBrowserBootstrapPayload,
  clearBrowserBootstrapConfig,
  normalizeBrowserBootstrapScope,
  resolveBrowserBootstrapTargets,
  writeBrowserBootstrapConfig,
} from "../dist/host/browser/browser-bootstrap-config.js";

const bootstrapConfig = {
  controlPlaneUrl: "ws://127.0.0.1:4100/terminal-gateway/control?token=test",
  demoDefaultShellProgram: "C:\\Windows\\system32\\cmd.exe",
  demoDefaultWorkingDirectory: "C:\\Users\\User\\PROJECT_IT\\terminal-platform",
  runtimeSlug: "terminal-demo",
  sessionStreamUrl: "ws://127.0.0.1:4100/terminal-gateway/stream?token=test",
};

test("browser bootstrap target scopes resolve public and dist files deterministically", () => {
  const appRoot = path.resolve("tmp", "terminal-demo");
  const relativeTarget = "terminal-runtime-bootstrap.json";

  assert.deepEqual(
    resolveBrowserBootstrapTargets({ appRoot, relativeTarget, scope: "public-and-dist" }),
    [
      path.join(appRoot, "public", relativeTarget),
      path.join(appRoot, "dist", "renderer", relativeTarget),
    ],
  );
  assert.deepEqual(
    resolveBrowserBootstrapTargets({ appRoot, relativeTarget, scope: "public-only" }),
    [path.join(appRoot, "public", relativeTarget)],
  );
  assert.deepEqual(
    resolveBrowserBootstrapTargets({ appRoot, relativeTarget, scope: "dist-only" }),
    [path.join(appRoot, "dist", "renderer", relativeTarget)],
  );
  assert.equal(normalizeBrowserBootstrapScope("typo"), "public-and-dist");
});

test("browser bootstrap payload is stable JSON with a trailing newline", () => {
  const payload = buildBrowserBootstrapPayload(bootstrapConfig);

  assert.equal(payload.endsWith("\n"), true);
  assert.deepEqual(JSON.parse(payload), bootstrapConfig);
});

test("browser bootstrap writer updates and clears public and dist bootstrap files", async () => {
  const appRoot = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-bootstrap-test-"));
  try {
    await writeBrowserBootstrapConfig({
      appRoot,
      config: bootstrapConfig,
      scope: "public-and-dist",
    });

    const publicPath = path.join(appRoot, "public", "terminal-runtime-bootstrap.json");
    const distPath = path.join(appRoot, "dist", "renderer", "terminal-runtime-bootstrap.json");
    assert.deepEqual(JSON.parse(await fs.readFile(publicPath, "utf8")), bootstrapConfig);
    assert.deepEqual(JSON.parse(await fs.readFile(distPath, "utf8")), bootstrapConfig);

    await clearBrowserBootstrapConfig({
      appRoot,
      scope: "public-and-dist",
    });

    await assert.rejects(() => fs.access(publicPath), { code: "ENOENT" });
    await assert.rejects(() => fs.access(distPath), { code: "ENOENT" });
  } finally {
    await fs.rm(appRoot, { recursive: true, force: true });
  }
});

test("browser bootstrap clear keeps newer configs owned by another browser host", async () => {
  const appRoot = await fs.mkdtemp(path.join(os.tmpdir(), "terminal-demo-bootstrap-owner-test-"));
  const newerConfig = {
    ...bootstrapConfig,
    controlPlaneUrl: "ws://127.0.0.1:4200/terminal-gateway/control?token=newer",
    sessionStreamUrl: "ws://127.0.0.1:4200/terminal-gateway/stream?token=newer",
  };

  try {
    await writeBrowserBootstrapConfig({
      appRoot,
      config: newerConfig,
      scope: "public-and-dist",
    });

    await clearBrowserBootstrapConfig({
      appRoot,
      expectedConfig: bootstrapConfig,
      scope: "public-and-dist",
    });

    const publicPath = path.join(appRoot, "public", "terminal-runtime-bootstrap.json");
    const distPath = path.join(appRoot, "dist", "renderer", "terminal-runtime-bootstrap.json");
    assert.deepEqual(JSON.parse(await fs.readFile(publicPath, "utf8")), newerConfig);
    assert.deepEqual(JSON.parse(await fs.readFile(distPath, "utf8")), newerConfig);

    await clearBrowserBootstrapConfig({
      appRoot,
      expectedConfig: newerConfig,
      scope: "public-and-dist",
    });

    await assert.rejects(() => fs.access(publicPath), { code: "ENOENT" });
    await assert.rejects(() => fs.access(distPath), { code: "ENOENT" });
  } finally {
    await fs.rm(appRoot, { recursive: true, force: true });
  }
});
