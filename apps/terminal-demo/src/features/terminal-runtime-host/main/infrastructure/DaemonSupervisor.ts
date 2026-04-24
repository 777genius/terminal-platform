import { spawn, spawnSync, type ChildProcess } from "node:child_process";
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

    this.spawnDaemon();
    this.#ownsProcess = true;
    await this.waitUntilReady();
  }

  async dispose(): Promise<void> {
    if (!this.#child || !this.#ownsProcess) {
      return;
    }

    if (this.#child.exitCode === null && !this.#child.killed) {
      this.#child.kill("SIGTERM");
    }

    await once(this.#child, "exit").catch(() => undefined);
    this.#child = null;
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

  private spawnDaemon(): void {
    const binaryPath = resolveDaemonBinaryPath();
    const args = ["--runtime-slug", this.#runtimeSlug];
    if (this.#sessionStorePath) {
      args.push("--session-store", this.#sessionStorePath);
    }

    const child = spawn(binaryPath, args, {
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

function findDaemonProcesses(runtimeSlug: string): number[] {
  if (process.platform === "win32") {
    return [];
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

      if (
        !command.includes("terminal-daemon")
        || !command.includes(`--runtime-slug ${runtimeSlug}`)
        || pid === process.pid
      ) {
        return null;
      }

      return pid;
    })
    .filter((pid): pid is number => pid != null);
}
