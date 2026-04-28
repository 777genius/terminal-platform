import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { once } from "node:events";
import { loadTerminalPlatformSdk } from "./terminal-platform-sdk.js";

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

      this.stopExistingDaemonProcesses();
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
    if (child.exitCode === null && !child.killed) {
      child.kill("SIGTERM");
    }

    if (child.exitCode === null) {
      await once(child, "exit").catch(() => undefined);
    }
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
    });
    this.#child = child;

    child.stdout?.on("data", (chunk: Buffer) => {
      process.stdout.write(`[terminal-daemon] ${chunk}`);
    });

    child.stderr?.on("data", (chunk: Buffer) => {
      process.stderr.write(`[terminal-daemon] ${chunk}`);
    });
  }

  private stopExistingDaemonProcesses(): void {
    for (const pid of findDaemonProcesses(this.#runtimeSlug)) {
      try {
        process.kill(pid, "SIGTERM");
      } catch {
        // Ignore races where the matched daemon exited before we could signal it.
      }
    }
  }

  private async waitUntilReady(): Promise<void> {
    const startedAt = Date.now();

    while (Date.now() - startedAt < 15_000) {
      if (this.#child?.exitCode != null) {
        throw new Error(`terminal-daemon exited with code ${this.#child.exitCode}`);
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
    await fs.rm(dir, { recursive: true, force: true }).catch(() => undefined);
  }
}

function resolveRepoRoot(): string {
  return path.resolve(process.cwd(), "../..");
}

function resolveDaemonBinaryPath(): string {
  const filename = process.platform === "win32"
    ? "terminal-daemon.exe"
    : "terminal-daemon";

  return path.resolve(resolveRepoRoot(), "target", "debug", filename);
}

async function resolveDaemonRuntimeBinary(): Promise<{ binaryPath: string; runtimeDir: string | null }> {
  const binaryPath = resolveDaemonBinaryPath();
  if (process.platform !== "win32" || process.env.TERMINAL_DEMO_DAEMON_RUNTIME_COPY === "0") {
    return { binaryPath, runtimeDir: null };
  }

  const runtimeDir = await fs.mkdtemp(
    path.join(os.tmpdir(), `terminal-demo-daemon-runtime-${process.pid}-`),
  );
  const runtimeBinaryPath = path.join(runtimeDir, path.basename(binaryPath));
  await fs.copyFile(binaryPath, runtimeBinaryPath);
  return { binaryPath: runtimeBinaryPath, runtimeDir };
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
