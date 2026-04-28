import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { loadTerminalPlatformSdk } from "./terminal-platform-sdk.js";
import { cleanupStaleWindowsRuntimeTempDirs } from "./windows-runtime-temp.js";

const DAEMON_GRACEFUL_SHUTDOWN_MS = 5_000;
const DAEMON_FORCED_SHUTDOWN_MS = 2_000;
const DAEMON_PROCESS_POLL_MS = 150;
const WINDOWS_RM_RETRIES = 8;
const TERMINAL_DAEMON_REPO_ROOT_ENV = "TERMINAL_DEMO_REPO_ROOT";

interface DaemonSupervisorOptions {
  runtimeSlug: string;
  forceRestartReadyDaemon?: boolean;
  sessionStorePath?: string | null;
}

export class DaemonSupervisor {
  readonly #runtimeSlug: string;
  readonly #forceRestartReadyDaemon: boolean;
  readonly #sessionStorePath: string | null;
  #child: ChildProcess | null = null;
  #ownsProcess = false;
  #runtimeDaemonDir: string | null = null;

  constructor(options: DaemonSupervisorOptions) {
    this.#runtimeSlug = options.runtimeSlug;
    this.#forceRestartReadyDaemon = options.forceRestartReadyDaemon ?? false;
    this.#sessionStorePath = options.sessionStorePath ?? null;
  }

  async ensureRunning(): Promise<void> {
    if (await this.isReady()) {
      if (!this.#forceRestartReadyDaemon) {
        return;
      }

      await this.stopExistingDaemonProcesses();
    }

    await this.spawnDaemon();
    this.#ownsProcess = true;
    try {
      await this.waitUntilReady();
    } catch (error) {
      await this.dispose();
      throw error;
    }
  }

  async dispose(): Promise<void> {
    if (!this.#child || !this.#ownsProcess) {
      return;
    }

    const child = this.#child;
    await stopChildProcess(child);
    this.#child = null;
    await this.cleanupRuntimeDaemonDir();
  }

