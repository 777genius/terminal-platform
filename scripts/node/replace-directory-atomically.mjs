import fs from "node:fs/promises";
import path from "node:path";

export async function createSiblingStagingDirectory(targetDir, label) {
  const absoluteTargetDir = path.resolve(targetDir);
  const parentDir = path.dirname(absoluteTargetDir);
  await fs.mkdir(parentDir, { recursive: true });
  return fs.mkdtemp(path.join(parentDir, `.${path.basename(absoluteTargetDir)}.${label}.`));
}

export async function withSiblingStagingDirectory(targetDir, label, work) {
  const stagedDir = await createSiblingStagingDirectory(targetDir, label);

  try {
    return await work(stagedDir);
  } finally {
    await fs.rm(stagedDir, { recursive: true, force: true });
  }
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

  if (targetExists && await directoriesEqual(absoluteTargetDir, absoluteStagedDir)) {
    return;
  }

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

async function directoriesEqual(leftDir, rightDir) {
  let leftEntries;
  let rightEntries;
  try {
    [leftEntries, rightEntries] = await Promise.all([
      fs.readdir(leftDir, { withFileTypes: true }),
      fs.readdir(rightDir, { withFileTypes: true }),
    ]);
  } catch {
    return false;
  }

  leftEntries.sort(compareDirEntries);
  rightEntries.sort(compareDirEntries);

  if (leftEntries.length !== rightEntries.length) {
    return false;
  }

  for (let index = 0; index < leftEntries.length; index += 1) {
    const leftEntry = leftEntries[index];
    const rightEntry = rightEntries[index];

    if (leftEntry.name !== rightEntry.name || dirEntryKind(leftEntry) !== dirEntryKind(rightEntry)) {
      return false;
    }

    const leftPath = path.join(leftDir, leftEntry.name);
    const rightPath = path.join(rightDir, rightEntry.name);

    if (leftEntry.isDirectory()) {
      if (!await directoriesEqual(leftPath, rightPath)) {
        return false;
      }
      continue;
    }

    if (leftEntry.isFile()) {
      if (!await filesEqual(leftPath, rightPath)) {
        return false;
      }
      continue;
    }

    return false;
  }

  return true;
}

function compareDirEntries(left, right) {
  return left.name.localeCompare(right.name);
}

function dirEntryKind(entry) {
  if (entry.isDirectory()) {
    return "directory";
  }
  if (entry.isFile()) {
    return "file";
  }
  return "other";
}

async function filesEqual(leftPath, rightPath) {
  let leftStat;
  let rightStat;
  try {
    [leftStat, rightStat] = await Promise.all([fs.stat(leftPath), fs.stat(rightPath)]);
  } catch {
    return false;
  }

  if (leftStat.size !== rightStat.size) {
    return false;
  }

  const [left, right] = await Promise.all([fs.readFile(leftPath), fs.readFile(rightPath)]);
  return left.equals(right);
}
