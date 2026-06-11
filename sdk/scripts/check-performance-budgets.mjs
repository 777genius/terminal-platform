import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const sdkRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const budgetsPath = path.join(sdkRoot, "docs/performance-budgets.md");

const requiredAreas = [
  "screen-render-smoke",
  "overlay-updates",
  "adapter-reconnect-churn",
  "subscription-fan-out",
];

const budgets = await readFile(budgetsPath, "utf8");
const rows = budgets.split("\n")
  .filter((line) => line.startsWith("| "))
  .map(parseMarkdownTableRow);
const dataRows = rows.filter((row) => row[0] !== "---" && row[0] !== "Area");
const rowsByArea = new Map(dataRows.map((row) => [row[0], row]));
const failures = [];

for (const area of requiredAreas) {
  const row = rowsByArea.get(area);

  if (!row) {
    failures.push(`missing performance budget area "${area}"`);
    continue;
  }

  const [, metric, previewBudget, evidenceCommand, notes] = row;

  if (!hasContent(metric)) {
    failures.push(`area "${area}" is missing a metric`);
  }

  if (!hasContent(previewBudget)) {
    failures.push(`area "${area}" is missing a preview budget`);
  }

  if (!hasContent(evidenceCommand)) {
    failures.push(`area "${area}" is missing an evidence command`);
  }

  if (!hasContent(notes)) {
    failures.push(`area "${area}" is missing notes`);
  }
}

if (failures.length > 0) {
  throw new Error([
    "performance budget check failed:",
    ...failures.map((failure) => `- ${failure}`),
  ].join("\n"));
}

console.log(`performance budgets ok: ${requiredAreas.length} areas`);

function parseMarkdownTableRow(row) {
  return row
    .split("|")
    .slice(1, -1)
    .map((cell) => cell.trim());
}

function hasContent(value) {
  return typeof value === "string" && value.length > 0 && value !== "TBD";
}
