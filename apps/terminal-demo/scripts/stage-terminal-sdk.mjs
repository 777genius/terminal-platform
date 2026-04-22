#!/usr/bin/env node

import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createSiblingStagingDirectory,
  replaceDirectoryAtomically,
} from "../../../scripts/node/replace-directory-atomically.mjs";
import { withFileLock } from "../../../scripts/node/with-file-lock.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const sdkRoot = path.resolve(repoRoot, "sdk");
const sdkPackagesRoot = path.resolve(sdkRoot, "packages");
const packageDir = path.resolve(repoRoot, "crates", "terminal-node-napi", "package");
const outDir = path.resolve(appRoot, ".generated", "terminal-platform-node");
const sdkScopeRoot = path.resolve(appRoot, "node_modules", "@terminal-platform");
const lockFile = path.resolve(appRoot, ".generated", "locks", "stage-sdk.lock");

await main();

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

async function main() {
  await withFileLock(lockFile, async () => {
    run("node", ["./scripts/build-local-package.mjs", "--out", outDir], packageDir);
    run("cargo", ["build", "-p", "terminal-daemon"], repoRoot);
    run("npm", ["run", "build"], sdkRoot);
    await linkSdkPackages();
  }, {
    metadata: {
      app: "terminal-demo",
      task: "stage-sdk",
    },
  });

  process.stdout.write(`${outDir}\n`);
}

async function linkSdkPackages() {
  await fs.mkdir(sdkScopeRoot, { recursive: true });
  const entries = await fs.readdir(sdkPackagesRoot, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const packageRoot = path.join(sdkPackagesRoot, entry.name);
    const packageJsonPath = path.join(packageRoot, "package.json");
    const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf8"));
    if (typeof packageJson.name !== "string" || !packageJson.name.startsWith("@terminal-platform/")) {
      continue;
    }

    const packageBasename = packageJson.name.slice("@terminal-platform/".length);
    const linkPath = path.join(sdkScopeRoot, packageBasename);
    const stagedDir = await createSiblingStagingDirectory(linkPath, "sdk-package");
    await fs.cp(packageRoot, stagedDir, { recursive: true });
    await replaceDirectoryAtomically(linkPath, stagedDir);
  }
}
