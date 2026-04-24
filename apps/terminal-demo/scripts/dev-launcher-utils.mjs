import { spawn, spawnSync } from "node:child_process";
import process from "node:process";
import path from "node:path";

export function runSync(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

export function spawnViteDevServer(appRoot, rendererPort) {
  const viteCliPath = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
  const child = spawn(process.execPath, [
    viteCliPath,
    "--force",
    "--host",
    "127.0.0.1",
    "--port",
    rendererPort,
  ], {
    cwd: appRoot,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
  });

  pipeProcess(child, "[terminal-demo:vite]");
  return child;
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

export function stopProcess(child) {
  if (child && !child.killed) {
    child.kill("SIGTERM");
  }
}

export async function waitForServer(url, options = {}) {
  const startedAt = Date.now();
  const timeoutMs = options.timeoutMs ?? 20_000;

  while (Date.now() - startedAt < timeoutMs) {
    if (options.child?.exitCode !== null && options.child?.exitCode !== undefined) {
      throw new Error(`${options.label ?? "Server"} exited before ${url} became ready with code ${options.child.exitCode}`);
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
