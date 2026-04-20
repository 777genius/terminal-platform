const assert = require("node:assert/strict");

async function runSmoke(createClient) {
  const client = createClient();

  const version = client.bindingVersion();
  const handshake = await client.handshakeInfo();
  const nativeCapabilities = await client.backendCapabilities("native");
  const tmuxCapabilities = await client.backendCapabilities("tmux");
  const zellijCapabilities = await client.backendCapabilities("zellij");
  const created = await client.createNativeSession({
    title: "node-smoke",
    launch: {
      program: "/bin/sh",
      args: ["-lc", "printf 'ready\\n'; exec cat"],
    },
  });
  const listed = await client.listSessions();
  const attached = await client.attachSession(created.session_id);
  const topology = await client.topologySnapshot(created.session_id);
  const focusedScreen = await client.screenSnapshot(
    created.session_id,
    attached.focused_screen.pane_id,
  );
  const topologySubscription = await client.openSubscription(created.session_id, {
    kind: "session_topology",
  });
  const initialTopologyEvent = await topologySubscription.nextEvent();
  const paneSubscription = await client.openSubscription(created.session_id, {
    kind: "pane_surface",
    pane_id: attached.focused_screen.pane_id,
  });
  const initialPaneEvent = await paneSubscription.nextEvent();
  const save = await client.dispatchMuxCommand(created.session_id, {
    kind: "save_session",
  });
  const saved = await client.listSavedSessions();
  const savedRecord = await client.savedSession(created.session_id);
  const sendInput = await client.dispatchMuxCommand(created.session_id, {
    kind: "send_input",
    pane_id: attached.focused_screen.pane_id,
    data: "hello from node smoke\r",
  });
  const screenAfterInput = await waitForLine(
    client,
    created.session_id,
    attached.focused_screen.pane_id,
    "hello from node smoke",
  );
  const screenDelta = await client.screenDelta(
    created.session_id,
    attached.focused_screen.pane_id,
    focusedScreen.sequence,
  );
  const newTab = await client.dispatchMuxCommand(created.session_id, {
    kind: "new_tab",
    title: "logs",
  });
  const topologyAfterDispatch = await client.topologySnapshot(created.session_id);
  const topologyUpdate = await waitForTopologyTabs(topologySubscription, 2);
  const restored = await client.restoreSavedSession(created.session_id);
  const deleted = await client.deleteSavedSession(created.session_id);
  const savedAfterDelete = await client.listSavedSessions();

  assert.equal(typeof client.address, "string");
  assert.equal(version.protocol.major, 0);
  assert.equal(version.protocol.minor, 1);
  assert.equal(handshake.assessment.can_use, true);
  assert.equal(Array.isArray(handshake.handshake.available_backends), true);
  assert.equal(nativeCapabilities.backend, "native");
  assert.equal(nativeCapabilities.capabilities.explicit_session_save, true);
  assert.equal(tmuxCapabilities.backend, "tmux");
  assert.equal(tmuxCapabilities.capabilities.read_only_client_mode, true);
  assert.equal(zellijCapabilities.backend, "zellij");
  assert.equal(zellijCapabilities.capabilities.tab_create, false);
  assert.equal(listed.some((session) => session.session_id === created.session_id), true);
  assert.equal(attached.session.session_id, created.session_id);
  assert.equal(attached.topology.session_id, created.session_id);
  assert.equal(topology.session_id, created.session_id);
  assert.equal(focusedScreen.pane_id, attached.focused_screen.pane_id);
  assert.equal(focusedScreen.surface.lines.length > 0, true);
  assert.equal(initialTopologyEvent.kind, "topology_snapshot");
  assert.equal(initialTopologyEvent.session_id, created.session_id);
  assert.equal(initialPaneEvent.kind, "screen_delta");
  assert.equal(initialPaneEvent.pane_id, attached.focused_screen.pane_id);
  assert.equal(save.changed, false);
  assert.equal(saved.some((session) => session.session_id === created.session_id), true);
  assert.equal(savedRecord.session_id, created.session_id);
  assert.equal(savedRecord.compatibility.can_restore, true);
  assert.equal(
    screenAfterInput.surface.lines.some((line) =>
      line.text.includes("hello from node smoke"),
    ),
    true,
  );
  assert.equal(screenDelta.pane_id, attached.focused_screen.pane_id);
  assert.equal(screenDelta.to_sequence >= screenDelta.from_sequence, true);
  assert.equal(screenDelta.patch !== null || screenDelta.full_replace !== null, true);
  assert.equal(newTab.changed, true);
  assert.equal(topologyAfterDispatch.tabs.length, 2);
  assert.equal(topologyUpdate.kind, "topology_snapshot");
  assert.equal(topologyUpdate.tabs.length, 2);
  assert.equal(restored.saved_session_id, created.session_id);
  assert.equal(restored.session.session_id === created.session_id, false);
  assert.equal(deleted.session_id, created.session_id);
  assert.equal(savedAfterDelete.some((session) => session.session_id === created.session_id), false);

  await topologySubscription.close();
  await paneSubscription.close();

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
      saved_session_id: savedRecord.session_id,
    }),
  );
}