  private async isReady(): Promise<boolean> {
    try {
      const sdk = await loadTerminalPlatformSdk();
      const client = sdk.TerminalNodeClient.fromRuntimeSlug(this.#runtimeSlug);
      await client.handshakeInfo();
      return true;
    } catch {
      return false;
    }
  }

  private async spawnDaemon(): Promise<void> {
    const runtimeDaemon = await resolveDaemonRuntimeBinary();
    this.#runtimeDaemonDir = runtimeDaemon.runtimeDir;
    const args = ["--runtime-slug", this.#runtimeSlug];
    if (this.#sessionStorePath) {
      args.push("--session-store", this.#sessionStorePath);
    }

    const child = spawn(runtimeDaemon.binaryPath, args, {
      cwd: resolveRepoRoot(),
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    this.#child = child;

    child.stdout?.on("data", (chunk: Buffer) => {
      process.stdout.write(`[terminal-daemon] ${chunk}`);
    });

    child.stderr?.on("data", (chunk: Buffer) => {
      process.stderr.write(`[terminal-daemon] ${chunk}`);
    });
  }

  private async stopExistingDaemonProcesses(): Promise<void> {
    const pids = findDaemonProcesses(this.#runtimeSlug);
    if (pids.length === 0) {
      return;
    }

    for (const pid of pids) {
      terminateProcessId(pid);
    }

    await waitForDaemonProcessesToExit(this.#runtimeSlug, pids, DAEMON_GRACEFUL_SHUTDOWN_MS);

    const survivors = findDaemonProcesses(this.#runtimeSlug)
      .filter((pid) => pids.includes(pid));
    for (const pid of survivors) {
      forceKillProcessId(pid);
    }

    await waitForDaemonProcessesToExit(this.#runtimeSlug, survivors, DAEMON_FORCED_SHUTDOWN_MS);
  }

  private async waitUntilReady(): Promise<void> {
    const startedAt = Date.now();

    while (Date.now() - startedAt < 15_000) {
      const exitState = resolveChildExitState(this.#child);
      if (exitState) {
        throw new Error(`terminal-daemon exited before becoming ready: ${exitState}`);
      }

      if (await this.isReady()) {
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, 200));
    }

    throw new Error("Timed out waiting for terminal-daemon to become ready");
  }

  private async cleanupRuntimeDaemonDir(): Promise<void> {
    if (!this.#runtimeDaemonDir) {
      return;
    }

    const dir = this.#runtimeDaemonDir;
    this.#runtimeDaemonDir = null;
    await fs.rm(dir, {
      recursive: true,
      force: true,
      maxRetries: process.platform === "win32" ? WINDOWS_RM_RETRIES : 0,
      retryDelay: process.platform === "win32" ? 250 : 0,
    }).catch(() => undefined);
  }
}

async function stopChildProcess(child: ChildProcess): Promise<void> {
  if (!isChildProcessRunning(child)) {
    return;
  }

  const exited = once(child, "exit").then(() => undefined).catch(() => undefined);
  terminateChildProcess(child);
  await Promise.race([exited, sleep(DAEMON_GRACEFUL_SHUTDOWN_MS)]);

  if (!isChildProcessRunning(child)) {
    return;
  }

  if (process.platform === "win32" && child.pid) {
    forceKillWindowsProcessTree(child.pid);
  } else {
    terminateChildProcess(child, "SIGKILL");
  }

  await Promise.race([exited, sleep(DAEMON_FORCED_SHUTDOWN_MS)]);
}

function isChildProcessRunning(child: ChildProcess): boolean {
  return child.exitCode === null && child.signalCode === null;
}

function resolveChildExitState(child: ChildProcess | null): string | null {
  if (!child) {
    return null;
  }

  if (child.exitCode !== null) {
    return `exit code ${child.exitCode}`;
  }

  if (child.signalCode !== null) {
    return `signal ${child.signalCode}`;
  }

  return null;
}

function terminateChildProcess(child: ChildProcess, signal: NodeJS.Signals = "SIGTERM"): void {
  try {
    child.kill(signal);
  } catch {
    // Ignore races where the child exited before we could signal it.
  }
}

function terminateProcessId(pid: number): void {
  try {
    process.kill(pid, "SIGTERM");
  } catch {
    // Ignore races where the matched daemon exited before we could signal it.
  }
}

function forceKillProcessId(pid: number): void {
  if (process.platform === "win32") {
    forceKillWindowsProcessTree(pid);
    return;
  }

  try {
    process.kill(pid, "SIGKILL");
  } catch {
    // Ignore races where the matched daemon exited before we could signal it.
  }
}

function forceKillWindowsProcessTree(pid: number): void {
  spawnSync("taskkill.exe", ["/PID", String(pid), "/T", "/F"], {
    encoding: "utf8",
    stdio: "ignore",
    windowsHide: true,
  });
}

async function waitForDaemonProcessesToExit(
  runtimeSlug: string,
  pids: number[],
  timeoutMs: number,
): Promise<void> {
  if (pids.length === 0) {
    return;
  }

  const pidSet = new Set(pids);
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const stillRunning = findDaemonProcesses(runtimeSlug)
      .some((pid) => pidSet.has(pid));
    if (!stillRunning) {
      return;
    }

    await sleep(DAEMON_PROCESS_POLL_MS);
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function resolveRepoRoot(): string {
  return resolveDaemonRepoRoot();
}

export function resolveDaemonRepoRoot(options: {
  env?: Readonly<Record<string, string | undefined>>;
  moduleUrl?: string;
} = {}): string {
  const env = options.env ?? process.env;
  const explicitRoot = env[TERMINAL_DAEMON_REPO_ROOT_ENV]?.trim();
  if (explicitRoot) {
    return path.resolve(explicitRoot);
  }

  const moduleDir = path.dirname(fileURLToPath(options.moduleUrl ?? import.meta.url));
  const appRoot = path.resolve(moduleDir, "../../../../../");
  return path.resolve(appRoot, "../..");
}

export function resolveDaemonBinaryPath(options: {
  env?: Readonly<Record<string, string | undefined>>;
  moduleUrl?: string;
  platform?: NodeJS.Platform;
} = {}): string {
  const platform = options.platform ?? process.platform;
  const filename = platform === "win32"
    ? "terminal-daemon.exe"
    : "terminal-daemon";

  return path.resolve(resolveDaemonRepoRoot(options), "target", "debug", filename);
}

async function resolveDaemonRuntimeBinary(): Promise<{ binaryPath: string; runtimeDir: string | null }> {
  const binaryPath = resolveDaemonBinaryPath();
  await assertDaemonBinaryExists(binaryPath);
  if (process.platform !== "win32" || process.env.TERMINAL_DEMO_DAEMON_RUNTIME_COPY === "0") {
    return { binaryPath, runtimeDir: null };
  }

  await cleanupStaleWindowsRuntimeTempDirs();
  const runtimeDir = await fs.mkdtemp(
    path.join(os.tmpdir(), `terminal-demo-daemon-runtime-${process.pid}-`),
  );
  const runtimeBinaryPath = path.join(runtimeDir, path.basename(binaryPath));
  await fs.copyFile(binaryPath, runtimeBinaryPath);
  return { binaryPath: runtimeBinaryPath, runtimeDir };
}

async function assertDaemonBinaryExists(binaryPath: string): Promise<void> {
  try {
    await fs.access(binaryPath);
  } catch {
    throw new Error(
      `terminal-daemon binary not found at ${binaryPath}. Run cargo build -p terminal-daemon before starting terminal-demo.`,
    );
  }
}

function findDaemonProcesses(runtimeSlug: string): number[] {
  if (process.platform === "win32") {
    return findWindowsDaemonProcesses(runtimeSlug);
  }

  const result = spawnSync("ps", ["-ax", "-o", "pid=,command="], {
    cwd: resolveRepoRoot(),
    env: process.env,
    encoding: "utf8",
  });

  if (result.status !== 0 || !result.stdout) {
    return [];
  }

  return result.stdout
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const match = line.match(/^(\d+)\s+(.*)$/);
      const pidText = match?.[1];
      const command = match?.[2];
      if (!pidText || !command) {
        return null;
      }

      const pid = Number.parseInt(pidText, 10);
      if (!Number.isInteger(pid)) {
        return null;
      }

      if (!isDaemonCommandForRuntime(command, runtimeSlug) || pid === process.pid) {
        return null;
      }

      return pid;
    })
    .filter((pid): pid is number => pid != null);
}

function findWindowsDaemonProcesses(runtimeSlug: string): number[] {
  const result = spawnSync(windowsPowerShellPath(), [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    [
      "$ErrorActionPreference = 'SilentlyContinue'",
      "$items = Get-CimInstance Win32_Process -Filter \"Name = 'terminal-daemon.exe'\"",
      "$items | Select-Object ProcessId,CommandLine | ConvertTo-Json -Compress",
    ].join("; "),
  ], {
    cwd: resolveRepoRoot(),
    env: process.env,
    encoding: "utf8",
    windowsHide: true,
  });

  if (result.status !== 0 || !result.stdout.trim()) {
    return [];
  }

  try {
    const parsed = JSON.parse(result.stdout) as
      | { ProcessId?: number; CommandLine?: string }
      | Array<{ ProcessId?: number; CommandLine?: string }>;
    const rows = Array.isArray(parsed) ? parsed : [parsed];
    return rows
      .map((row) => {
        const pid = row.ProcessId;
        const command = row.CommandLine;
        if (!Number.isInteger(pid) || !command) {
          return null;
        }

        if (!isDaemonCommandForRuntime(command, runtimeSlug) || pid === process.pid) {
          return null;
        }

        return pid;
      })
      .filter((pid): pid is number => pid != null);
  } catch {
    return [];
  }
}

function isDaemonCommandForRuntime(command: string, runtimeSlug: string): boolean {
  const runtimePattern = new RegExp(
    `--runtime-slug(?:=|\\s+["']?)${escapeRegExp(runtimeSlug)}(?:["']?(?:\\s|$))`,
    "i",
  );
  return /terminal-daemon(?:\.exe)?/i.test(command) && runtimePattern.test(command);
}

function windowsPowerShellPath(): string {
  const windowsRoot = process.env.SystemRoot || process.env.WINDIR || "C:\\Windows";
  return path.join(windowsRoot, "System32", "WindowsPowerShell", "v1.0", "powershell.exe");
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}
