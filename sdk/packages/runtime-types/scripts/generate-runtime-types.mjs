import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { replaceDirectoryAtomically, withSiblingStagingDirectory } from "../../../../scripts/node/replace-directory-atomically.mjs";
import { withFileLock } from "../../../../scripts/node/with-file-lock.mjs";

function packageDirFromMeta(metaUrl) {
  const scriptDir = path.dirname(fileURLToPath(metaUrl));
  return path.resolve(scriptDir, "..");
}

function repoRootFromPackageDir(packageDir) {
  return path.resolve(packageDir, "../../..");
}

export function resolveRuntimeTypesPaths(metaUrl) {
  const packageDir = packageDirFromMeta(metaUrl);
  const repoRoot = repoRootFromPackageDir(packageDir);
  const rawDir = path.join(packageDir, "src/generated/raw");
  return { packageDir, repoRoot, rawDir };
}

export function snapshotDirectory(dir) {
  if (!path.isAbsolute(dir)) {
    throw new Error(`snapshotDirectory expects an absolute path, got: ${dir}`);
  }

  const snapshot = new Map();

  if (!readdirSafe(dir)) {
    return snapshot;
  }

  walk(dir, dir, snapshot);
  return snapshot;
}

export function generateRuntimeTypes(metaUrl) {
  const { packageDir, repoRoot, rawDir } = resolveRuntimeTypesPaths(metaUrl);

  return withFileLock(path.join(packageDir, ".runtime-types-generate.lock"), async () => {
    await withSiblingStagingDirectory(rawDir, "generate", async (stagedRawDir) => {
      execFileSync(
        "cargo",
        [
          "run",
          "-p",
          "xtask",
          "--",
          "export-sdk-runtime-types",
          "--out",
          path.relative(repoRoot, stagedRawDir),
        ],
        {
          cwd: repoRoot,
          stdio: "inherit",
        },
      );

      await replaceDirectoryAtomically(rawDir, stagedRawDir);
    });
  }, {
    metadata: {
      script: "runtime-types-generate",
      targetDir: rawDir,
    },
  });
}

function readdirSafe(dir) {
  try {
    readdirSync(dir);
    return true;
  } catch {
    return false;
  }
}

function walk(rootDir, currentDir, snapshot) {
  for (const entry of readdirSync(currentDir, { withFileTypes: true })) {
    const fullPath = path.join(currentDir, entry.name);
    if (entry.isDirectory()) {
      walk(rootDir, fullPath, snapshot);
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    snapshot.set(path.relative(rootDir, fullPath), readFileSync(fullPath, "utf8"));
  }
}
