import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];

await checkPackageScripts();
await checkRequiredDocs();
await checkImplementationChecklist();

if (failures.length > 0) {
  throw new Error([
    "release readiness check failed:",
    ...failures.map((failure) => `- ${failure}`),
  ].join("\n"));
}

console.log("release readiness ok: release checklist, rollback plan, deprecation checkpoints, and gates are declared");

async function checkPackageScripts() {
  const packageJson = await readJson("package.json");
  const scripts = packageJson.scripts ?? {};

  requireScript(scripts, "check");
  requireScript(scripts, "check:release-readiness");
  requireScript(scripts, "test:public-api");
  requireScript(scripts, "test:packed-consumer");
  requireScript(scripts, "check:examples");

  if (typeof scripts.check === "string" && !scripts.check.includes("check:release-readiness")) {
    failures.push("package.json check script must include check:release-readiness");
  }
}

async function checkRequiredDocs() {
  await requireDoc("docs/release-checklist.md", [
    "## Release Candidate Checklist",
    "## Stable Promotion Checklist",
    "npm run check",
    "npm run test:public-api",
    "npm run test:packed-consumer",
    "npm run check:examples",
    "npm run check:release-readiness",
    "cargo run -p xtask -- verify-v1-readiness",
    "cargo run -p xtask -- verify-v1-readiness --require-recorded-passes",
    "browser matrix reviewed and recorded",
    "rollback owner",
  ]);

  await requireDoc("docs/rollback-plan.md", [
    "## Preview Release Rollback",
    "## Stable Release Rollback",
    "withdrawn or superseded",
    "published package versions",
    "compatibility matrix",
    "degraded semantics",
  ]);

  await requireDoc("docs/deprecation-checkpoints.md", [
    "## Deprecation Entry Template",
    "## Required Checkpoints",
    "at least one `MINOR` release",
    "next `MAJOR` release",
    "migration guidance",
    "Current Deprecations",
  ]);

  await requireDoc("docs/release-policy.md", [
    "Release Checklist",
    "Rollback Plan",
    "Deprecation Checkpoints",
    "npm run check:release-readiness",
  ]);

  await requireDoc("docs/build-and-ci-model.md", [
    "release readiness policy check",
    "npm run check:release-readiness",
  ]);

  await requireDoc("README.md", [
    "Release Checklist",
    "Rollback Plan",
    "Deprecation Checkpoints",
  ]);
}

async function checkImplementationChecklist() {
  const checklist = await readText("docs/implementation-checklist.md");

  for (const item of [
    "Confirm rollback plan for release cut",
    "Add deprecation policy checkpoints",
    "Add release checklist",
    "Verify production-grade release gates",
  ]) {
    if (!checklist.includes(`- [x] ${item}`)) {
      failures.push(`implementation checklist item is not checked: ${item}`);
    }
  }

  if (!checklist.includes("- [ ] Run browser test matrix")) {
    failures.push("implementation checklist must keep browser matrix unchecked until browser proof exists");
  }
}

function requireScript(scripts, name) {
  if (typeof scripts[name] !== "string" || scripts[name].length === 0) {
    failures.push(`package.json is missing script ${name}`);
  }
}

async function requireDoc(relativePath, requiredText) {
  const text = await readText(relativePath);

  for (const expected of requiredText) {
    if (!text.includes(expected)) {
      failures.push(`${relativePath} is missing required text: ${expected}`);
    }
  }
}

async function readText(relativePath) {
  return readFile(path.join(sdkRoot, relativePath), "utf8");
}

async function readJson(relativePath) {
  return JSON.parse(await readText(relativePath));
}
