import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import process from "node:process";

const DEFAULT_LOCK_TIMEOUT_MS = 10 * 60 * 1_000;
const DEFAULT_STALE_LOCK_MS = 15 * 60 * 1_000;
const LOCK_RETRY_DELAY_MS = 500;
const LOCK_HEARTBEAT_MS = 5_000;

export async function withTerminalDemoSmokeLock(label, callback, options = {}) {
  if (process.env.TERMINAL_DEMO_SMOKE_DISABLE_LOCK === "1") {
    return callback();
  }

  const release = await acquireTerminalDemoSmokeLock(label, options);
  try {
    return await callback();
  } finally {
    await release();
  }
}

async function acquireTerminalDemoSmokeLock(label, options = {}) {
  const lockDir = options.lockDir ?? process.env.TERMINAL_DEMO_SMOKE_LOCK_DIR
    ?? path.join(os.tmpdir(), "terminal-demo-browser-smoke.lock");
  const timeoutMs = options.timeoutMs ?? DEFAULT_LOCK_TIMEOUT_MS;
  const staleMs = options.staleMs ?? DEFAULT_STALE_LOCK_MS;
  const ownerPath = path.join(lockDir, "owner.json");
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    try {
      await fs.mkdir(lockDir);
      await writeLockOwner(ownerPath, label);
      const heartbeat = setInterval(() => {
        void writeLockOwner(ownerPath, label).catch(() => undefined);
      }, LOCK_HEARTBEAT_MS);
      heartbeat.unref?.();

      return async () => {
        clearInterval(heartbeat);
        await fs.rm(lockDir, {
          force: true,
          recursive: true,
          maxRetries: process.platform === "win32" ? 8 : 0,
          retryDelay: process.platform === "win32" ? 250 : 0,
        });
      };
    } catch (error) {
      if (error?.code !== "EEXIST") {
        throw error;
      }

      if (await isStaleLock(ownerPath, lockDir, staleMs)) {
        await fs.rm(lockDir, {
          force: true,
          recursive: true,
          maxRetries: process.platform === "win32" ? 8 : 0,
          retryDelay: process.platform === "win32" ? 250 : 0,
        });
        continue;
      }

      await sleep(LOCK_RETRY_DELAY_MS);
    }
  }

  throw new Error(`Timed out waiting for terminal demo smoke lock at ${lockDir}`);
}

async function writeLockOwner(ownerPath, label) {
  await fs.writeFile(
    ownerPath,
    `${JSON.stringify({
      label,
      pid: process.pid,
      updatedAt: new Date().toISOString(),
    }, null, 2)}\n`,
    "utf8",
  );
}

async function isStaleLock(ownerPath, lockDir, staleMs) {
  const stat = await fs.stat(ownerPath).catch(() => fs.stat(lockDir).catch(() => null));
  if (!stat) {
    return true;
  }

  return Date.now() - stat.mtimeMs > staleMs;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
