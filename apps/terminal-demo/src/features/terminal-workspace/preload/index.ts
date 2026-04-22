import path from "node:path";
import { fileURLToPath } from "node:url";

export function resolveTerminalWorkspacePreloadPath(): string {
  const moduleDir = path.dirname(fileURLToPath(import.meta.url));
  return path.resolve(moduleDir, "entry.cjs");
}
