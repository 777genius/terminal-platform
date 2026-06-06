import { existsSync, statSync } from "node:fs";
import path from "node:path";

export const DEFAULT_TERMINAL_RUNTIME_SLUG = "terminal-demo";
export const DEFAULT_TERMINAL_DEMO_UNIX_SHELL = "bash";
export const DEFAULT_TERMINAL_DEMO_MACOS_SHELL = "zsh";
export const DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL = "cmd.exe";

export function resolveDemoDefaultShellProgram(options: {
  env?: Readonly<Record<string, string | undefined>>;
  platform?: NodeJS.Platform;
  validateWindowsPaths?: boolean;
} = {}): string {
  const env = options.env ?? process.env;
  const platform = options.platform ?? process.platform;
  const explicitProgram = normalizeTerminalShellProgram(env.TERMINAL_DEMO_DEFAULT_SHELL);
  if (explicitProgram) {
    return explicitProgram;
  }

  if (platform === "win32") {
    return resolveWindowsShellProgram(env, options.validateWindowsPaths ?? false);
  }

  return normalizeTerminalShellProgram(env.SHELL)
    ?? (platform === "darwin" ? DEFAULT_TERMINAL_DEMO_MACOS_SHELL : DEFAULT_TERMINAL_DEMO_UNIX_SHELL);
}

export function resolveDemoDefaultWorkingDirectory(options: {
  env?: Readonly<Record<string, string | undefined>>;
  cwd?: string;
  validateExists?: boolean;
} = {}): string | null {
  const env = options.env ?? process.env;
  const explicitCwd = normalizeTerminalShellProgram(env.TERMINAL_DEMO_DEFAULT_CWD);
  const fallbackCwd = normalizeTerminalShellProgram(options.cwd ?? process.cwd());
  if (!options.validateExists) {
    return explicitCwd ?? fallbackCwd;
  }

  return resolveExistingDirectory(explicitCwd)
    ?? resolveExistingDirectory(fallbackCwd)
    ?? null;
}

export function normalizeTerminalShellProgram(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized || null;
}

function resolveWindowsCmdProgram(env: Readonly<Record<string, string | undefined>>): string | null {
  const windowsRoot = normalizeTerminalShellProgram(env.SystemRoot) ?? normalizeTerminalShellProgram(env.WINDIR);
  return windowsRoot ? `${windowsRoot}\\System32\\cmd.exe` : null;
}

function resolveWindowsShellProgram(
  env: Readonly<Record<string, string | undefined>>,
  validateWindowsPaths: boolean,
): string {
  const candidates = [
    normalizeTerminalShellProgram(env.ComSpec),
    normalizeTerminalShellProgram(env.COMSPEC),
    resolveWindowsCmdProgram(env),
    DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL,
  ];

  for (const candidate of candidates) {
    if (!candidate) {
      continue;
    }

    if (
      validateWindowsPaths
      && isPathLikeShellProgram(candidate)
      && !isExistingFile(candidate)
    ) {
      continue;
    }

    return candidate;
  }

  return DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL;
}

function resolveExistingDirectory(value: string | null): string | null {
  if (!value) {
    return null;
  }

  try {
    return statSync(value).isDirectory() ? path.resolve(value) : null;
  } catch {
    return null;
  }
}

function isExistingFile(value: string): boolean {
  try {
    return existsSync(value) && statSync(value).isFile();
  } catch {
    return false;
  }
}

function isPathLikeShellProgram(value: string): boolean {
  return /^[a-z]:[\\/]/iu.test(value)
    || value.startsWith("\\\\")
    || value.includes("/")
    || value.includes("\\");
}
