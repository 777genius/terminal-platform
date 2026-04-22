import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type * as TerminalPlatformSdk from "../../../../../.generated/terminal-platform-node/index.mjs";

export type TerminalPlatformSdkModule = typeof TerminalPlatformSdk;

let sdkPromise: Promise<TerminalPlatformSdkModule> | null = null;

export async function loadTerminalPlatformSdk(): Promise<TerminalPlatformSdkModule> {
  sdkPromise ??= import(resolveSdkModuleUrl()) as Promise<TerminalPlatformSdkModule>;
  return sdkPromise;
}

function resolveSdkModuleUrl(): string {
  const moduleDir = path.dirname(fileURLToPath(import.meta.url));
  const appRoot = path.resolve(moduleDir, "../../../../../");
  const modulePath = path.resolve(
    appRoot,
    ".generated/terminal-platform-node/index.mjs",
  );

  return pathToFileURL(modulePath).href;
}
