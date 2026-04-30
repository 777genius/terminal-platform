const { runPackageWatchSmoke, runSmoke } = require("./smoke_flow.cjs");

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
  const clients = new Set();
  const createTrackedClient = () => {
    const client = createClient(sdk);
    clients.add(client);
    return client;
  };

  try {
    await runSmoke(createTrackedClient, sdk);
    await runPackageWatchSmoke(createTrackedClient, sdk);
  } finally {
    await Promise.allSettled(
      Array.from(clients, (client) =>
        typeof client.close === "function" ? client.close() : Promise.resolve(),
      ),
    );
  }
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
