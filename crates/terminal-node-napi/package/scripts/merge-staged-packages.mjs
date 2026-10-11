#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, "..");
const crateDir = path.resolve(packageDir, "..");
const workspaceRoot = path.resolve(crateDir, "..", "..");
const rootFiles = ["package.json", "README.md", "index.cjs", "index.mjs", "index.d.ts", "index.d.mts"];

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const outDir = path.resolve(options.out);
  const stagedDirs = options.packages.map((value) => path.resolve(value));

  assertSafeOutputDir(outDir);

  const inputs = [];
  for (const stagedDir of stagedDirs) {
    inputs.push(await readStagedInput(stagedDir));
  }

  const packageVersion = inputs[0].manifest.packageVersion;
  const targets = [];
  const seenTargets = new Map();

  for (const input of inputs) {
    if (input.manifest.packageVersion !== packageVersion) {
      throw new Error(
        `Staged package ${input.dir} has packageVersion ${input.manifest.packageVersion}, expected ${packageVersion}`,
      );
    }

    for (const target of input.manifest.targets) {
      const key = targetKey(target);
      const existing = seenTargets.get(key);
      if (existing) {
        if (existing.file !== target.file) {
          throw new Error(
            `Conflicting native targets for ${key}: ${existing.file} vs ${target.file}`,
          );
        }
        continue;
      }

      seenTargets.set(key, target);
      targets.push(target);
    }
  }

  await fs.rm(outDir, { recursive: true, force: true });
  await fs.mkdir(outDir, { recursive: true });

  for (const file of rootFiles) {
    await copyIfPresent(inputs[0].dir, outDir, file);
  }

  await copyDirectory(path.join(inputs[0].dir, "bindings"), path.join(outDir, "bindings"));

  const nativeDir = path.join(outDir, "native");
  await fs.mkdir(nativeDir, { recursive: true });

  for (const input of inputs) {
    const nativeEntries = await fs.readdir(path.join(input.dir, "native"), { withFileTypes: true });

    for (const entry of nativeEntries) {
      if (!entry.isFile() || entry.name === "manifest.json") {
        continue;
      }

      const sourcePath = path.join(input.dir, "native", entry.name);
      const targetPath = path.join(nativeDir, entry.name);
      if (await exists(targetPath)) {
        if (await filesDiffer(sourcePath, targetPath)) {
          throw new Error(`Conflicting native file name across staged packages: ${entry.name}`);
        }
        continue;
      }

      await fs.copyFile(sourcePath, targetPath);
    }
  }

  await fs.writeFile(
    path.join(nativeDir, "manifest.json"),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        packageVersion,
        targets,
      },
      null,
      2,
    )}\n`,
  );

  process.stdout.write(
    `${JSON.stringify({
      outDir,
      packageVersion,
      stagedInputs: inputs.length,
      targets: targets.map((target) => targetKey(target)),
    })}\n`,
  );
}

async function readStagedInput(dir) {
  const manifestPath = path.join(dir, "native", "manifest.json");
  const manifest = JSON.parse(await fs.readFile(manifestPath, "utf8"));

  if (manifest.schemaVersion !== 1) {
    throw new Error(`Staged package ${dir} has unexpected manifest schemaVersion`);
  }
  if (!manifest.packageVersion) {
    throw new Error(`Staged package ${dir} is missing packageVersion in its manifest`);
  }
  if (!Array.isArray(manifest.targets) || manifest.targets.length === 0) {
    throw new Error(`Staged package ${dir} does not define any native targets`);
  }

  for (const target of manifest.targets) {
    if (!target.file) {
      throw new Error(`Staged package ${dir} has a target without a file name`);
    }
    await assertFile(path.join(dir, "native", target.file), `native target ${target.file} in ${dir}`);
  }

  return { dir, manifest };
}

function targetKey(target) {
  return [target.platform, target.arch, target.libc].filter(Boolean).join("-");
}

async function copyIfPresent(sourceDir, targetDir, file) {
  const sourcePath = path.join(sourceDir, file);

  if (await exists(sourcePath)) {
    await fs.copyFile(sourcePath, path.join(targetDir, file));
  }
}

async function copyDirectory(sourceDir, targetDir) {
  const stat = await fs.stat(sourceDir).catch(() => null);

  if (!stat?.isDirectory()) {
    throw new Error(`Missing directory: ${sourceDir}`);
  }

  await fs.mkdir(targetDir, { recursive: true });

  for (const entry of await fs.readdir(sourceDir, { withFileTypes: true })) {
    if (!entry.isFile()) {
      continue;
    }

    await fs.copyFile(path.join(sourceDir, entry.name), path.join(targetDir, entry.name));
  }
}

async function filesDiffer(left, right) {
  const [leftContent, rightContent] = await Promise.all([
    fs.readFile(left),
    fs.readFile(right),
  ]);

  return !leftContent.equals(rightContent);
}

function parseArgs(argv) {
  const options = {
    out: null,
    packages: [],
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--out") {
      options.out = readFlagValue(argv, index, arg);
      index += 1;
      continue;
    }

    if (arg === "--package") {
      options.packages.push(readFlagValue(argv, index, arg));
      index += 1;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
  }

  if (!options.out) {
    throw new Error("Missing required --out argument");
  }
  if (options.packages.length === 0) {
    throw new Error("Missing required --package arguments (repeat for each staged platform)");
  }

  return options;
}

function readFlagValue(argv, index, flag) {
  const value = argv[index + 1];

  if (!value || value.startsWith("--")) {
    throw new Error(`Missing value for ${flag}`);
  }

  return value;
}

function assertSafeOutputDir(outDir) {
  const root = path.parse(outDir).root;
  const blocked = new Set([root, workspaceRoot, crateDir, packageDir]);

  if (blocked.has(outDir)) {
    throw new Error(`Refusing to merge package into unsafe output directory: ${outDir}`);
  }
}

async function assertFile(value, label) {
  const stat = await fs.stat(value).catch(() => null);

  if (!stat?.isFile()) {
    throw new Error(`Missing ${label}: ${value}`);
  }
}

async function exists(value) {
  return (await fs.stat(value).catch(() => null)) != null;
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
