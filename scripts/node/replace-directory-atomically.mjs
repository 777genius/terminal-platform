import fs from "node:fs/promises";
import path from "node:path";

export async function createSiblingStagingDirectory(targetDir, label) {
  const absoluteTargetDir = path.resolve(targetDir);
  const parentDir = path.dirname(absoluteTargetDir);
  await fs.mkdir(parentDir, { recursive: true });
  return fs.mkdtemp(path.join(parentDir, `.${path.basename(absoluteTargetDir)}.${label}.`));
}

export async function replaceDirectoryAtomically(targetDir, stagedDir) {
  const absoluteTargetDir = path.resolve(targetDir);
  const absoluteStagedDir = path.resolve(stagedDir);
  const parentDir = path.dirname(absoluteTargetDir);
  const backupDir = path.join(
    parentDir,
    `.${path.basename(absoluteTargetDir)}.backup.${process.pid}.${Date.now()}`,
  );

  const targetExists = await directoryExists(absoluteTargetDir);

  if (targetExists) {
    await fs.rm(backupDir, { recursive: true, force: true });
    await fs.rename(absoluteTargetDir, backupDir);
  }

  try {
    await fs.rename(absoluteStagedDir, absoluteTargetDir);
  } catch (error) {
    if (targetExists && await directoryExists(backupDir)) {
      await fs.rename(backupDir, absoluteTargetDir);
    }
    throw error;
  }

  if (targetExists) {
    await fs.rm(backupDir, { recursive: true, force: true });
  }
}

async function directoryExists(dir) {
  try {
    const stat = await fs.stat(dir);
    return stat.isDirectory();
  } catch {
    return false;
  }
}
