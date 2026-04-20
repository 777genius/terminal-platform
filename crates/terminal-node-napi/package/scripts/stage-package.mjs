#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, "..");
const crateManifestPath = path.resolve(packageDir, "..", "Cargo.toml");
const workspaceBindingsDir = path.resolve(packageDir, "..", "..", "terminal-node", "bindings");
const staticFiles = ["README.md", "index.cjs", "index.mjs", "index.d.ts"];

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const outDir = path.resolve(options.out);
  const addonPath = path.resolve(options.addon);
  const packageVersion = await readCrateVersion();

  await assertFile(crateManifestPath, "crate manifest");
  await assertFile(addonPath, "addon");
  await assertDirectory(workspaceBindingsDir, "bindings directory");

  await fs.rm(outDir, { recursive: true, force: true });
  await fs.mkdir(outDir, { recursive: true });

  for (const file of staticFiles) {
    await fs.copyFile(path.join(packageDir, file), path.join(outDir, file));
  }

  await stagePackageManifest(outDir, packageVersion);

  const bindingsOutDir = path.join(outDir, "bindings");
  await fs.mkdir(bindingsOutDir, { recursive: true });
  const bindings = await fs.readdir(workspaceBindingsDir, { withFileTypes: true });

  for (const binding of bindings) {
    if (!binding.isFile() || !binding.name.endsWith(".ts")) {
      continue;
    }

    const sourcePath = path.join(workspaceBindingsDir, binding.name);
    const targetPath = path.join(
      bindingsOutDir,
      binding.name.replace(/\.ts$/u, ".d.ts"),
    );
    await fs.copyFile(sourcePath, targetPath);
  }

  const nativeDir = path.join(outDir, "native");
  await fs.mkdir(nativeDir, { recursive: true });
  const targetDescriptor = currentTargetDescriptor(packageVersion);
  await fs.copyFile(addonPath, path.join(nativeDir, targetDescriptor.file));
  await fs.writeFile(
    path.join(nativeDir, "manifest.json"),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        packageVersion,
        targets: [targetDescriptor],
      },
      null,
      2,
    )}\n`,
  );

  process.stdout.write(`${outDir}\n`);
}

async function stagePackageManifest(outDir, packageVersion) {
  const templateManifestPath = path.join(packageDir, "package.json");
  const packageManifest = JSON.parse(await fs.readFile(templateManifestPath, "utf8"));
  packageManifest.version = packageVersion;
  delete packageManifest.scripts;
  await fs.writeFile(
    path.join(outDir, "package.json"),
    `${JSON.stringify(packageManifest, null, 2)}\n`,
  );
}

function parseArgs(argv) {
  const options = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--out") {
      options.out = argv[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--addon") {
      options.addon = argv[index + 1];
      index += 1;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
  }

  if (!options.out) {
    throw new Error("Missing required --out argument");
  }

  if (!options.addon) {
    throw new Error("Missing required --addon argument");
  }

  return options;
}

async function assertFile(value, label) {
  const stat = await fs.stat(value).catch(() => null);
  if (!stat || !stat.isFile()) {
    throw new Error(`Missing ${label}: ${value}`);
  }
}

async function assertDirectory(value, label) {
  const stat = await fs.stat(value).catch(() => null);
  if (!stat || !stat.isDirectory()) {
    throw new Error(`Missing ${label}: ${value}`);
  }
}

async function readCrateVersion() {
  const manifest = await fs.readFile(crateManifestPath, "utf8");
  const match = manifest.match(/^version\s*=\s*"(?<version>[^"]+)"/mu);

  if (!match?.groups?.version) {
    throw new Error(`Failed to resolve crate version from ${crateManifestPath}`);
  }

  return match.groups.version;
}

function currentTargetDescriptor(packageVersion) {
  const libc = detectLibc();

  return {
    platform: process.platform,
    arch: process.arch,
    libc,
    file: ["terminal_node_napi", process.platform, process.arch, libc]
      .filter(Boolean)
      .join(".") + ".node",
    packageVersion,
  };
}

function detectLibc() {
  if (process.platform !== "linux") {
    return null;
  }

  const report = process.report?.getReport?.();
  return report?.header?.glibcVersionRuntime ? "gnu" : "musl";
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
