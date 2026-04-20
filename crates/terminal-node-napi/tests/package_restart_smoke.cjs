const fs = require("node:fs/promises");
const { runRestartRecoverySmoke } = require("./smoke_flow.cjs");

function createClient(sdk) {
  const kind = process.env.TERMINAL_NODE_ADDRESS_KIND;
  const value = process.env.TERMINAL_NODE_ADDRESS_VALUE;

  if (kind === "namespaced") {
    return sdk.TerminalNodeClient.fromNamespacedAddress(value);
  }

  if (kind === "filesystem") {
    return sdk.TerminalNodeClient.fromFilesystemPath(value);
  }

  throw new Error(`Unsupported address kind: ${kind}`);
}

async function waitForFile(path, label) {
  for (let attempt = 0; attempt < 200; attempt += 1) {
    try {
      await fs.access(path);
      return;
    } catch (_error) {
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  throw new Error(`Timed out waiting for ${label}: ${path}`);
}

async function main() {
  const sdk = require(process.env.TERMINAL_NODE_PACKAGE);
  const result = await runRestartRecoverySmoke(() => createClient(sdk), {
    onInitialReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_INITIAL_READY_FILE, "ready\n");
    },
    waitForStop: async () => {
      await waitForFile(process.env.TERMINAL_NODE_STOP_FILE, "daemon stop signal");
    },
    onStaleObserved: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_STALE_READY_FILE, "stale\n");
    },
    waitForRestart: async () => {
      await waitForFile(process.env.TERMINAL_NODE_RESTART_FILE, "daemon restart signal");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
