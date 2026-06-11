import { execFile } from "node:child_process";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const keepTemp = process.env.TERMINAL_PLATFORM_KEEP_PACKED_SMOKE === "1";

const workspacePackagePaths = [
  "packages/foundation",
  "packages/runtime-types",
  "packages/design-tokens",
  "packages/workspace-contracts",
  "packages/workspace-core",
  "packages/workspace-adapter-websocket",
  "packages/workspace-adapter-preload",
  "packages/workspace-adapter-memory",
  "packages/workspace-elements",
  "packages/workspace-react",
  "packages/testing",
];

const tempRoot = await mkdtemp(path.join(tmpdir(), "terminal-platform-packed-consumer-"));

try {
  const tarballDir = path.join(tempRoot, "tarballs");
  const consumerDir = path.join(tempRoot, "consumer");
  await mkdir(tarballDir, { recursive: true });
  await mkdir(consumerDir, { recursive: true });

  const tarballs = [];
  for (const packagePath of workspacePackagePaths) {
    tarballs.push(await packWorkspacePackage(packagePath, tarballDir));
  }

  await writeFile(
    path.join(consumerDir, "package.json"),
    `${JSON.stringify({
      private: true,
      type: "module",
      name: "terminal-platform-packed-consumer-smoke",
    }, null, 2)}\n`,
  );

  await run("npm", [
    "install",
    "--ignore-scripts",
    "--package-lock=false",
    "--audit=false",
    "--fund=false",
    ...tarballs,
  ], { cwd: consumerDir });

  await writeFile(path.join(consumerDir, "smoke.mjs"), createConsumerSmokeScript());
  const smoke = await run("node", ["smoke.mjs"], { cwd: consumerDir });
  process.stdout.write(smoke.stdout);

  console.log(`packed consumer smoke passed for ${tarballs.length} packages`);
} finally {
  if (keepTemp) {
    console.log(`kept packed consumer smoke temp dir: ${tempRoot}`);
  } else {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

async function packWorkspacePackage(packagePath, tarballDir) {
  const packageDir = path.join(sdkRoot, packagePath);
  const { stdout } = await run("npm", [
    "pack",
    "--json",
    "--pack-destination",
    tarballDir,
  ], { cwd: packageDir });
  const packResult = JSON.parse(stdout);
  const tarballName = packResult[0]?.filename;

  if (typeof tarballName !== "string" || tarballName.length === 0) {
    throw new Error(`npm pack did not report a tarball for ${packagePath}`);
  }

  return path.join(tarballDir, tarballName);
}

async function run(command, args, options) {
  try {
    return await execFileAsync(command, args, {
      cwd: options.cwd,
      maxBuffer: 10 * 1024 * 1024,
    });
  } catch (error) {
    const stdout = error.stdout ? `\nstdout:\n${error.stdout}` : "";
    const stderr = error.stderr ? `\nstderr:\n${error.stderr}` : "";
    throw new Error(`command failed: ${command} ${args.join(" ")}${stdout}${stderr}`);
  }
}

function createConsumerSmokeScript() {
  return `import {
  assertTerminalPlatformPackedConsumerSmoke,
  createWorkspaceTestHarness,
} from "@terminal-platform/testing";

const result = await assertTerminalPlatformPackedConsumerSmoke((specifier) => import(specifier));

if (result.checked < 20) {
  throw new Error(\`packed consumer smoke covered too few entrypoints: \${result.checked}\`);
}

const harness = createWorkspaceTestHarness();
await harness.bootstrap();

if (harness.kernel.getSnapshot().connection.state !== "ready") {
  throw new Error("packed consumer harness did not bootstrap");
}

await harness.dispose();

console.log(\`packed consumer smoke checked \${result.checked} public entrypoints\`);
`;
}
