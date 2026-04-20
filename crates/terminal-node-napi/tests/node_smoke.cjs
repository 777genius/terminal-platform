const assert = require("node:assert/strict");

function createClient(binding) {
  const kind = process.env.TERMINAL_NODE_ADDRESS_KIND;
  const value = process.env.TERMINAL_NODE_ADDRESS_VALUE;

  if (kind === "namespaced") {
    return binding.TerminalNodeClient.fromNamespacedAddress(value);
  }

  if (kind === "filesystem") {
    return binding.TerminalNodeClient.fromFilesystemPath(value);
  }

  throw new Error(`Unsupported address kind: ${kind}`);
}

async function main() {
  const binding = require(process.env.TERMINAL_NODE_ADDON);
  const client = createClient(binding);

  const version = client.bindingVersion();
  const handshake = await client.handshakeInfo();
  const created = await client.createNativeSession({ title: "node-smoke" });
  const listed = await client.listSessions();
  const attached = await client.attachSession(created.session_id);
  const topology = await client.topologySnapshot(created.session_id);
  const focusedScreen = await client.screenSnapshot(
    created.session_id,
    attached.focused_screen.pane_id,
  );

  assert.equal(typeof client.address, "string");
  assert.equal(version.protocol.major, 0);
  assert.equal(version.protocol.minor, 1);
  assert.equal(handshake.assessment.can_use, true);
  assert.equal(Array.isArray(handshake.handshake.available_backends), true);
  assert.equal(listed.some((session) => session.session_id === created.session_id), true);
  assert.equal(attached.session.session_id, created.session_id);
  assert.equal(attached.topology.session_id, created.session_id);
  assert.equal(topology.session_id, created.session_id);
  assert.equal(focusedScreen.pane_id, attached.focused_screen.pane_id);
  assert.equal(focusedScreen.surface.lines.length > 0, true);

  let invalidSessionFailed = false;
  try {
    await client.attachSession("not-a-uuid");
  } catch (error) {
    invalidSessionFailed = error.message.startsWith("invalid_session_id:");
  }
  assert.equal(invalidSessionFailed, true);

  process.stdout.write(
    JSON.stringify({
      session_id: created.session_id,
      pane_id: focusedScreen.pane_id,
      available_backends: handshake.handshake.available_backends.length,
    }),
  );
}

main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exit(1);
});
