#!/usr/bin/env node

import fsSync from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const srcRoot = path.join(appRoot, "src");
const supportedExtensions = [".ts", ".tsx", ".cts"];
const importPattern = /(?:import|export)\s+(?:[^\"'`]+?\s+from\s+)?[\"'`]([^\"'`]+)[\"'`]|import\(\s*[\"'`]([^\"'`]+)[\"'`]\s*\)|require\(\s*[\"'`]([^\"'`]+)[\"'`]\s*\)/g;
const publicFeatureEntrypointPattern = /^@features\/[^/]+\/(contracts|main|preload|renderer)$/;
const kernelContractsEntrypoint = "@features/terminal-workspace-kernel/contracts";
const legacyTerminalWorkspaceRendererEntrypoint = "@features/terminal-workspace/renderer";
const runtimeHostFeature = "terminal-runtime-host";
const restrictedRendererPackages = new Set(["electron", "ws", "zustand"]);
const restrictedCorePackages = new Set(["electron", "react", "react-dom", "ws", "zustand"]);
const runtimeHostOnlyPackages = new Set(["electron", "ws", "@terminal-platform/runtime-types"]);

const allFiles = await collectSourceFiles(srcRoot);
const entrypointFiles = allFiles.filter((filePath) => {
  const relative = path.relative(appRoot, filePath).replace(/\\/g, "/");
  return relative.startsWith("src/host/") || relative.startsWith("src/renderer/app/");
});
const files = await collectReachableFiles(entrypointFiles);
const violations = [];

for (const filePath of files) {
  const source = await fs.readFile(filePath, "utf8");
  const fileInfo = describeFile(filePath);
  const specifiers = [...source.matchAll(importPattern)]
    .map((match) => match[1] ?? match[2] ?? match[3])
    .filter(Boolean);

  for (const specifier of specifiers) {
    const resolved = resolveProjectModule(filePath, specifier);
    const targetInfo = resolved ? describeFile(resolved) : null;
    const importRef = `${fileInfo.relative} -> ${specifier}`;

    if (specifier.includes(".generated/terminal-platform-node") && !fileInfo.isFeatureMain) {
      violations.push(`${importRef}: staged SDK is only allowed inside feature main adapters/infrastructure`);
    }

    if (runtimeHostOnlyPackages.has(specifier) && fileInfo.isFeatureInternal && fileInfo.featureName !== runtimeHostFeature) {
      violations.push(`${importRef}: only terminal-runtime-host may depend on ${specifier}`);
    }

    if (fileInfo.isFeatureInternal && specifier.startsWith("@features/")) {
      if (specifier !== kernelContractsEntrypoint) {
        violations.push(`${importRef}: feature internals may import only @features/terminal-workspace-kernel/contracts across feature boundaries`);
      }
    }

    if (fileInfo.isFeatureInternal && targetInfo?.isFeatureInternal && fileInfo.featureName !== targetInfo.featureName && specifier !== kernelContractsEntrypoint) {
      violations.push(`${importRef}: cross-feature deep imports are forbidden`);
    }

    if (fileInfo.isShell && specifier.startsWith("@features/") && !publicFeatureEntrypointPattern.test(specifier)) {
      violations.push(`${importRef}: shell may import only public feature entrypoints`);
    }

    if (fileInfo.isShell && specifier === legacyTerminalWorkspaceRendererEntrypoint) {
      violations.push(`${importRef}: shell must compose the active runtime-host workspace path, not the legacy terminal-workspace renderer`);
    }

    if (fileInfo.isShell && targetInfo?.isFeatureInternal && !specifier.startsWith("@features/")) {
      violations.push(`${importRef}: shell must not deep-import feature internals by relative path`);
    }

    if (fileInfo.isContracts) {
      if (isPackageImport(specifier) || specifier.startsWith("node:")) {
        violations.push(`${importRef}: contracts layer must stay package-free and framework-free`);
      }

      if (targetInfo) {
        const isSiblingContracts = sameFeatureContracts(fileInfo.relative, targetInfo.relative);
        const isKernelSelfExport = fileInfo.featureName === "terminal-workspace-kernel"
          && targetInfo.featureName === "terminal-workspace-kernel"
          && (targetInfo.isCoreDomain || targetInfo.isCoreApplication);
        const isRuntimeHostKernelDependency = fileInfo.featureName === runtimeHostFeature
          && specifier === kernelContractsEntrypoint;

        if (!isSiblingContracts && !isKernelSelfExport && !isRuntimeHostKernelDependency) {
          violations.push(`${importRef}: contracts may import only same-feature contracts, kernel self-exports, or kernel public contracts`);
        }
      }
    }

    if (fileInfo.isCoreDomain || fileInfo.isCoreApplication) {
      if (specifier.startsWith("node:") || restrictedCorePackages.has(specifier)) {
        violations.push(`${importRef}: core layers must not depend on runtime or framework packages`);
      }

      if (targetInfo?.isRenderer || targetInfo?.isPreload || targetInfo?.isFeatureMain) {
        violations.push(`${importRef}: core layers must not depend on renderer/preload/main layers`);
      }

      if (fileInfo.isCoreDomain && targetInfo?.isCoreApplication) {
        violations.push(`${importRef}: core/domain must not depend on core/application`);
      }
    }

    if (fileInfo.isRendererUi) {
      if (specifier.startsWith("node:") || restrictedRendererPackages.has(specifier)) {
        violations.push(`${importRef}: renderer/ui must stay free of runtime adapters and store packages`);
      }

      if (targetInfo?.isRendererAdapter || targetInfo?.isRendererHook || targetInfo?.isRendererPresenter) {
        violations.push(`${importRef}: renderer/ui must not import hooks, presenters, or adapters directly`);
      }

      if (targetInfo?.isContracts || targetInfo?.isCoreDomain || targetInfo?.isCoreApplication) {
        violations.push(`${importRef}: renderer/ui must depend on renderer-level view models, not contracts or core layers`);
      }
    }

    if (fileInfo.isRendererCommand) {
      if (specifier.startsWith("node:") || restrictedRendererPackages.has(specifier)) {
        violations.push(`${importRef}: renderer/commands must stay free of runtime adapters and store packages`);
      }

      if (targetInfo?.isRendererUi || targetInfo?.isRendererPresenter) {
        violations.push(`${importRef}: renderer/commands must not depend on ui or presenters`);
      }
    }

    if (fileInfo.isFeatureMain && (targetInfo?.isRenderer || targetInfo?.isPreload)) {
      violations.push(`${importRef}: feature main must not depend on renderer or preload layers`);
    }
  }
}

if (violations.length > 0) {
  console.error("Architecture boundary violations found:\n");
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

console.log(`Architecture boundaries verified for ${files.length} source files.`);

async function collectReachableFiles(entrypointFiles) {
  const visited = new Set();
  const queued = [...entrypointFiles];

  while (queued.length > 0) {
    const filePath = queued.pop();
    if (!filePath || visited.has(filePath)) {
      continue;
    }

    visited.add(filePath);

    const source = await fs.readFile(filePath, "utf8");
    const specifiers = [...source.matchAll(importPattern)]
      .map((match) => match[1] ?? match[2] ?? match[3])
      .filter(Boolean);

    for (const specifier of specifiers) {
      const resolved = resolveProjectModule(filePath, specifier);
      if (resolved && resolved.startsWith(srcRoot) && !visited.has(resolved)) {
        queued.push(resolved);
      }
    }
  }

  return [...visited];
}

async function collectSourceFiles(directory) {
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...await collectSourceFiles(entryPath));
      continue;
    }

    if (supportedExtensions.includes(path.extname(entry.name))) {
      files.push(entryPath);
    }
  }

  return files;
}