async function runPackageWatchSmoke(createClient, sdk) {
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-package-watch",
    launch: {
      program: "/bin/sh",
      args: ["-lc", "printf 'ready\\n'; exec cat"],
    },
  });
  const attached = await client.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;

  let dispatchedTopology = false;
  const topologyEvents = [];
  const topologyAbort = new AbortController();
  await client.watchTopology(created.session_id, {
    signal: topologyAbort.signal,
    onEvent: async (event) => {
      topologyEvents.push(event);
      if (!dispatchedTopology && event.kind === "topology_snapshot") {
        dispatchedTopology = true;
        await client.dispatchMuxCommand(created.session_id, {
          kind: "new_tab",
          title: "watch",
        });
        return;
      }

      if (event.kind === "topology_snapshot" && event.tabs.length === 2) {
        topologyAbort.abort();
      }
    },
  });

  const paneSubscription = await client.subscribePane(created.session_id, paneId);
  const paneEvents = [];
  const paneAbort = new AbortController();
  const panePump = paneSubscription.pump({
    signal: paneAbort.signal,
    onEvent: async (event) => {
      paneEvents.push(event);
      if (
        event.kind === "screen_delta" &&
        deltaContainsText(event, "package watch input")
      ) {
        paneAbort.abort();
      }
    },
  });
  await client.dispatchMuxCommand(created.session_id, {
    kind: "send_input",
    pane_id: paneId,
    data: "package watch input\r",
  });
  await panePump;

  assert.equal(
    topologyEvents.some(
      (event) => event.kind === "topology_snapshot" && event.tabs.length === 2,
    ),
    true,
  );
  assert.equal(
    paneEvents.some(
      (event) =>
        event.kind === "screen_delta" &&
        deltaContainsText(event, "package watch input"),
    ),
    true,
  );

  const baseState = sdk.createSessionState(attached);
  const paneDelta = await client.screenDelta(
    created.session_id,
    paneId,
    attached.focused_screen.sequence,
  );
  const reducedScreen = sdk.applyScreenDelta(attached.focused_screen, paneDelta);
  const reducedState = sdk.reduceSessionWatchEvent(baseState, {
    kind: "screen_delta",
    delta: paneDelta,
  });

  assert.equal(baseState.session.session_id, created.session_id);
  assert.equal(baseState.focusedScreen.pane_id, paneId);
  assert.equal(reducedScreen.pane_id, paneId);
  assert.equal(
    reducedScreen.surface.lines.some((line) => line.text.includes("package watch input")),
    true,
  );
  assert.equal(
    reducedState.focusedScreen.surface.lines.some((line) =>
      line.text.includes("package watch input"),
    ),
    true,
  );

  const sessionCreated = await client.createNativeSession({
    title: "node-package-session-watch",
    launch: {
      program: "/bin/sh",
      args: ["-lc", "printf 'ready\\n'; exec cat"],
    },
  });
  const sessionAttached = await client.attachSession(sessionCreated.session_id);
  const sessionPaneId = sessionAttached.focused_screen.pane_id;
  const sessionStates = [];
  let sessionDispatched = false;
  let sessionOpenedTab = false;
  const sessionAbort = new AbortController();
  await client.watchSessionState(sessionCreated.session_id, {
    signal: sessionAbort.signal,
    onState: async (state) => {
      sessionStates.push(state);

      if (
        !sessionDispatched &&
        state.focusedScreen &&
        state.topology.tabs.length === 1
      ) {
        sessionDispatched = true;
        await client.dispatchMuxCommand(sessionCreated.session_id, {
          kind: "send_input",
          pane_id: sessionPaneId,
          data: "package session watch input\r",
        });
        return;
      }

      if (
        !sessionOpenedTab &&
        state.focusedScreen?.surface.lines.some((line) =>
          line.text.includes("package session watch input"),
        )
      ) {
        sessionOpenedTab = true;
        await client.dispatchMuxCommand(sessionCreated.session_id, {
          kind: "new_tab",
          title: "watch-state",
        });
        return;
      }

      if (sessionOpenedTab && state.topology.tabs.length === 2) {
        sessionAbort.abort();
      }
    },
  });

  assert.equal(sessionStates.length > 0, true);
  assert.equal(
    sessionStates.some((state) =>
      state.focusedScreen?.surface.lines.some((line) =>
        line.text.includes("package session watch input"),
      ),
    ),
    true,
  );
  assert.equal(
    sessionStates.some((state) => state.topology.tabs.length === 2),
    true,
  );
  assert.equal(
    sessionStates.every((state) => {
      const expectedPaneId = focusedPaneIdFromTopology(state.topology);
      return expectedPaneId ? state.focusedScreen?.pane_id === expectedPaneId : true;
    }),
    true,
  );
}

async function waitForLine(client, sessionId, paneId, needle) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const snapshot = await client.screenSnapshot(sessionId, paneId);
    if (snapshot.surface.lines.some((line) => line.text.includes(needle))) {
      return snapshot;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error(`Timed out waiting for screen line: ${needle}`);
}

async function waitForTopologyTabs(subscription, tabCount) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const event = await subscription.nextEvent();
    if (event == null) {
      break;
    }
    if (event.kind === "topology_snapshot" && event.tabs.length === tabCount) {
      return event;
    }
  }

  throw new Error(`Timed out waiting for topology with ${tabCount} tabs`);
}

function deltaContainsText(delta, needle) {
  return (
    delta.patch?.line_updates?.some((line) => line.line.text.includes(needle)) ||
    delta.full_replace?.lines?.some((line) => line.text.includes(needle)) ||
    false
  );
}

function firstPaneId(node) {
  if (!node) {
    return null;
  }

  if (node.kind === "leaf") {
    return node.pane_id;
  }

  return firstPaneId(node.first) ?? firstPaneId(node.second);
}

function focusedPaneIdFromTopology(topology) {
  const focusedTab =
    topology.tabs.find((tab) => tab.tab_id === topology.focused_tab) ?? topology.tabs[0];

  if (!focusedTab) {
    return null;
  }

  return focusedTab.focused_pane ?? firstPaneId(focusedTab.root);
}

module.exports = {
  runPackageWatchSmoke,
  runSmoke,
};
