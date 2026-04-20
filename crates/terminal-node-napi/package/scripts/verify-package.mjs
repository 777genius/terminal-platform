#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, "..");

function main() {
  const options = parseArgs(process.argv.slice(2));
  const rootDir = path.resolve(options.packageDir);
  const packageJson = readJson(path.join(rootDir, "package.json"));
  const requiredFiles = [
    "README.md",
    "index.cjs",
    "index.mjs",
    "index.d.ts",
    path.join("native", "terminal_node_napi.node"),
  ];

  assertValue(packageJson.name === "terminal-platform-node", "Unexpected package name");
  assertValue(packageJson.version, "Missing package version");
  assertValue(packageJson.main === "./index.cjs", "Unexpected main entrypoint");
  assertValue(packageJson.module === "./index.mjs", "Unexpected module entrypoint");
  assertValue(packageJson.types === "./index.d.ts", "Unexpected types entrypoint");
  assertValue(!("scripts" in packageJson), "Published package should not include dev scripts");

  for (const relativePath of requiredFiles) {
    assertFile(path.join(rootDir, relativePath), `Missing required file: ${relativePath}`);
  }

  const bindingsDir = path.join(rootDir, "bindings");
  assertDirectory(bindingsDir, "Missing bindings directory");
  const bindingFiles = fs
    .readdirSync(bindingsDir, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith(".d.ts"));
  assertValue(bindingFiles.length > 0, "Bindings directory is empty");
  assertValue(
    bindingFiles.some((entry) => entry.name === "NodeHandshakeInfo.d.ts"),
    "Expected NodeHandshakeInfo.d.ts binding to exist",
  );

  process.stdout.write(
    JSON.stringify({
      packageDir: rootDir,
      version: packageJson.version,
      bindings: bindingFiles.length,
    }),
  );
}

function parseArgs(argv) {
  const options = {
    packageDir: path.join(packageDir, "artifacts", "local"),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--package-dir") {
      options.packageDir = argv[index + 1];
      index += 1;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
  }

  return options;
}

function readJson(value) {
  return JSON.parse(fs.readFileSync(value, "utf8"));
}

function assertFile(value, message) {
  const stat = tryStat(value);
  assertValue(stat?.isFile(), message);
}

function assertDirectory(value, message) {
  const stat = tryStat(value);
  assertValue(stat?.isDirectory(), message);
}

function tryStat(value) {
  try {
    return fs.statSync(value);
  } catch {
    return null;
  }
}

function assertValue(value, message) {
  if (!value) {
    throw new Error(message);
  }
}

main();
