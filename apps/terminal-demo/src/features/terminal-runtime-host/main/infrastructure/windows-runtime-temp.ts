import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

export const STALE_WINDOWS_RUNTIME_TEMP_DIR_MIN_AGE_MS = 30 * 60_000;

const WINDOWS_RM_RETRIES = 8;
const WINDOWS_RM_RETRY_DELAY_MS = 250;

type WindowsRuntimeTempDirKind = "daemon" | "sdk";

interface ParsedWindowsRuntimeTempDirName {
  kind: WindowsRuntimeTempDirKind;
  ownerPid: number;
}

export interface WindowsRuntimeTempDir extends ParsedWindowsRuntimeTempDirName {
  ageMs: number;
  name: string;
  path: string;
}

export interface WindowsRuntimeTempDirCleanupResult extends WindowsRuntimeTempDir {
  error?: string;
  status: "deleted" | "failed";
}

interface WindowsRuntimeTempDirOptions {
  currentPid?: number;
  isPidRunning?: (pid: number) => boolean;
  minAgeMs?: number;
  nowMs?: number;
  platform?: NodeJS.Platform;
  tmpDir?: string;
}

const RUNTIME_TEMP_DIR_NAME_PATTERNS: Array<{
  kind: WindowsRuntimeTempDirKind;
  pattern: RegExp;
}> = [
  { kind: "daemon", pattern: /^terminal-demo-daemon-runtime-(\d+)-[A-Za-z0-9]+$/u },
  { kind: "sdk", pattern: /^terminal-demo-sdk-runtime-(\d+)-[A-Za-z0-9]+$/u },
];

export async function collectStaleWindowsRuntimeTempDirs(
  options: WindowsRuntimeTempDirOptions = {},
): Promise<WindowsRuntimeTempDir[]> {
  const platform = options.platform ?? process.platform;
  if (platform !== "win32") {
    return [];
  }

  const tmpDir = path.resolve(options.tmpDir ?? os.tmpdir());
  const nowMs = options.nowMs ?? Date.now();
  const minAgeMs = options.minAgeMs ?? STALE_WINDOWS_RUNTIME_TEMP_DIR_MIN_AGE_MS;
  const currentPid = options.currentPid ?? process.pid;
  const isPidRunning = options.isPidRunning ?? isProcessRunning;
  const entries = await fs.readdir(tmpDir, { withFileTypes: true }).catch(() => []);
  const staleDirs: WindowsRuntimeTempDir[] = [];

  for (const entry of entries) {
    const parsedName = parseWindowsRuntimeTempDirName(entry.name);
    if (!parsedName || !entry.isDirectory()) {
      continue;
    }

    if (parsedName.ownerPid === currentPid || isPidRunning(parsedName.ownerPid)) {
      continue;
    }

    const dirPath = path.resolve(path.join(tmpDir, entry.name));
    if (path.dirname(dirPath) !== tmpDir) {
      continue;
    }

    const stat = await fs.lstat(dirPath).catch(() => null);
    if (!stat?.isDirectory() || stat.isSymbolicLink()) {
      continue;
    }

    const ageMs = Math.max(0, nowMs - stat.mtimeMs);
    if (ageMs < minAgeMs) {
      continue;
    }

    staleDirs.push({
      ageMs,
      kind: parsedName.kind,
      name: entry.name,
      ownerPid: parsedName.ownerPid,
      path: dirPath,
    });
  }

  return staleDirs.sort((left, right) => left.name.localeCompare(right.name));
}

export async function cleanupStaleWindowsRuntimeTempDirs(
  options: WindowsRuntimeTempDirOptions = {},
): Promise<WindowsRuntimeTempDirCleanupResult[]> {
  const staleDirs = await collectStaleWindowsRuntimeTempDirs(options);
  const platform = options.platform ?? process.platform;
  const results: WindowsRuntimeTempDirCleanupResult[] = [];

  for (const staleDir of staleDirs) {
    try {
      await fs.rm(staleDir.path, {
        force: true,
        maxRetries: platform === "win32" ? WINDOWS_RM_RETRIES : 0,
        recursive: true,
        retryDelay: platform === "win32" ? WINDOWS_RM_RETRY_DELAY_MS : 0,
      });
      results.push({ ...staleDir, status: "deleted" });
    } catch (error) {
      results.push({
        ...staleDir,
        error: error instanceof Error ? error.message : String(error),
        status: "failed",
      });
    }
  }

  return results;
}

function parseWindowsRuntimeTempDirName(name: string): ParsedWindowsRuntimeTempDirName | null {
  for (const item of RUNTIME_TEMP_DIR_NAME_PATTERNS) {
    const match = item.pattern.exec(name);
    const pidText = match?.[1];
    if (!pidText) {
      continue;
    }

    const ownerPid = Number.parseInt(pidText, 10);
    if (!Number.isSafeInteger(ownerPid) || ownerPid <= 0) {
      return null;
    }

    return { kind: item.kind, ownerPid };
  }

  return null;
}

function isProcessRunning(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}
