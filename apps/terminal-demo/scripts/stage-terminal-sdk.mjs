#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const packageDir = path.resolve(repoRoot, "crates", "terminal-node-napi", "package");
const outDir = path.resolve(appRoot, ".generated", "terminal-platform-node");
const sdkRoot = path.resolve(repoRoot, "sdk");
const sdkPackagesDir = path.resolve(sdkRoot, "packages");
const appNodeModulesDir = path.resolve(appRoot, "node_modules");
const terminalPlatformNodeModulesDir = path.resolve(appNodeModulesDir, "@terminal-platform");

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
  run("npm", ["run", "build"], sdkRoot);
  await linkWorkspaceSdkPackages();
  run("node", ["./scripts/build-local-package.mjs", "--out", outDir], packageDir);
  run("cargo", ["build", "-p", "terminal-daemon"], repoRoot);
  await fs.copyFile(
    path.join(outDir, "index.d.ts"),
    path.join(outDir, "index.d.mts"),
  );
  process.stdout.write(`${outDir}\n`);
}

async function linkWorkspaceSdkPackages() {
  await fs.mkdir(terminalPlatformNodeModulesDir, { recursive: true });
  const entries = await fs.readdir(sdkPackagesDir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }

    const sourceDir = path.join(sdkPackagesDir, entry.name);
    const packageJsonPath = path.join(sourceDir, "package.json");
    const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf8"));
    const packageName = packageJson.name;
    if (typeof packageName !== "string" || !packageName.startsWith("@terminal-platform/")) {
      continue;
    }

    const targetDir = path.join(
      appNodeModulesDir,
      ...packageName.split("/"),
    );

    await fs.rm(targetDir, {
      recursive: true,
      force: true,
    });
    await fs.mkdir(path.dirname(targetDir), { recursive: true });
    await fs.symlink(sourceDir, targetDir, "dir");
  }
}
