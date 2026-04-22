import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const DEFAULT_TIMEOUT_MS = 5 * 60 * 1000;
const DEFAULT_RETRY_MS = 100;
const DEFAULT_STALE_MS = 15 * 60 * 1000;

export async function withFileLock(lockFile, work, options = {}) {
  const absoluteLockFile = path.resolve(lockFile);
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const retryMs = options.retryMs ?? DEFAULT_RETRY_MS;
  const staleMs = options.staleMs ?? DEFAULT_STALE_MS;
  const metadata = options.metadata ?? {};
  const startedAt = Date.now();

  await fs.mkdir(path.dirname(absoluteLockFile), { recursive: true });

  while (true) {
    let handle = null;
    try {
      handle = await fs.open(absoluteLockFile, "wx");
      await handle.writeFile(
        `${JSON.stringify({
          pid: process.pid,
          createdAt: new Date().toISOString(),
          ...metadata,
        }, null, 2)}\n`,
      );
      await handle.close();
      break;
    } catch (error) {
      await handle?.close().catch(() => {});

      if (error?.code !== "EEXIST" && error?.code !== "EISDIR") {
        throw error;
      }

      await clearStaleLockIfNeeded(absoluteLockFile, staleMs);

      if (Date.now() - startedAt >= timeoutMs) {
        throw new Error(`Timed out waiting for lock ${absoluteLockFile}`);
      }

      await sleep(retryMs);
    }
  }

  try {
    return await work();
  } finally {
    await fs.rm(absoluteLockFile, { recursive: true, force: true });
  }
}

async function clearStaleLockIfNeeded(lockFile, staleMs) {
  const metadata = await readMetadata(lockFile);
  const createdAt = metadata?.createdAt ? Date.parse(metadata.createdAt) : Number.NaN;
  const ageMs = Number.isFinite(createdAt) ? Date.now() - createdAt : Number.POSITIVE_INFINITY;
  const pid = typeof metadata?.pid === "number" ? metadata.pid : null;

  if (ageMs < staleMs && pid && isPidAlive(pid)) {
    return;
  }

  await fs.rm(lockFile, { recursive: true, force: true });
}

async function readMetadata(lockFile) {
  try {
    const payload = await fs.readFile(lockFile, "utf8");
    return JSON.parse(payload);
  } catch {
    return null;
  }
}

function isPidAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === "EPERM";
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
