async function main() {
  const sdk = require(process.env.TERMINAL_NODE_PACKAGE);

  let failedAsExpected = false;
  try {
    sdk.TerminalNodeClient.fromRuntimeSlug("incompatible-target");
  } catch (error) {
    failedAsExpected = String(error.message).includes(
      "manifest does not contain a compatible target",
    );
  }

  if (!failedAsExpected) {
    throw new Error("expected loader to reject incompatible manifest target");
  }

  process.stdout.write("incompatible-target-rejected");
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
