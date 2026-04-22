#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { statSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createSiblingStagingDirectory, replaceDirectoryAtomically } from "../../../../scripts/node/replace-directory-atomically.mjs";
import { withFileLock } from "../../../../scripts/node/with-file-lock.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, "..");
const crateDir = path.resolve(packageDir, "..");
const workspaceRoot = path.resolve(crateDir, "..", "..");

function main() {
  const options = parseArgs(process.argv.slice(2));
  return withFileLock(`${path.resolve(options.out)}.lock`, async () => {
    const cargoArgs = ["build", "-p", "terminal-node-napi"];

    if (options.release) {
      cargoArgs.push("--release");
    }

    run("cargo", cargoArgs, workspaceRoot);

    const addonPath = locateAddon(options.release ? "release" : "debug");
    const stagedOutDir = await createSiblingStagingDirectory(options.out, "stage");
    run("node", ["./scripts/stage-package.mjs", "--out", stagedOutDir, "--addon", addonPath], packageDir);
    await fs.copyFile(
      path.join(stagedOutDir, "index.d.ts"),
      path.join(stagedOutDir, "index.d.mts"),
    );
    await replaceDirectoryAtomically(options.out, stagedOutDir);

    process.stdout.write(`${path.resolve(options.out)}\n`);
  }, {
    metadata: {
      script: "build-local-package",
      outDir: path.resolve(options.out),
    },
  });
}

function parseArgs(argv) {
  const options = {
    out: path.join(packageDir, "artifacts", "local"),
    release: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--out") {
      options.out = argv[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--release") {
      options.release = true;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
  }

  return options;
}

function locateAddon(profile) {
  const targetDir = path.join(workspaceRoot, "target", profile);

  for (const name of candidateAddonNames()) {
    const candidate = path.join(targetDir, name);
    if (isFile(candidate)) {
      return candidate;
    }
  }

  throw new Error(`Failed to locate built addon in ${targetDir}`);
}

function candidateAddonNames() {
  switch (process.platform) {
    case "darwin":
      return ["libterminal_node_napi.dylib"];
    case "win32":
      return ["terminal_node_napi.dll"];
    default:
      return ["libterminal_node_napi.so"];
  }
}

function isFile(value) {
  try {
    return statSync(value).isFile();
  } catch {
    return false;
  }
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

await main();
