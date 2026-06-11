import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const examplesRoot = path.join(sdkRoot, "examples");
const workspacePackagePaths = await readWorkspacePackagePaths();
const publicEntrypoints = await readPublicEntrypoints(workspacePackagePaths);
const exampleFiles = await collectExampleSourceFiles(examplesRoot);
const failures = [];

for (const filePath of exampleFiles) {
  const sourceText = await readFile(filePath, "utf8");
  const sourceFile = ts.createSourceFile(
    filePath,
    sourceText,
    ts.ScriptTarget.Latest,
    true,
    scriptKindForPath(filePath),
  );

  collectImportSpecifiers(sourceFile, (specifier) => {
    validateImportSpecifier(filePath, specifier);
  });
}

if (failures.length > 0) {
  throw new Error([
    "example public import check failed:",
    ...failures.map((failure) => `- ${failure}`),
  ].join("\n"));
}

console.log(`example public imports ok: ${exampleFiles.length} files`);

async function readWorkspacePackagePaths() {
  const packageJson = await readJson(path.join(sdkRoot, "package.json"));
  const workspaces = packageJson.workspaces;

  if (!Array.isArray(workspaces)) {
    throw new Error("sdk/package.json must define a workspace package list");
  }

  return workspaces.map((workspacePath) => path.join(sdkRoot, workspacePath));
}

async function readPublicEntrypoints(packagePaths) {
  const entrypoints = new Set();

  for (const packagePath of packagePaths) {
    const packageJson = await readJson(path.join(packagePath, "package.json"));

    if (typeof packageJson.name !== "string" || !packageJson.name.startsWith("@terminal-platform/")) {
      throw new Error(`workspace package has invalid name: ${packagePath}`);
    }

    if (!packageJson.exports || typeof packageJson.exports !== "object") {
      throw new Error(`workspace package is missing explicit exports: ${packageJson.name}`);
    }

    for (const exportPath of Object.keys(packageJson.exports)) {
      entrypoints.add(exportPath === "." ? packageJson.name : `${packageJson.name}/${exportPath.slice(2)}`);
    }
  }

  return entrypoints;
}

async function collectExampleSourceFiles(root) {
  const files = [];
  await collectSourceFiles(root, files);
  return files.sort();
}

async function collectSourceFiles(directory, files) {
  const entries = await readdir(directory, { withFileTypes: true });

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      await collectSourceFiles(entryPath, files);
      continue;
    }

    if (isSourceFile(entryPath)) {
      files.push(entryPath);
    }
  }
}

function collectImportSpecifiers(sourceFile, visitSpecifier) {
  visit(sourceFile);

  function visit(node) {
    if (
      (ts.isImportDeclaration(node) || ts.isExportDeclaration(node))
      && node.moduleSpecifier
      && ts.isStringLiteral(node.moduleSpecifier)
    ) {
      visitSpecifier(node.moduleSpecifier.text);
    }

    if (
      ts.isImportTypeNode(node)
      && node.argument
      && ts.isLiteralTypeNode(node.argument)
      && ts.isStringLiteral(node.argument.literal)
    ) {
      visitSpecifier(node.argument.literal.text);
    }

    if (
      ts.isCallExpression(node)
      && node.arguments.length > 0
      && ts.isStringLiteralLike(node.arguments[0])
      && (node.expression.kind === ts.SyntaxKind.ImportKeyword
        || (ts.isIdentifier(node.expression) && node.expression.text === "require"))
    ) {
      visitSpecifier(node.arguments[0].text);
    }

    ts.forEachChild(node, visit);
  }
}

function validateImportSpecifier(filePath, specifier) {
  if (isForbiddenSpecifier(specifier)) {
    failures.push(`${relative(filePath)} imports forbidden SDK internals: ${specifier}`);
    return;
  }

  if (specifier.startsWith("@terminal-platform/") && !publicEntrypoints.has(specifier)) {
    failures.push(`${relative(filePath)} imports undocumented SDK entrypoint: ${specifier}`);
    return;
  }

  if (specifier.startsWith(".")) {
    const resolved = path.resolve(path.dirname(filePath), specifier);
    if (isForbiddenResolvedPath(resolved)) {
      failures.push(`${relative(filePath)} reaches outside examples through ${specifier}`);
    }
  }
}

function isForbiddenSpecifier(specifier) {
  return specifier.includes("/src/")
    || specifier.includes("/dist/")
    || specifier.includes("apps/terminal-demo")
    || specifier.includes("packages/");
}

function isForbiddenResolvedPath(resolvedPath) {
  const segments = path.normalize(resolvedPath).split(path.sep);
  return segments.includes("packages") || hasAdjacentSegments(segments, "apps", "terminal-demo");
}

function isSourceFile(filePath) {
  return [".js", ".jsx", ".mjs", ".ts", ".tsx"].includes(path.extname(filePath));
}

function scriptKindForPath(filePath) {
  if (filePath.endsWith(".tsx") || filePath.endsWith(".jsx")) {
    return ts.ScriptKind.TSX;
  }

  if (filePath.endsWith(".js") || filePath.endsWith(".mjs")) {
    return ts.ScriptKind.JS;
  }

  return ts.ScriptKind.TS;
}

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

function hasAdjacentSegments(segments, first, second) {
  return segments.some((segment, index) => segment === first && segments[index + 1] === second);
}

function relative(filePath) {
  return path.relative(sdkRoot, filePath);
}
