import { generateRuntimeTypes, resolveRuntimeTypesPaths, snapshotDirectory } from "./generate-runtime-types.mjs";

const { rawDir } = resolveRuntimeTypesPaths(import.meta.url);
const before = snapshotDirectory(rawDir);

await generateRuntimeTypes(import.meta.url);

const after = snapshotDirectory(rawDir);

if (!mapsEqual(before, after)) {
  console.error("runtime-types generated output drifted. Re-run npm run generate and commit the results.");
  process.exitCode = 1;
}

function mapsEqual(left, right) {
  if (left.size !== right.size) {
    return false;
  }

  for (const [key, leftValue] of left) {
    if (right.get(key) !== leftValue) {
      return false;
    }
  }

  return true;
}
