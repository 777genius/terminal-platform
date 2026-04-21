#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, "..");

function main() {
  const options = parseArgs(process.argv.slice(2));
  const buildArgs = ["./scripts/build-local-package.mjs", "--out", options.out];

  if (options.release) {
    buildArgs.push("--release");
  }

  run("node", buildArgs, packageDir);
  run("node", ["./scripts/verify-package.mjs", "--package-dir", options.out], packageDir);

  const packResult = spawnSync("npm", ["pack", "--json"], {
    cwd: options.out,
    env: packageManagerEnv(options),
    encoding: "utf8",
    stdio: ["ignore", "pipe", "inherit"],
  });

  if (packResult.status !== 0) {
    throw new Error(`npm pack failed with exit code ${packResult.status}`);
  }

  const payload = JSON.parse(packResult.stdout);
  const filename = payload[0]?.filename;
  if (!filename) {
    throw new Error("npm pack did not return a filename");
  }

  process.stdout.write(`${path.join(options.out, filename)}\n`);
}

function parseArgs(argv) {
  const options = {
    out: path.join(packageDir, "artifacts", "local"),
    release: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--out") {
      options.out = path.resolve(readFlagValue(argv, index, arg));
      index += 1;
      continue;
    }

    if (arg === "--release") {
      options.release = true;
      continue;
    }

    throw new Error(`Unsupported argument: ${arg}`);
  }

  return options;
}

function readFlagValue(argv, index, flag) {
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`Missing value for ${flag}`);
  }
  return value;
}

function run(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: "inherit",
  });

  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

function packageManagerEnv(options) {
  return {
    ...process.env,
    npm_config_cache: process.env.npm_config_cache ?? path.join(options.out, ".npm-cache"),
  };
}

main();
