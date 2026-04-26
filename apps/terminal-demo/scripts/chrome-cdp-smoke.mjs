import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import process from "node:process";

export async function launchChromeWithCdp({
  appRoot,
  binaryMissingMessage,
  cdpPort,
  extraArgs = [],
  headlessModeEnv,
  logPrefix,
  profilePrefix,
  readyTimeoutMs = 20_000,
}) {
  const chromeBinary = resolveChromeBinary({ appRoot, binaryMissingMessage });
  const chromeVersion = resolveChromeVersion({ appRoot, chromeBinary });
  const chromeLaunchModes = resolveChromeLaunchModes({ envName: headlessModeEnv });
  const failures = [];

  for (const headlessMode of chromeLaunchModes) {
    const userDataDir = path.join("/tmp", `${profilePrefix}-${process.pid}-${Date.now()}-${headlessMode}`);
    const child = spawn(chromeBinary, buildChromeArgs({
      cdpPort,
      extraArgs,
      headlessMode,
      userDataDir,
    }), {
      cwd: appRoot,
      env: process.env,
      stdio: "pipe",
    });
    const readOutput = pipeProcess(child, `[${logPrefix}:${headlessMode}]`);

    try {
      await waitForHttpServer(`http://127.0.0.1:${cdpPort}/json/version`, {
        child,
        label: `Chrome CDP (${headlessMode})`,
        timeoutMs: readyTimeoutMs,
      });
      process.stdout.write(`Chrome CDP ready - ${chromeVersion} (${headlessMode} headless)\n`);
      return {
        child,
        chromeBinary,
        chromeVersion,
        headlessMode,
        userDataDir,
      };
    } catch (error) {
      failures.push(formatChromeLaunchFailure({
        error,
        headlessMode,
        output: readOutput(),
        version: chromeVersion,
      }));
      await stopProcess(child);
      await fs.rm(userDataDir, { recursive: true, force: true });
    }
  }

  throw new Error([
    `Chrome CDP did not become ready after ${chromeLaunchModes.length} launch attempt(s).`,
    `Binary: ${chromeBinary}`,
    `Version: ${chromeVersion}`,
    "Launch failures:",
    failures.join("\n\n"),
  ].join("\n"));
}

export async function waitForHttpServer(url, options = {}) {
  const startedAt = Date.now();
  const timeoutMs = options.timeoutMs ?? 20_000;

  while (Date.now() - startedAt < timeoutMs) {
    const exitState = processExitState(options.child);
    if (exitState) {
      throw new Error(`${options.label ?? "Server"} exited before ${url} became ready - ${exitState}`);
    }

    try {
      const response = await fetch(url, { method: "GET" });
      if (response.ok) {
        return;
      }
    } catch {
      // Server is still starting.
    }

    await sleep(200);
  }

  throw new Error(`Timed out waiting for ${url}`);
}

export async function stopProcess(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) {
    return;
  }

  const exited = new Promise((resolve) => {
    child.once("exit", () => resolve());
  });
  child.kill("SIGTERM");
  await Promise.race([exited, sleep(5_000)]);
  if (child.exitCode === null && child.signalCode === null) {
    child.kill("SIGKILL");
  }
}

export function pipeProcess(child, prefix) {
  const chunks = [];
  const capture = (stream, chunk) => {
    const text = chunk.toString();
    chunks.push(`${stream}: ${text}`);
    if (chunks.join("").length > 8_000) {
      chunks.splice(0, chunks.length - 20);
    }
    return text;
  };

  child.stdout?.on("data", (chunk) => {
    process.stdout.write(`${prefix} ${capture("stdout", chunk)}`);
  });
  child.stderr?.on("data", (chunk) => {
    process.stderr.write(`${prefix} ${capture("stderr", chunk)}`);
  });

  return () => chunks.join("").slice(-4_000).trim();
}

export function resolveRuntimeEvaluationValue(result) {
  if (result.exceptionDetails) {
    throw new Error(`Browser evaluation failed: ${formatRuntimeException(result.exceptionDetails)}`);
  }

  return result.result?.value;
}

function buildChromeArgs({ cdpPort, extraArgs, headlessMode, userDataDir }) {
  const headlessFlag = headlessMode === "old" ? "--headless" : `--headless=${headlessMode}`;
  return [
    headlessFlag,
    `--remote-debugging-port=${cdpPort}`,
    `--user-data-dir=${userDataDir}`,
    ...extraArgs,
    "about:blank",
  ];
}

function formatChromeLaunchFailure({ error, headlessMode, output, version }) {
  const outputSection = output
    ? `\nLast Chrome output:\n${indent(output)}`
    : "\nLast Chrome output: <empty>";
  return [
    `- headless mode: ${headlessMode}`,
    `  version: ${version}`,
    `  reason: ${error.message}`,
    outputSection,
  ].join("\n");
}

function formatRuntimeException(exceptionDetails) {
  const text = exceptionDetails.text ?? "Runtime exception";
  const description = exceptionDetails.exception?.description ?? exceptionDetails.exception?.value ?? "";
  const location = [
    exceptionDetails.url,
    Number.isInteger(exceptionDetails.lineNumber) ? `:${exceptionDetails.lineNumber + 1}` : "",
    Number.isInteger(exceptionDetails.columnNumber) ? `:${exceptionDetails.columnNumber + 1}` : "",
  ].join("");
  return [text, description, location].filter(Boolean).join(" - ");
}

function processExitState(child) {
  if (!child) {
    return null;
  }

  if (child.exitCode !== null) {
    return `exit code ${child.exitCode}`;
  }

  if (child.signalCode !== null) {
    return `signal ${child.signalCode}`;
  }

  return null;
}

function resolveChromeBinary({ appRoot, binaryMissingMessage }) {
  const candidates = [
    process.env.TERMINAL_DEMO_CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    resolveBinaryFromShell({ appRoot, name: "google-chrome" }),
    resolveBinaryFromShell({ appRoot, name: "chromium" }),
    resolveBinaryFromShell({ appRoot, name: "chromium-browser" }),
  ].filter(Boolean);

  const binary = candidates[0];
  if (!binary) {
    throw new Error(binaryMissingMessage);
  }

  return binary;
}

function resolveBinaryFromShell({ appRoot, name }) {
  const result = spawnSync("bash", ["-lc", `command -v ${name}`], {
    cwd: appRoot,
    env: process.env,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    return null;
  }

  const value = result.stdout.trim();
  return value.length > 0 ? value : null;
}

function resolveChromeLaunchModes({ envName }) {
  const rawValue = process.env[envName] ?? "new,old";
  const values = rawValue
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
  const allowed = new Set(["new", "old"]);
  const invalid = values.filter((value) => !allowed.has(value));
  if (invalid.length > 0) {
    throw new Error(`Unsupported ${envName} value(s): ${invalid.join(", ")}. Use new, old, or new,old.`);
  }
  return values.length > 0 ? values : ["new", "old"];
}

function resolveChromeVersion({ appRoot, chromeBinary }) {
  const result = spawnSync(chromeBinary, ["--version"], {
    cwd: appRoot,
    env: process.env,
    encoding: "utf8",
  });
  const version = `${result.stdout ?? ""}${result.stderr ?? ""}`.trim();
  return version || "unknown";
}

function indent(value) {
  return value
    .split("\n")
    .map((line) => `    ${line}`)
    .join("\n");
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
