#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const TERMINAL_TEMP_ARTIFACT_PATTERNS = [
  /^terminal-capi-.+/,
  /^terminal-demo-.+/,
  /^terminal-node-.+/,
  /^terminal-platform-.+/,
  /^terminal-runtime-.+/,
];

export function isKnownTerminalTempArtifactName(name) {
  return TERMINAL_TEMP_ARTIFACT_PATTERNS.some((pattern) => pattern.test(name));
}

export async function collectTerminalTempArtifacts(options = {}) {
  const tmpDir = path.resolve(options.tmpDir ?? os.tmpdir());
  const minAgeMs = options.minAgeMs ?? 0;
  const nowMs = options.nowMs ?? Date.now();
  const entries = await fs.readdir(tmpDir, { withFileTypes: true });
  const artifacts = [];

  for (const entry of entries) {
    if (!isKnownTerminalTempArtifactName(entry.name)) {
      continue;
    }

    const artifactPath = path.join(tmpDir, entry.name);
    const resolvedPath = path.resolve(artifactPath);
    if (path.dirname(resolvedPath) !== tmpDir) {
      continue;
    }

    const stat = await fs.lstat(resolvedPath);
    if (stat.isSymbolicLink()) {
      artifacts.push({
        name: entry.name,
        path: resolvedPath,
        reason: "symlink",
        status: "skipped",
      });
      continue;
    }

    const ageMs = Math.max(0, nowMs - stat.mtimeMs);
    if (ageMs < minAgeMs) {
      artifacts.push({
        ageMs,
        kind: kindFromStat(stat),
        name: entry.name,
        path: resolvedPath,
        reason: "too-recent",
        status: "skipped",
      });
      continue;
    }

    artifacts.push({
      ageMs,
      kind: kindFromStat(stat),
      name: entry.name,
      path: resolvedPath,
      status: "candidate",
    });
  }

  return artifacts.sort((left, right) => left.name.localeCompare(right.name));
}

export async function cleanTerminalTempArtifacts(options = {}) {
  const {
    apply = false,
    checkOpenFiles = true,
    openFileChecker = checkOpenFilesWithPlatformDefault,
  } = options;
  const artifacts = await collectTerminalTempArtifacts(options);
  const results = [];

  for (const artifact of artifacts) {
    if (artifact.status === "skipped") {
      results.push(artifact);
      continue;
    }

    if (checkOpenFiles) {
      const openFiles = openFileChecker(artifact);
      if (openFiles.open) {
        results.push({
          ...artifact,
          reason: "open-files",
          status: "skipped",
        });
        continue;
      }

      if (openFiles.error) {
        results.push({
          ...artifact,
          error: openFiles.error,
          reason: openFiles.reason ?? "open-file-check-error",
          status: "skipped",
        });
        continue;
      }
    }

    if (!apply) {
      results.push({
        ...artifact,
        status: "dry-run",
      });
      continue;
    }

    try {
      await fs.rm(artifact.path, { recursive: true, force: true });
      results.push({
        ...artifact,
        status: "deleted",
      });
    } catch (error) {
      results.push({
        ...artifact,
        error: error instanceof Error ? error.message : String(error),
        status: "failed",
      });
    }
  }

  return results;
}

export function checkOpenFilesWithLsof(artifact) {
  const args = artifact.kind === "directory"
    ? ["-nP", "+D", artifact.path]
    : ["-nP", artifact.path];
  const result = spawnSync("lsof", args, {
    encoding: "utf8",
  });

  if (result.error) {
    return { error: result.error.message, reason: "lsof-error", open: false };
  }

  if (result.status === 0) {
    return { open: true };
  }

  if (result.status === 1) {
    return { open: false };
  }

  return {
    error: result.stderr.trim() || `lsof exited with ${result.status}`,
    reason: "lsof-error",
    open: false,
  };
}

export function checkOpenFilesWithPlatformDefault(artifact) {
  if (process.platform === "win32") {
    return checkOpenFilesWithWindowsProcesses(artifact);
  }

  return checkOpenFilesWithLsof(artifact);
}

let windowsOpenPathSnapshot = null;

export function checkOpenFilesWithWindowsProcesses(artifact) {
  const snapshot = windowsOpenPaths();
  if (snapshot.error) {
    return {
      error: snapshot.error,
      reason: "windows-open-file-check-error",
      open: false,
    };
  }

  const artifactPath = normalizeWindowsPath(artifact.path);
  const artifactDirectoryPrefix = `${artifactPath}${path.sep.toLowerCase()}`;
  const open = snapshot.paths.some((openPath) => {
    if (artifact.kind === "directory") {
      return openPath === artifactPath || openPath.includes(artifactDirectoryPrefix);
    }

    return openPath.includes(artifactPath);
  });

  return { open };
}

