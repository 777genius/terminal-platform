import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";
import { once } from "node:events";
import { loadTerminalPlatformSdk } from "./terminal-platform-sdk.js";

interface DaemonSupervisorOptions {
  runtimeSlug: string;
}

export class DaemonSupervisor {
  readonly #runtimeSlug: string;
  #child: ChildProcess | null = null;
  #ownsProcess = false;

  constructor(options: DaemonSupervisorOptions) {
    this.#runtimeSlug = options.runtimeSlug;
  }

  async ensureRunning(): Promise<void> {
    if (await this.isReady()) {
      return;
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
    const child = spawn(binaryPath, ["--runtime-slug", this.#runtimeSlug], {
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