function resolveProjectModule(fromFile, specifier) {
  if (specifier.startsWith("./") || specifier.startsWith("../")) {
    return resolveCandidate(path.resolve(path.dirname(fromFile), specifier));
  }

  if (specifier.startsWith("@features/")) {
    const featurePath = path.join(appRoot, "src/features", specifier.slice("@features/".length));
    return resolveCandidate(featurePath);
  }

  return null;
}

function resolveCandidate(candidateBase) {
  const normalized = candidateBase.replace(/\\/g, "/");
  const candidates = new Set([
    candidateBase,
    normalized.endsWith(".js") ? candidateBase.slice(0, -3) + ".ts" : candidateBase,
    normalized.endsWith(".js") ? candidateBase.slice(0, -3) + ".tsx" : candidateBase,
    normalized.endsWith(".js") ? candidateBase.slice(0, -3) + ".cts" : candidateBase,
    `${candidateBase}.ts`,
    `${candidateBase}.tsx`,
    `${candidateBase}.cts`,
    path.join(candidateBase, "index.ts"),
    path.join(candidateBase, "index.tsx"),
    path.join(candidateBase, "index.cts"),
  ]);

  for (const candidate of candidates) {
    try {
      const stat = fsSync.statSync(candidate);
      if (stat.isFile()) {
        return candidate;
      }
    } catch {
      // ignore missing candidates
    }
  }

  return null;
}

function describeFile(filePath) {
  const relative = path.relative(appRoot, filePath).replace(/\\/g, "/");
  const match = relative.match(/^src\/features\/([^/]+)\/(.+)$/);
  const featureName = match?.[1] ?? null;
  const featurePath = match?.[2] ?? null;

  return {
    relative,
    featureName,
    isFeatureInternal: Boolean(featureName),
    isContracts: Boolean(featurePath?.startsWith("contracts/")),
    isCoreDomain: Boolean(featurePath?.startsWith("core/domain/")),
    isCoreApplication: Boolean(featurePath?.startsWith("core/application/")),
    isFeatureMain: Boolean(featurePath?.startsWith("main/")),
    isPreload: Boolean(featurePath?.startsWith("preload/")),
    isRenderer: Boolean(featurePath?.startsWith("renderer/")),
    isRendererUi: Boolean(featurePath?.startsWith("renderer/ui/")),
    isRendererHook: Boolean(featurePath?.startsWith("renderer/hooks/")),
    isRendererAdapter: Boolean(featurePath?.startsWith("renderer/adapters/")),
    isRendererCommand: Boolean(featurePath?.startsWith("renderer/commands/")),
    isRendererPresenter: Boolean(featurePath?.startsWith("renderer/presenters/")),
    isRendererViewModel: Boolean(featurePath?.startsWith("renderer/view-models/")),
    isShell: relative.startsWith("src/host/") || relative.startsWith("src/renderer/app/"),
  };
}

function sameFeatureContracts(sourceRelative, targetRelative) {
  const sourceMatch = sourceRelative.match(/^src\/features\/([^/]+)\/contracts\//);
  const targetMatch = targetRelative.match(/^src\/features\/([^/]+)\/contracts\//);
  return Boolean(sourceMatch && targetMatch && sourceMatch[1] === targetMatch[1]);
}

function isPackageImport(specifier) {
  return !specifier.startsWith("./")
    && !specifier.startsWith("../")
    && !specifier.startsWith("@features/")
    && !specifier.startsWith("node:");
}
