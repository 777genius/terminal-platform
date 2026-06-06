import fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
import path from "node:path";
import {
  TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH,
  type TerminalRuntimeBootstrapConfig,
} from "@features/terminal-runtime-host/contracts";

export type BrowserBootstrapScope = "dist-only" | "public-and-dist" | "public-only";

const WINDOWS_FILE_OPERATION_RETRIES = 8;
const WINDOWS_FILE_OPERATION_RETRY_DELAY_MS = 250;

export async function writeBrowserBootstrapConfig(options: {
  appRoot: string;
  config: TerminalRuntimeBootstrapConfig;
  scope: string;
}): Promise<void> {
  const relativeTarget = TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH.replace(/^\/+/, "");
  const targets = resolveBrowserBootstrapTargets({
    appRoot: options.appRoot,
    relativeTarget,
    scope: options.scope,
  });
  const payload = buildBrowserBootstrapPayload(options.config);

  await Promise.all(targets.map((targetPath) => writeFileAtomically(targetPath, payload)));
}

export async function clearBrowserBootstrapConfig(options: {
  appRoot: string;
  expectedConfig?: TerminalRuntimeBootstrapConfig | null;
  scope: string;
}): Promise<void> {
  const relativeTarget = TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH.replace(/^\/+/, "");
  const targets = resolveBrowserBootstrapTargets({
    appRoot: options.appRoot,
    relativeTarget,
    scope: options.scope,
  });

  const expectedPayload = options.expectedConfig
    ? buildBrowserBootstrapPayload(options.expectedConfig)
    : null;

  await Promise.all(
    targets.map((targetPath) => removeBrowserBootstrapTarget(targetPath, expectedPayload)),
  );
}

export function buildBrowserBootstrapPayload(config: TerminalRuntimeBootstrapConfig): string {
  return `${JSON.stringify(config, null, 2)}\n`;
}

export function resolveBrowserBootstrapTargets(options: {
  appRoot: string;
  relativeTarget: string;
  scope: string;
}): string[] {
  const scope = normalizeBrowserBootstrapScope(options.scope);
  if (scope === "dist-only") {
    return [path.join(options.appRoot, "dist", "renderer", options.relativeTarget)];
  }

  if (scope === "public-only") {
    return [path.join(options.appRoot, "public", options.relativeTarget)];
  }

  return [
    path.join(options.appRoot, "public", options.relativeTarget),
    path.join(options.appRoot, "dist", "renderer", options.relativeTarget),
  ];
}

export function normalizeBrowserBootstrapScope(scope: string): BrowserBootstrapScope {
  if (scope === "dist-only" || scope === "public-only") {
    return scope;
  }

  return "public-and-dist";
}

async function writeFileAtomically(targetPath: string, payload: string): Promise<void> {
  await fs.mkdir(path.dirname(targetPath), { recursive: true });
  const tempPath = [
    targetPath,
    ".",
    process.pid,
    ".",
    Date.now(),
    ".",
    randomUUID(),
    ".tmp",
  ].join("");

  try {
    await fs.writeFile(tempPath, payload, "utf8");
    await renameWithWindowsRetries(tempPath, targetPath);
  } catch (error) {
    await removeFileWithWindowsRetries(tempPath).catch(() => undefined);
    throw error;
  }
}

async function renameWithWindowsRetries(fromPath: string, toPath: string): Promise<void> {
  await retryWindowsFileOperation(() => fs.rename(fromPath, toPath));
}

async function removeFileWithWindowsRetries(filePath: string): Promise<void> {
  await retryWindowsFileOperation(() => fs.rm(filePath, { force: true }));
}

async function removeBrowserBootstrapTarget(
  targetPath: string,
  expectedPayload: string | null,
): Promise<void> {
  if (expectedPayload) {
    const currentPayload = await readOptionalFile(targetPath);
    if (currentPayload === null || currentPayload !== expectedPayload) {
      return;
    }
  }

  await removeFileWithWindowsRetries(targetPath);
}

async function readOptionalFile(filePath: string): Promise<string | null> {
  try {
    return await retryWindowsFileOperation(() => fs.readFile(filePath, "utf8"));
  } catch (error) {
    if (
      (error as NodeJS.ErrnoException).code === "ENOENT"
      || isRetriableWindowsFileError(error)
    ) {
      return null;
    }

    throw error;
  }
}

async function retryWindowsFileOperation<T>(operation: () => Promise<T>): Promise<T> {
  let lastError: unknown = null;
  const maxAttempts = process.platform === "win32" ? WINDOWS_FILE_OPERATION_RETRIES + 1 : 1;
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    try {
      return await operation();
    } catch (error) {
      lastError = error;
      if (!isRetriableWindowsFileError(error) || attempt === maxAttempts - 1) {
        throw error;
      }

      await sleep(WINDOWS_FILE_OPERATION_RETRY_DELAY_MS);
    }
  }

  throw lastError;
}

export function isRetriableWindowsBootstrapFileError(
  error: unknown,
  platform: NodeJS.Platform = process.platform,
): boolean {
  if (platform !== "win32" || typeof error !== "object" || error === null) {
    return false;
  }

  const code = (error as NodeJS.ErrnoException).code;
  return code === "EBUSY" || code === "ENOTEMPTY" || code === "EPERM";
}

function isRetriableWindowsFileError(error: unknown): boolean {
  return isRetriableWindowsBootstrapFileError(error);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
