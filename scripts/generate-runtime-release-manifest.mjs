#!/usr/bin/env node

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const supportedPlatforms = {
  "darwin-arm64": {
    archiveKind: "tar.gz",
    binaryName: "terminal-daemon",
  },
  "darwin-x64": {
    archiveKind: "tar.gz",
    binaryName: "terminal-daemon",
  },
  "linux-x64": {
    archiveKind: "tar.gz",
    binaryName: "terminal-daemon",
  },
  "win32-x64": {
    archiveKind: "zip",
    binaryName: "terminal-daemon.exe",
  },
  "win32-arm64": {
    archiveKind: "zip",
    binaryName: "terminal-daemon.exe",
  },
};

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    printUsage();
    return;
  }

  const assetsDir = path.resolve(options.assetsDir);
  const assets = {};
  const checksumLines = [];

  for (const [platform, descriptor] of Object.entries(supportedPlatforms)) {
    const file = runtimeAssetName(platform, options.version, descriptor.archiveKind);
    const assetPath = findFile(assetsDir, file);
    if (!assetPath) {
      throw new Error(`Missing release asset for ${platform}: ${file}`);
    }

    const sha256 = hashFile(assetPath);
    assets[platform] = {
      file,
      archiveKind: descriptor.archiveKind,
      binaryName: descriptor.binaryName,
      packageDirName: "terminal-platform-node",
      payloadDirName: "terminal-platform",
      sha256,
    };
    checksumLines.push(`${sha256}  ${file}`);
  }

  const manifest = {
    schemaVersion: 1,
    version: options.version,
    sourceRepository: options.sourceRepository,
    sourceRef: options.sourceRef,
    sourceCommit: options.sourceCommit,
    releaseRepository: options.releaseRepository,
    releaseTag: options.releaseTag,
    generatedAt: new Date().toISOString(),
    assets,
  };

  fs.mkdirSync(path.dirname(options.out), { recursive: true });
  fs.writeFileSync(options.out, `${JSON.stringify(manifest, null, 2)}\n`);

  fs.mkdirSync(path.dirname(options.checksumsOut), { recursive: true });
  fs.writeFileSync(options.checksumsOut, `${checksumLines.sort().join("\n")}\n`);

  process.stdout.write(`${options.out}\n${options.checksumsOut}\n`);
}

function printUsage() {
  process.stdout.write(`Usage: node scripts/generate-runtime-release-manifest.mjs [options]

Options:
  --assets-dir <dir>          Directory containing runtime archives.
  --out <path>                Manifest output path.
  --checksums-out <path>      SHA256SUMS output path.
  --version <version>         Runtime artifact version.
  --source-repository <repo>  Source repository owner/name.
  --source-ref <ref>          Source ref used for build.
  --source-commit <sha>       Source commit used for build.
  --release-repository <repo> Release repository owner/name.
  --release-tag <tag>         Release tag containing assets.
  --help                      Show this message.
`);
}

function parseArgs(argv) {
  const options = {};

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    switch (arg) {
      case "--help":
      case "-h":
        options.help = true;
        break;
      case "--assets-dir":
        options.assetsDir = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--out":
        options.out = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--checksums-out":
        options.checksumsOut = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--version":
        options.version = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--source-repository":
        options.sourceRepository = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--source-ref":
        options.sourceRef = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--source-commit":
        options.sourceCommit = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--release-repository":
        options.releaseRepository = readFlagValue(argv, index, arg);
        index += 1;
        break;
      case "--release-tag":
        options.releaseTag = readFlagValue(argv, index, arg);
        index += 1;
        break;
      default:
        throw new Error(`Unsupported argument: ${arg}`);
    }
  }

  if (options.help) {
    return options;
  }

  for (const key of [
    "assetsDir",
    "out",
    "checksumsOut",
    "version",
    "sourceRepository",
    "sourceRef",
    "sourceCommit",
    "releaseRepository",
    "releaseTag",
  ]) {
    if (!options[key]) {
      throw new Error(`Missing required --${toKebabCase(key)} argument`);
    }
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

function toKebabCase(value) {
  return value.replace(/[A-Z]/gu, (match) => `-${match.toLowerCase()}`);
}

function runtimeAssetName(platform, version, archiveKind) {
  return `terminal-platform-runtime-${platform}-v${version}.${archiveKind}`;
}

function findFile(rootDir, fileName) {
  const queue = [rootDir];
  while (queue.length > 0) {
    const currentDir = queue.pop();
    if (!currentDir || !fs.existsSync(currentDir)) {
      continue;
    }

    for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
      const entryPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        queue.push(entryPath);
        continue;
      }
      if (entry.isFile() && entry.name === fileName) {
        return entryPath;
      }
    }
  }

  return null;
}

function hashFile(filePath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(filePath));
  return hash.digest("hex");
}

main();