function windowsOpenPaths() {
  if (windowsOpenPathSnapshot) {
    return windowsOpenPathSnapshot;
  }

  const result = spawnSync(windowsPowerShellPath(), [
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    [
      "$ErrorActionPreference = 'SilentlyContinue'",
      "$items = New-Object 'System.Collections.Generic.List[string]'",
      "Get-CimInstance Win32_Process | ForEach-Object { if ($_.CommandLine) { $items.Add($_.CommandLine) } }",
      "Get-Process | ForEach-Object { try { $_.Modules | ForEach-Object { if ($_.FileName) { $items.Add($_.FileName) } } } catch {} }",
      "$items | ConvertTo-Json -Compress",
    ].join("; "),
  ], {
    encoding: "utf8",
    windowsHide: true,
  });

  if (result.error) {
    windowsOpenPathSnapshot = {
      error: result.error.message,
      paths: [],
    };
    return windowsOpenPathSnapshot;
  }

  if (result.status !== 0) {
    windowsOpenPathSnapshot = {
      error: result.stderr.trim() || `PowerShell exited with ${result.status}`,
      paths: [],
    };
    return windowsOpenPathSnapshot;
  }

  try {
    const parsed = result.stdout.trim() ? JSON.parse(result.stdout) : [];
    const values = Array.isArray(parsed) ? parsed : [parsed];
    windowsOpenPathSnapshot = {
      paths: values
        .filter((value) => typeof value === "string")
        .map(normalizeWindowsPath),
    };
  } catch (error) {
    windowsOpenPathSnapshot = {
      error: error instanceof Error ? error.message : String(error),
      paths: [],
    };
  }

  return windowsOpenPathSnapshot;
}

function normalizeWindowsPath(value) {
  return String(value).replace(/\//gu, "\\").toLowerCase();
}

function windowsPowerShellPath() {
  const windowsRoot = process.env.SystemRoot || process.env.WINDIR || "C:\\Windows";
  return path.join(windowsRoot, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
}

function kindFromStat(stat) {
  if (stat.isDirectory()) {
    return "directory";
  }
  if (stat.isFile()) {
    return "file";
  }
  return "other";
}

function parseArgs(argv) {
  const options = {
    apply: false,
    checkOpenFiles: true,
    minAgeMs: 0,
    tmpDir: os.tmpdir(),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--apply") {
      options.apply = true;
      continue;
    }

    if (arg === "--no-lsof" || arg === "--no-open-file-check") {
      options.checkOpenFiles = false;
      continue;
    }

    if (arg === "--tmp-dir") {
      options.tmpDir = readFlagValue(argv, index, arg);
      index += 1;
      continue;
    }

    if (arg === "--min-age-minutes") {
      options.minAgeMs = Number(readFlagValue(argv, index, arg)) * 60_000;
      if (!Number.isFinite(options.minAgeMs) || options.minAgeMs < 0) {
        throw new Error(`${arg} must be a non-negative number`);
      }
      index += 1;
      continue;
    }

    if (arg === "--help") {
      options.help = true;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
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

function printHelp() {
  process.stdout.write(`Usage: node scripts/node/clean-terminal-temp-artifacts.mjs [--apply] [--tmp-dir PATH] [--min-age-minutes N] [--no-open-file-check]

Safely cleans known Terminal Platform temp artifacts from one temp directory.
Default mode is dry-run. Deletion checks open files with lsof on Unix-like systems and process/module paths on Windows unless --no-open-file-check is provided.
`);
}

function printResults(results, apply) {
  const groups = results.reduce((counts, item) => {
    counts.set(item.status, (counts.get(item.status) ?? 0) + 1);
    return counts;
  }, new Map());

  const summary = Array.from(groups.entries())
    .map(([status, count]) => `${status}:${count}`)
    .join(" ");
  process.stdout.write(`terminal temp cleanup ${apply ? "apply" : "dry-run"} ${summary || "empty"}\n`);

  for (const item of results) {
    const suffix = item.reason ? ` (${item.reason})` : "";
    process.stdout.write(`${item.status}${suffix} ${item.path}\n`);
  }
}

const isCli = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isCli) {
  try {
    const options = parseArgs(process.argv.slice(2));
    if (options.help) {
      printHelp();
      process.exit(0);
    }

    const results = await cleanTerminalTempArtifacts(options);
    printResults(results, options.apply);

    if (results.some((item) => item.status === "failed")) {
      process.exitCode = 1;
    }
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
  }
}
