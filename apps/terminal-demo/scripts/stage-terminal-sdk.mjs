#!/usr/bin/env node

import fs from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  replaceDirectoryAtomically,
  withSiblingStagingDirectory,
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
const typescriptCliPath = path.resolve(sdkRoot, "node_modules", "typescript", "bin", "tsc");
const workspaceSdkProjectRefs = [
  "packages/foundation/tsconfig.json",
  "packages/runtime-types/tsconfig.json",
  "packages/design-tokens/tsconfig.json",
  "packages/workspace-contracts/tsconfig.json",
  "packages/workspace-core/tsconfig.json",
  "packages/workspace-adapter-websocket/tsconfig.json",
  "packages/workspace-adapter-preload/tsconfig.json",
  "packages/workspace-adapter-memory/tsconfig.json",
  "packages/workspace-elements/tsconfig.json",
  "packages/workspace-react/tsconfig.json",
  "packages/testing/tsconfig.json",
];
const workspaceOnly = process.argv.includes("--workspace-only");

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
    buildWorkspaceSdkPackages();
    await linkSdkPackages();
    if (workspaceOnly) {
      return;
    }

    run("node", ["./scripts/build-local-package.mjs", "--out", outDir], packageDir);
    run("cargo", ["build", "-p", "terminal-daemon"], repoRoot);
  }, {
    metadata: {
      app: "terminal-demo",
      task: "stage-sdk",
    },
  });

  process.stdout.write(`${workspaceOnly ? sdkScopeRoot : outDir}\n`);
}

function buildWorkspaceSdkPackages() {
  run(process.execPath, [typescriptCliPath, "-b", ...workspaceSdkProjectRefs], sdkRoot);
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
    await withSiblingStagingDirectory(linkPath, "sdk-package", async (stagedDir) => {
      await copyDirectoryStable(packageRoot, stagedDir);
      await replaceDirectoryAtomically(linkPath, stagedDir);
    });
  }
}

async function copyDirectoryStable(sourceDir, targetDir) {
  await fs.mkdir(targetDir, { recursive: true });

  let entries;
  try {
    entries = await fs.readdir(sourceDir, { withFileTypes: true });
  } catch (error) {
    if (isMissingPathError(error)) {
      return;
    }
    throw error;
  }

  for (const entry of entries) {
    if (shouldSkipPackageArtifact(entry.name)) {
      continue;
    }

    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);

    if (entry.isDirectory()) {
      await copyDirectoryStable(sourcePath, targetPath);
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    try {
      await fs.copyFile(sourcePath, targetPath);
    } catch (error) {
      if (isMissingPathError(error)) {
        continue;
      }
      throw error;
    }
  }
}

function shouldSkipPackageArtifact(name) {
  return (
    name.endsWith(".lock")
    || name.includes(".generate.")
    || name.includes(".stage.")
    || name === ".DS_Store"
    || name.endsWith(".tsbuildinfo")
  );
}

function isMissingPathError(error) {
  return Boolean(error) && typeof error === "object" && "code" in error && error.code === "ENOENT";
}
