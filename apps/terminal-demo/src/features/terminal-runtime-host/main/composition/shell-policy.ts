export const DEFAULT_TERMINAL_RUNTIME_SLUG = "terminal-demo";
export const DEFAULT_TERMINAL_DEMO_UNIX_SHELL = "bash";
export const DEFAULT_TERMINAL_DEMO_MACOS_SHELL = "zsh";
export const DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL = "pwsh.exe";

export function resolveDemoDefaultShellProgram(options: {
  env?: Readonly<Record<string, string | undefined>>;
  platform?: NodeJS.Platform;
} = {}): string {
  const env = options.env ?? process.env;
  const platform = options.platform ?? process.platform;
  const explicitProgram = normalizeTerminalShellProgram(env.TERMINAL_DEMO_DEFAULT_SHELL);
  if (explicitProgram) {
    return explicitProgram;
  }

  if (platform === "win32") {
    return normalizeTerminalShellProgram(env.ComSpec)
      ?? normalizeTerminalShellProgram(env.COMSPEC)
      ?? DEFAULT_TERMINAL_DEMO_WINDOWS_SHELL;
  }

  return normalizeTerminalShellProgram(env.SHELL)
    ?? (platform === "darwin" ? DEFAULT_TERMINAL_DEMO_MACOS_SHELL : DEFAULT_TERMINAL_DEMO_UNIX_SHELL);
}

export function normalizeTerminalShellProgram(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  return normalized || null;
}
