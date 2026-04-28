import { spawn, spawnSync } from "node:child_process";
import process from "node:process";
import path from "node:path";

const DEFAULT_PROCESS_GRACE_MS = 5_000;
const DEFAULT_PROCESS_FORCE_GRACE_MS = 2_000;

export function runSync(command, args, cwd) {
  const resolved = resolveSpawnCommand(command, args);
  const result = spawnSync(resolved.command, resolved.args, {
    cwd,
    env: process.env,
    shell: resolved.shell,
    stdio: "inherit",
    windowsHide: true,
  });

  if (result.error) {
    throw new Error(`${command} ${args.join(" ")} failed: ${result.error.message}`);
  }

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

export function spawnViteDevServer(appRoot, rendererPort) {
  const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
  const child = spawn(process.execPath, buildViteDevServerArgs(viteCliPath, rendererPort), {
    cwd: appRoot,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });

  pipeProcess(child, "[terminal-demo:vite]");
  return child;
}

export function buildViteDevServerArgs(viteCliPath, rendererPort) {
  return [
    viteCliPath,
    "--force",
    "--host",
    "127.0.0.1",
    "--port",
    rendererPort,
    "--strictPort",
  ];
}

export function spawnElectronPreview(appRoot, rendererUrl) {
  const electronCliPath = path.join(appRoot, "node_modules", "electron", "cli.js");
  const child = spawn(process.execPath, [electronCliPath, "./dist/host/main/index.js"], {
    cwd: appRoot,
    env: {
      ...process.env,
      TERMINAL_DEMO_RENDERER_URL: rendererUrl,
    },
    stdio: "inherit",
  });

  return child;
}

export async function stopProcess(child, options = {}) {
  if (!isProcessRunning(child)) {
    return false;
  }

  const exited = waitForProcessExit(child);
  sendProcessSignal(child, options.gracefulSignal ?? resolveGracefulStopSignal());
  await Promise.race([exited, sleep(options.graceMs ?? DEFAULT_PROCESS_GRACE_MS)]);

  if (!isProcessRunning(child)) {
    return true;
  }

  if (process.platform === "win32" && child.pid && options.forceProcessTree !== false) {
    spawnSync("taskkill.exe", buildWindowsTaskkillArgs(child.pid), {
      stdio: "ignore",
      windowsHide: true,
    });
  } else {
    sendProcessSignal(child, "SIGKILL");
  }

  await Promise.race([exited, sleep(options.forceGraceMs ?? DEFAULT_PROCESS_FORCE_GRACE_MS)]);
  return !isProcessRunning(child);
}

export async function waitForServer(url, options = {}) {
  const startedAt = Date.now();
  const timeoutMs = options.timeoutMs ?? 30_000;

  while (Date.now() - startedAt < timeoutMs) {
    const exitState = processExitState(options.child);
    if (exitState) {
      throw new Error(`${options.label ?? "Server"} exited before ${url} became ready - ${exitState}`);
    }

    try {
      const response = await fetch(url, { method: "GET" });
      if (response.ok) {
        return;
      }
    } catch {
      // Server is still starting.
    }

    await new Promise((resolve) => setTimeout(resolve, 250));
  }

  throw new Error(`Timed out waiting for ${options.label ?? "server"} at ${url}`);
}

function processExitState(child) {
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

function pipeProcess(child, label) {
  const pipe = (stream) => {
    stream?.on("data", (chunk) => {
      for (const line of chunk.toString().split(/\r?\n/u)) {
        if (line.length > 0) {
          process.stdout.write(`${label} ${line}\n`);
        }
      }
    });
  };

  pipe(child.stdout);
  pipe(child.stderr);
}

export function resolveGracefulStopSignal(platform = process.platform) {
  return platform === "win32" ? "SIGINT" : "SIGTERM";
}

export function buildWindowsTaskkillArgs(pid) {
  return ["/PID", String(pid), "/T", "/F"];
}

function isProcessRunning(child) {
  return Boolean(child && child.exitCode === null && child.signalCode === null);
}

function waitForProcessExit(child) {
  return new Promise((resolve) => {
    child.once("exit", () => resolve());
  });
}

function sendProcessSignal(child, signal) {
  try {
    child.kill(signal);
  } catch {
    // The process may have exited between the running check and the signal.
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function resolveSpawnCommand(command, args, env = process.env) {
  if (process.platform !== "win32" || command.includes("/") || command.includes("\\")) {
    return { args, command, shell: false };
  }

  if (command === "npm" || command === "npx" || command === "pnpm" || command === "yarn") {
    const npmExecPath = command === "npm" ? env.npm_execpath : null;
    if (npmExecPath) {
      return { args: [npmExecPath, ...args], command: process.execPath, shell: false };
    }

    return { args, command: `${command}.cmd`, shell: true };
  }

  return { args, command, shell: false };
}

export function resolveCommandForSpawn(command) {
  return resolveSpawnCommand(command, []).command;
}
