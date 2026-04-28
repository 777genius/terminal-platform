import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type * as TerminalPlatformSdk from "../../../../../.generated/terminal-platform-node/index.mjs";
import { cleanupStaleWindowsRuntimeTempDirs } from "./windows-runtime-temp.js";

export type TerminalPlatformSdkModule = typeof TerminalPlatformSdk;

let sdkPromise: Promise<TerminalPlatformSdkModule> | null = null;
let sdkModuleUrlPromise: Promise<string> | null = null;

export async function loadTerminalPlatformSdk(): Promise<TerminalPlatformSdkModule> {
  sdkPromise ??= resolveSdkModuleUrl().then(
    (moduleUrl) => import(moduleUrl) as Promise<TerminalPlatformSdkModule>,
  );
  return sdkPromise;
}

async function resolveSdkModuleUrl(): Promise<string> {
  sdkModuleUrlPromise ??= resolveSdkModulePath().then((modulePath) => pathToFileURL(modulePath).href);
  return sdkModuleUrlPromise;
}

async function resolveSdkModulePath(): Promise<string> {
  const moduleDir = path.dirname(fileURLToPath(import.meta.url));
  const appRoot = path.resolve(moduleDir, "../../../../../");
  const sdkDir = path.resolve(
    appRoot,
    ".generated/terminal-platform-node",
  );

  if (process.platform !== "win32" || process.env.TERMINAL_DEMO_SDK_RUNTIME_COPY === "0") {
    return path.join(sdkDir, "index.mjs");
  }

  await cleanupStaleWindowsRuntimeTempDirs();
  const runtimeSdkDir = await fs.mkdtemp(
    path.join(os.tmpdir(), `terminal-demo-sdk-runtime-${process.pid}-`),
  );
  await fs.cp(sdkDir, runtimeSdkDir, { recursive: true });
  return path.join(runtimeSdkDir, "index.mjs");
}
