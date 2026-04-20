const fs = require("node:fs/promises");
const { runShutdownSmoke } = require("./smoke_flow.cjs");

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

async function main() {
  const sdk = require(process.env.TERMINAL_NODE_PACKAGE);
  const result = await runShutdownSmoke(() => createClient(sdk), {
    onReady: async () => {
      await fs.writeFile(process.env.TERMINAL_NODE_READY_FILE, "ready\n");
    },
  });
  process.stdout.write(JSON.stringify(result));
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
