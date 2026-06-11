import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(sdkRoot, "..");

const matrixPath = path.join(sdkRoot, "docs/compatibility-matrix.md");
const protocolConstantsPath = path.join(
  repoRoot,
  "crates/terminal-domain/src/version_info/constants.rs",
);

const protocolVersion = await readCurrentProtocolVersion(protocolConstantsPath);
const expectedRow = [
  protocolVersion,
  await readPackageVersion("packages/runtime-types/package.json"),
  await readPackageVersion("packages/workspace-contracts/package.json"),
  await readPackageVersion("packages/workspace-core/package.json"),
  await readPackageVersion("packages/workspace-elements/package.json"),
  await readPackageVersion("packages/workspace-react/package.json"),
  "preview",
];
const matrix = await readFile(matrixPath, "utf8");
const rows = matrix.split("\n")
  .filter((line) => line.startsWith("| "))
  .map(parseMarkdownTableRow);
const dataRows = rows.filter((row) => row[0] !== "---" && row[0] !== "Runtime Protocol");
const matchingRow = dataRows.find((row) =>
  expectedRow.every((expectedCell, index) => row[index] === expectedCell),
);

if (!matchingRow) {
  throw new Error([
    "compatibility matrix is not aligned with current runtime/package versions",
    `expected prefix: | ${expectedRow.join(" | ")} |`,
    `file: ${path.relative(repoRoot, matrixPath)}`,
  ].join("\n"));
}

console.log(`compatibility matrix ok: ${expectedRow.join(" / ")}`);

async function readCurrentProtocolVersion(filePath) {
  const source = await readFile(filePath, "utf8");
  const major = readRustConstU16(source, "CURRENT_PROTOCOL_MAJOR");
  const minor = readRustConstU16(source, "CURRENT_PROTOCOL_MINOR");

  return `${major}.${minor}`;
}

async function readPackageVersion(relativePath) {
  const packageJson = JSON.parse(await readFile(path.join(sdkRoot, relativePath), "utf8"));
  const version = packageJson.version;

  if (typeof version !== "string" || version.length === 0) {
    throw new Error(`${relativePath} is missing a package version`);
  }

  return version;
}

function readRustConstU16(source, constName) {
  const pattern = new RegExp(`pub const ${constName}: u16 = (\\d+);`);
  const match = source.match(pattern);

  if (!match?.[1]) {
    throw new Error(`missing Rust protocol constant ${constName}`);
  }

  return Number.parseInt(match[1], 10);
}

function parseMarkdownTableRow(row) {
  return row
    .split("|")
    .slice(1, -1)
    .map((cell) => cell.trim());
}
