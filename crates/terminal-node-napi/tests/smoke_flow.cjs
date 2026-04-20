const assert = require("node:assert/strict");
const { spawn, spawnSync } = require("node:child_process");
const { EventEmitter } = require("node:events");

const DEFAULT_EVENT_TIMEOUT_MS = process.platform === "win32" ? 15000 : 5000;
const DEFAULT_POLL_ATTEMPTS = process.platform === "win32" ? 200 : 50;
const DEFAULT_ZELLIJ_DISCOVERY_ATTEMPTS = process.platform === "win32" ? 600 : 300;

function readyEchoLaunch() {
  if (process.platform === "win32") {
    return {
      program: process.env.COMSPEC || "cmd.exe",
      args: ["/Q", "/K", "echo ready & more"],
    };
  }

  return {
    program: "/bin/sh",
    args: ["-lc", "printf 'ready\\n'; exec cat"],
  };
}

async function runSmoke(createClient) {
  const client = createClient();

  const version = client.bindingVersion();
  const handshake = await client.handshakeInfo();
  const nativeCapabilities = await client.backendCapabilities("native");
  const tmuxCapabilities = await client.backendCapabilities("tmux");
  const zellijCapabilities = await client.backendCapabilities("zellij");
  const created = await client.createNativeSession({
    title: "node-smoke",
    launch: readyEchoLaunch(),
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
  const initialTopologyEvent = await withTimeout(
    topologySubscription.nextEvent(),
    5000,
    "Timed out waiting for initial topology subscription event",
  );
  const paneSubscription = await client.openSubscription(created.session_id, {
    kind: "pane_surface",
    pane_id: attached.focused_screen.pane_id,
  });
  const initialPaneEvent = await withTimeout(
    paneSubscription.nextEvent(),
    5000,
    "Timed out waiting for initial pane subscription event",
  );
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
  if (zellijCapabilities.capabilities.rendered_viewport_snapshot) {
    assert.equal(zellijCapabilities.capabilities.tab_create, true);
    assert.equal(zellijCapabilities.capabilities.tab_close, true);
    assert.equal(zellijCapabilities.capabilities.tab_focus, true);
    assert.equal(zellijCapabilities.capabilities.tab_rename, true);
    assert.equal(zellijCapabilities.capabilities.rendered_viewport_stream, true);
    assert.equal(zellijCapabilities.capabilities.session_scoped_tab_refs, true);
    assert.equal(zellijCapabilities.capabilities.session_scoped_pane_refs, true);
    assert.equal(zellijCapabilities.capabilities.pane_close, true);
    assert.equal(zellijCapabilities.capabilities.pane_focus, true);
    assert.equal(zellijCapabilities.capabilities.pane_input_write, true);
    assert.equal(zellijCapabilities.capabilities.pane_paste_write, true);
    assert.equal(zellijCapabilities.capabilities.plugin_panes, true);
    assert.equal(zellijCapabilities.capabilities.advisory_metadata_subscriptions, true);
    assert.equal(zellijCapabilities.capabilities.read_only_client_mode, true);
  } else {
    assert.equal(zellijCapabilities.capabilities.tab_create, false);
    assert.equal(zellijCapabilities.capabilities.tab_close, false);
    assert.equal(zellijCapabilities.capabilities.tab_focus, false);
    assert.equal(zellijCapabilities.capabilities.tab_rename, false);
    assert.equal(zellijCapabilities.capabilities.pane_close, false);
    assert.equal(zellijCapabilities.capabilities.pane_focus, false);
    assert.equal(zellijCapabilities.capabilities.pane_input_write, false);
    assert.equal(zellijCapabilities.capabilities.pane_paste_write, false);
    assert.equal(zellijCapabilities.capabilities.rendered_viewport_stream, false);
  }
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

  await runSubscriptionBackpressureSmoke(createClient);
  await runZellijImportSmoke(createClient, zellijCapabilities);

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

async function runZellijImportSmoke(createClient, zellijCapabilities) {
  const client = createClient();
  const sessionName = uniqueZellijSessionName("pkg");
  const zellijWrapper = spawnZellijSession(sessionName);

  try {
    const candidate = await waitForDiscoveredZellijSession(client, sessionName);
    scheduleZellijWrapperCleanup(zellijWrapper);

    if (!zellijCapabilities.capabilities.rendered_viewport_snapshot) {
      await assert.rejects(
        client.importSession(candidate.route, candidate.title),
        /backend_unsupported:.*zellij/i,
      );
      return;
    }

    const imported = await client.importSession(candidate.route, candidate.title);
    const topology = await client.topologySnapshot(imported.session_id);
    const initialTabCount = topology.tabs.length;
    const initialFocusedTab = topology.focused_tab ?? topology.tabs[0]?.tab_id ?? null;
    const focusedPaneId = focusedPaneIdFromTopology(topology);

    assert.equal(imported.route.backend, "zellij");
    assert.equal(topology.backend_kind, "zellij");
    assert.equal(typeof initialFocusedTab, "string");
    assert.equal(typeof focusedPaneId, "string");

    const sendInput = await client.dispatchMuxCommand(imported.session_id, {
      kind: "send_input",
      pane_id: focusedPaneId,
      data: "echo zellij package smoke\r",
    });
    const screen = await waitForLine(
      client,
      imported.session_id,
      focusedPaneId,
      "zellij package smoke",
    );
    const newTab = await client.dispatchMuxCommand(imported.session_id, {
      kind: "new_tab",
      title: "package-rich",
    });
    const topologyAfterCreate = await waitForTopologyState(
      client,
      imported.session_id,
      (snapshot) =>
        snapshot.tabs.length === initialTabCount + 1 &&
        snapshot.tabs.some((tab) => tab.title === "package-rich"),
      "zellij package new tab",
    );
    const richTabId =
      topologyAfterCreate.tabs.find((tab) => tab.title === "package-rich")?.tab_id ?? null;

    assert.equal(sendInput.changed, true);
    assert.equal(
      screen.surface.lines.some((line) => line.text.includes("zellij package smoke")),
      true,
    );
    assert.equal(newTab.changed, true);
    assert.equal(typeof richTabId, "string");

    const renameTab = await client.dispatchMuxCommand(imported.session_id, {
      kind: "rename_tab",
      tab_id: richTabId,
      title: "package-rich-renamed",
    });
    const topologyAfterRename = await waitForTopologyState(
      client,
      imported.session_id,
      (snapshot) =>
        snapshot.tabs.some(
          (tab) =>
            tab.tab_id === richTabId && tab.title === "package-rich-renamed",
        ),
      "zellij package rename tab",
    );
    const focusTab = await client.dispatchMuxCommand(imported.session_id, {
      kind: "focus_tab",
      tab_id: initialFocusedTab,
    });
    const topologyAfterFocus = await waitForTopologyState(
      client,
      imported.session_id,
      (snapshot) => snapshot.focused_tab === initialFocusedTab,
      "zellij package focus tab",
    );
    const closeTab = await client.dispatchMuxCommand(imported.session_id, {
      kind: "close_tab",
      tab_id: richTabId,
    });
    const topologyAfterClose = await waitForTopologyState(
      client,
      imported.session_id,
      (snapshot) =>
        snapshot.tabs.length === initialTabCount &&
        snapshot.tabs.every((tab) => tab.tab_id !== richTabId),
      "zellij package close tab",
    );

    assert.equal(renameTab.changed, true);
    assert.equal(
      topologyAfterRename.tabs.some(
        (tab) => tab.tab_id === richTabId && tab.title === "package-rich-renamed",
      ),
      true,
    );
    assert.equal(focusTab.changed, true);
    assert.equal(topologyAfterFocus.focused_tab, initialFocusedTab);
    assert.equal(closeTab.changed, true);
    assert.equal(topologyAfterClose.tabs.length, initialTabCount);
  } finally {
    if (zellijWrapper.exitCode === null && !zellijWrapper.killed) {
      try {
        zellijWrapper.kill();
      } catch (_error) {
        // The wrapper may have already exited once the background session was created.
      }
    }
    stopZellijSession(sessionName);
  }
}

async function runSubscriptionBackpressureSmoke(createClient) {
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-backpressure-close",
    launch: readyEchoLaunch(),
  });
  const attached = await client.attachSession(created.session_id);
  const tabId =
    attached.topology.focused_tab ?? attached.topology.tabs[0]?.tab_id ?? null;
  const subscription = await client.openSubscription(created.session_id, {
    kind: "session_topology",
  });
  const initialEvent = await withTimeout(
    subscription.nextEvent(),
    5000,
    "Timed out waiting for initial topology event before backpressure test",
  );

  assert.equal(initialEvent?.kind, "topology_snapshot");
  assert.equal(typeof tabId, "string");

  for (let revision = 0; revision < 96; revision += 1) {
    await client.dispatchMuxCommand(created.session_id, {
      kind: "rename_tab",
      tab_id: tabId,
      title: `backpressure-${revision}`,
    });
  }

  await withTimeout(
    subscription.close(),
    5000,
    "Timed out closing topology subscription under backpressure",
  );
}

async function runSubscriptionCycleSmoke(createClient) {
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-addon-repeat-subscriptions",
    launch: readyEchoLaunch(),
  });
  const attached = await client.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;
  let observedMarkers = 0;

  for (let cycle = 0; cycle < 24; cycle += 1) {
    const topologySubscription = await client.openSubscription(created.session_id, {
      kind: "session_topology",
    });
    const initialTopology = await withTimeout(
      topologySubscription.nextEvent(),
      5000,
      `Timed out waiting for topology subscription cycle ${cycle}`,
    );
    assert.equal(initialTopology?.kind, "topology_snapshot");
    assert.equal(initialTopology?.session_id, created.session_id);
    await withTimeout(
      topologySubscription.close(),
      5000,
      `Timed out closing topology subscription cycle ${cycle}`,
    );

    const paneSubscription = await client.openSubscription(created.session_id, {
      kind: "pane_surface",
      pane_id: paneId,
    });
    const initialPane = await withTimeout(
      paneSubscription.nextEvent(),
      5000,
      `Timed out waiting for pane subscription cycle ${cycle}`,
    );
    assert.equal(initialPane?.kind, "screen_delta");
    assert.equal(initialPane?.pane_id, paneId);

    if (cycle % 6 === 5) {
      const marker = `node addon repeat ${cycle}`;
      await client.dispatchMuxCommand(created.session_id, {
        kind: "send_input",
        pane_id: paneId,
        data: `${marker}\r`,
      });
      const paneUpdate = await waitForSubscriptionText(
        paneSubscription,
        marker,
        cycle,
      );
      assert.equal(deltaContainsText(paneUpdate, marker), true);
      observedMarkers += 1;
    }

    await withTimeout(
      paneSubscription.close(),
      5000,
      `Timed out closing pane subscription cycle ${cycle}`,
    );
  }

  return {
    session_id: created.session_id,
    pane_id: paneId,
    cycles: 24,
    observed_markers: observedMarkers,
  };
}

async function runAddonShutdownSmoke(createClient, options = {}) {
  const { onReady } = options;
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-addon-shutdown",
    launch: readyEchoLaunch(),
  });
  const attached = await client.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;
  const paneSubscription = await client.openSubscription(created.session_id, {
    kind: "pane_surface",
    pane_id: paneId,
  });
  const initialEvent = await withTimeout(
    paneSubscription.nextEvent(),
    5000,
    "Timed out waiting for initial addon pane subscription event",
  );

  assert.equal(initialEvent?.kind, "screen_delta");
  assert.equal(initialEvent?.pane_id, paneId);

  if (typeof onReady === "function") {
    await onReady({ sessionId: created.session_id, paneId });
  }

  let subscriptionClosed = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const event = await withTimeout(
      paneSubscription.nextEvent(),
      5000,
      "Timed out waiting for addon pane subscription closure after daemon shutdown",
    );
    if (event == null) {
      subscriptionClosed = true;
      break;
    }
  }

  assert.equal(subscriptionClosed, true);

  return {
    session_id: created.session_id,
    pane_id: paneId,
    subscription_closed: subscriptionClosed,
  };
}

async function runPackageWatchSmoke(createClient, sdk) {
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-package-watch",
    launch: readyEchoLaunch(),
  });
  const attached = await client.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;

  let dispatchedTopology = false;
  const topologyEvents = [];
  const topologyAbort = new AbortController();
  await withTimeout(
    client.watchTopology(created.session_id, {
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
    }),
    5000,
    "Timed out waiting for watchTopology helper to finish",
  );

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
  await withTimeout(
    panePump,
    5000,
    "Timed out waiting for pane subscription pump to finish",
  );

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
    launch: readyEchoLaunch(),
  });
  const sessionAttached = await client.attachSession(sessionCreated.session_id);
  const sessionPaneId = sessionAttached.focused_screen.pane_id;
  const sessionStates = [];
  let sessionDispatched = false;
  let sessionOpenedTab = false;
  const sessionAbort = new AbortController();
  await withTimeout(
    client.watchSessionState(sessionCreated.session_id, {
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
    }),
    5000,
    "Timed out waiting for watchSessionState helper to finish",
  );

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
  await runSessionStateFocusChurnSmoke(createClient);

  await runElectronBridgeSmoke(createClient, sdk);
}

async function runSessionStateFocusChurnSmoke(createClient) {
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-package-focus-churn",
    launch: readyEchoLaunch(),
  });
  const initialAttached = await client.attachSession(created.session_id);
  const initialPaneId = initialAttached.focused_screen.pane_id;

  assert.equal(typeof initialPaneId, "string");

  for (const title of ["focus-a", "focus-b"]) {
    const newTab = await client.dispatchMuxCommand(created.session_id, {
      kind: "new_tab",
      title,
    });
    assert.equal(newTab.changed, true);
  }

  const initialTopology = await waitForTopologyState(
    client,
    created.session_id,
    (snapshot) => snapshot.tabs.length === 3,
    "session state focus churn setup",
  );
  const tabIds = initialTopology.tabs.map((tab) => tab.tab_id);
  const focusSequence = [
    tabIds[1],
    tabIds[2],
    tabIds[0],
    tabIds[2],
    tabIds[1],
    tabIds[0],
    tabIds[2],
    tabIds[1],
    tabIds[0],
    tabIds[2],
  ];
  const expectedFinalTab = focusSequence[focusSequence.length - 1];
  const observedStates = [];
  const focusAbort = new AbortController();
  const watchPromise = client.watchSessionState(created.session_id, {
    signal: focusAbort.signal,
    onState: async (state) => {
      observedStates.push(state);
      if (
        state.topology.focused_tab === expectedFinalTab &&
        state.focusedScreen?.pane_id === focusedPaneIdFromTopology(state.topology)
      ) {
        focusAbort.abort();
      }
    },
  });

  for (const tabId of focusSequence) {
    const focused = await client.dispatchMuxCommand(created.session_id, {
      kind: "focus_tab",
      tab_id: tabId,
    });
    assert.equal(focused.changed, true);
  }

  await withTimeout(
    watchPromise,
    5000,
    "Timed out waiting for watchSessionState focus churn to finish",
  );

  const finalTopology = await waitForTopologyState(
    client,
    created.session_id,
    (snapshot) => snapshot.focused_tab === expectedFinalTab,
    "session state focus churn final focus",
  );

  assert.equal(finalTopology.focused_tab, expectedFinalTab);
  assert.equal(observedStates.length > 0, true);
  assert.equal(
    observedStates.every((state) => {
      const expectedPaneId = focusedPaneIdFromTopology(state.topology);
      return expectedPaneId ? state.focusedScreen?.pane_id === expectedPaneId : true;
    }),
    true,
  );
  assert.equal(
    observedStates.some((state) => state.topology.focused_tab === expectedFinalTab),
    true,
  );
}

async function runShutdownSmoke(createClient, options = {}) {
  const { onReady } = options;
  const client = createClient();
  const created = await client.createNativeSession({
    title: "node-package-shutdown",
    launch: readyEchoLaunch(),
  });
  const attached = await client.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;
  const paneSubscription = await client.subscribePane(created.session_id, paneId);
  const initialEvent = await withTimeout(
    paneSubscription.nextEvent(),
    5000,
    "Timed out waiting for initial pane subscription event",
  );

  assert.equal(initialEvent?.kind, "screen_delta");

  let watchStates = 0;
  let readyResolve;
  const ready = new Promise((resolve) => {
    readyResolve = resolve;
  });
  const watchPromise = client.watchSessionState(created.session_id, {
    onState: async (state) => {
      watchStates += 1;
      if (watchStates === 1) {
        readyResolve(state);
      }
    },
  });
  const initialState = await withTimeout(
    ready,
    5000,
    "Timed out waiting for initial watchSessionState callback",
  );

  assert.equal(initialState.session.session_id, created.session_id);
  assert.equal(initialState.focusedScreen?.pane_id, paneId);

  if (typeof onReady === "function") {
    await onReady({ sessionId: created.session_id, paneId });
  }

  let subscriptionClosed = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const event = await withTimeout(
      paneSubscription.nextEvent(),
      5000,
      "Timed out waiting for pane subscription closure after daemon shutdown",
    );
    if (event == null) {
      subscriptionClosed = true;
      break;
    }
  }

  assert.equal(subscriptionClosed, true);

  await withTimeout(
    watchPromise,
    5000,
    "Timed out waiting for watchSessionState to close after daemon shutdown",
  );

  return {
    session_id: created.session_id,
    subscription_closed: subscriptionClosed,
    watch_closed: true,
    observed_states: watchStates,
  };
}

async function runRestartRecoverySmoke(createClient, options = {}) {
  const { onInitialReady, waitForStop, onStaleObserved, waitForRestart } = options;
  const client = createClient();
  const initialHandshake = await client.handshakeInfo();
  const initial = await client.createNativeSession({
    title: "node-package-restart-before",
    launch: readyEchoLaunch(),
  });

  if (typeof onInitialReady === "function") {
    await onInitialReady({
      sessionId: initial.session_id,
      protocol: initialHandshake.handshake.protocol_version,
    });
  }

  if (typeof waitForStop === "function") {
    await waitForStop();
  }

  let staleErrorObserved = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      await client.handshakeInfo();
    } catch (_error) {
      staleErrorObserved = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  if (!staleErrorObserved) {
    throw new Error("Expected stale daemon request to fail after shutdown");
  }

  if (typeof onStaleObserved === "function") {
    await onStaleObserved();
  }

  if (typeof waitForRestart === "function") {
    await waitForRestart();
  }

  let recoveredSessionId = null;
  let recoveredSubscriptionOk = false;
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      const handshake = await client.handshakeInfo();
      const created = await client.createNativeSession({
        title: "node-package-restart-after",
        launch: readyEchoLaunch(),
      });
      if (handshake.assessment.can_use && created.session_id) {
        recoveredSessionId = created.session_id;
        const attached = await client.attachSession(created.session_id);
        const paneId = attached.focused_screen?.pane_id;
        if (typeof paneId === "string") {
          const subscription = await client.openSubscription(created.session_id, {
            kind: "pane_surface",
            pane_id: paneId,
          });
          const initialEvent = await withTimeout(
            subscription.nextEvent(),
            5000,
            "Timed out waiting for recovered subscription initial event",
          );
          recoveredSubscriptionOk =
            initialEvent?.kind === "screen_delta" && initialEvent?.pane_id === paneId;
          await withTimeout(
            subscription.close(),
            5000,
            "Timed out closing recovered subscription",
          );
        }
        break;
      }
    } catch (_error) {
      // Keep retrying while the replacement daemon is becoming ready.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  if (recoveredSessionId == null) {
    throw new Error("Failed to recover against restarted daemon using the same client");
  }

  return {
    initial_session_id: initial.session_id,
    stale_error_observed: staleErrorObserved,
    recovered: true,
    recovered_session_id: recoveredSessionId,
    recovered_subscription_ok: recoveredSubscriptionOk,
  };
}

async function waitForLine(client, sessionId, paneId, needle) {
  for (let attempt = 0; attempt < DEFAULT_POLL_ATTEMPTS; attempt += 1) {
    const snapshot = await client.screenSnapshot(sessionId, paneId);
    if (snapshot.surface.lines.some((line) => line.text.includes(needle))) {
      return snapshot;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error(`Timed out waiting for screen line: ${needle}`);
}

async function waitForTopologyTabs(subscription, tabCount) {
  for (let attempt = 0; attempt < DEFAULT_POLL_ATTEMPTS; attempt += 1) {
    const event = await withTimeout(
      subscription.nextEvent(),
      DEFAULT_EVENT_TIMEOUT_MS,
      `Timed out waiting for topology event while expecting ${tabCount} tabs`,
    );
    if (event == null) {
      break;
    }
    if (event.kind === "topology_snapshot" && event.tabs.length === tabCount) {
      return event;
    }
  }

  throw new Error(`Timed out waiting for topology with ${tabCount} tabs`);
}

async function waitForSubscriptionText(subscription, needle, cycle) {
  for (let attempt = 0; attempt < DEFAULT_POLL_ATTEMPTS; attempt += 1) {
    const event = await withTimeout(
      subscription.nextEvent(),
      DEFAULT_EVENT_TIMEOUT_MS,
      `Timed out waiting for pane delta cycle ${cycle}`,
    );
    if (event?.kind === "screen_delta" && deltaContainsText(event, needle)) {
      return event;
    }
  }

  throw new Error(`Timed out waiting for pane delta text: ${needle}`);
}

async function waitForTopologyState(client, sessionId, predicate, label) {
  for (let attempt = 0; attempt < DEFAULT_POLL_ATTEMPTS; attempt += 1) {
    const snapshot = await client.topologySnapshot(sessionId);
    if (predicate(snapshot)) {
      return snapshot;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error(`Timed out waiting for topology state: ${label}`);
}

async function withTimeout(promise, ms, message) {
  let timer = null;

  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => {
          reject(new Error(message));
        }, ms);
      }),
    ]);
  } finally {
    if (timer != null) {
      clearTimeout(timer);
    }
  }
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

function uniqueZellijSessionName(label) {
  return `tp-${label}-${process.pid}-${Date.now().toString(16)}`;
}

function spawnZellijSession(sessionName) {
  const child = spawn(
    "zellij",
    [
      "attach",
      "--create-background",
      sessionName,
      "options",
      "--default-layout",
      "default",
    ],
    {
      stdio: "ignore",
      windowsHide: true,
    },
  );
  child.unref();
  return child;
}

function scheduleZellijWrapperCleanup(child) {
  const cleanupTimer = setTimeout(() => {
    if (child.exitCode === null && !child.killed) {
      try {
        child.kill();
      } catch (_error) {
        // Best-effort cleanup for a wrapper process that may linger after the
        // background session becomes discoverable.
      }
    }
  }, 3000);
  cleanupTimer.unref?.();
}

function stopZellijSession(sessionName) {
  spawnSync("zellij", ["kill-session", sessionName], { encoding: "utf8" });
}

function isHeadlessZellijSpawnError(stderr) {
  return (
    stderr.includes("could not get terminal attribute") ||
    stderr.includes("could not enable raw mode") ||
    stderr.includes("No such device or address") ||
    stderr.includes("The handle is invalid")
  );
}

async function waitForDiscoveredZellijSession(client, sessionName) {
  for (let attempt = 0; attempt < DEFAULT_ZELLIJ_DISCOVERY_ATTEMPTS; attempt += 1) {
    const sessions = await client.discoverSessions("zellij");
    const candidate =
      sessions.find((session) => session.title === sessionName) ?? null;
    if (candidate) {
      return candidate;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }

  throw new Error(`Timed out waiting for discovered zellij session: ${sessionName}`);
}

async function runElectronBridgeSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-smoke",
    client,
    ipcMain,
  });
  const rendererClient = new sdk.ElectronTerminalNodeClient({
    channelPrefix: "terminal-platform-smoke",
    ipcRenderer,
  });

  const version = await rendererClient.bindingVersion();
  const handshake = await rendererClient.handshakeInfo();
  const created = await rendererClient.createNativeSession({
    title: "electron-bridge-smoke",
    launch: readyEchoLaunch(),
  });
  const attached = await rendererClient.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;
  const stateEvents = [];
  let sentInput = false;
  let openedTab = false;
  const watchAbort = new AbortController();

  await withTimeout(
    rendererClient.watchSessionState(created.session_id, {
      signal: watchAbort.signal,
      onState: async (state) => {
        stateEvents.push(state);

        if (!sentInput && state.focusedScreen) {
          sentInput = true;
          await rendererClient.dispatchMuxCommand(created.session_id, {
            kind: "send_input",
            pane_id: paneId,
            data: "electron bridge input\r",
          });
          return;
        }

        if (
          !openedTab &&
          state.focusedScreen?.surface.lines.some((line) =>
            line.text.includes("electron bridge input"),
          )
        ) {
          openedTab = true;
          await rendererClient.dispatchMuxCommand(created.session_id, {
            kind: "new_tab",
            title: "electron",
          });
          return;
        }

        if (openedTab && state.topology.tabs.length === 2) {
          watchAbort.abort();
        }
      },
    }),
    5000,
    "Timed out waiting for ElectronTerminalNodeClient.watchSessionState happy path",
  );

  const topology = await rendererClient.topologySnapshot(created.session_id);
  const screen = await rendererClient.screenSnapshot(created.session_id, paneId);

  assert.equal(version.protocol.major, 0);
  assert.equal(handshake.assessment.can_use, true);
  assert.equal(topology.session_id, created.session_id);
  assert.equal(screen.pane_id, paneId);
  assert.equal(
    stateEvents.some((state) =>
      state.focusedScreen?.surface.lines.some((line) =>
        line.text.includes("electron bridge input"),
      ),
    ),
    true,
  );
  assert.equal(
    stateEvents.every((state) => {
      const expectedPaneId = focusedPaneIdFromTopology(state.topology);
      return expectedPaneId ? state.focusedScreen?.pane_id === expectedPaneId : true;
    }),
    true,
  );

  bridge.dispose();
  await runElectronBridgeDisposeSmoke(createClient, sdk);
  await runElectronBridgeRepeatedWatchCyclesSmoke(createClient, sdk);

  await runElectronPreloadSmoke(createClient, sdk);
}

function createFakeElectronIpc() {
  const handlers = new Map();
  const events = new EventEmitter();
  const sender = {
    send(channel, payload) {
      setImmediate(() => {
        events.emit(channel, { sender }, payload);
      });
    },
    isDestroyed() {
      return false;
    },
  };

  return {
    ipcMain: {
      handle(channel, listener) {
        handlers.set(channel, listener);
      },
      removeHandler(channel) {
        handlers.delete(channel);
      },
    },
    ipcRenderer: {
      invoke(channel, payload) {
        const handler = handlers.get(channel);
        if (!handler) {
          return Promise.reject(new Error(`Missing fake Electron handler for ${channel}`));
        }
        return Promise.resolve(handler({ sender }, payload));
      },
      on(channel, listener) {
        events.on(channel, listener);
      },
      off(channel, listener) {
        events.off(channel, listener);
      },
    },
  };
}

async function runElectronPreloadSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-preload-smoke",
    client,
    ipcMain,
  });
  const exposed = {};
  const preloadApi = sdk.installElectronPreloadBridge({
    channelPrefix: "terminal-platform-preload-smoke",
    contextBridge: {
      exposeInMainWorld(key, value) {
        exposed[key] = value;
      },
    },
    exposeKey: "terminalPlatform",
    ipcRenderer,
  });

  assert.equal(exposed.terminalPlatform, preloadApi);
  assert.equal(typeof exposed.terminalPlatform.subscribeSessionState, "function");

  const handshake = await exposed.terminalPlatform.handshakeInfo();
  assert.equal(handshake.assessment.can_use, true);

  const created = await exposed.terminalPlatform.createNativeSession({
    title: "electron-preload-smoke",
    launch: readyEchoLaunch(),
  });
  const attached = await exposed.terminalPlatform.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;
  let resolveState;
  let rejectState;
  const seenState = new Promise((resolve, reject) => {
    resolveState = resolve;
    rejectState = reject;
  });

  const subscriptionId = await exposed.terminalPlatform.subscribeSessionState(
    created.session_id,
    async (state) => {
      if (
        state.focusedScreen?.surface.lines.some((line) =>
          line.text.includes("electron preload input"),
        )
      ) {
        resolveState(state);
      }
    },
    async (error) => {
      rejectState(error);
    },
  );

  await exposed.terminalPlatform.dispatchMuxCommand(created.session_id, {
    kind: "send_input",
    pane_id: paneId,
    data: "electron preload input\r",
  });

  const observedState = await withTimeout(
    seenState,
    5000,
    "Timed out waiting for Electron preload observed state",
  );
  assert.equal(observedState.session.session_id, created.session_id);
  assert.equal(observedState.focusedScreen.pane_id, paneId);

  const stopped = await exposed.terminalPlatform.unsubscribeSessionState(subscriptionId);
  const stoppedAgain = await exposed.terminalPlatform.unsubscribeSessionState(subscriptionId);

  assert.equal(stopped, true);
  assert.equal(stoppedAgain, false);

  await withTimeout(
    exposed.terminalPlatform.dispose(),
    5000,
    "Timed out waiting for Electron preload dispose()",
  );
  bridge.dispose();
  await runElectronBridgeStopDrainSmoke(sdk);
  await runElectronPreloadRepeatedSubscribeSmoke(createClient, sdk);
  await runElectronPreloadDisposeSmoke(createClient, sdk);
}

async function runElectronBridgeDisposeSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-dispose-smoke",
    client,
    ipcMain,
  });
  const rendererClient = new sdk.ElectronTerminalNodeClient({
    channelPrefix: "terminal-platform-dispose-smoke",
    ipcRenderer,
  });
  const created = await rendererClient.createNativeSession({
    title: "electron-bridge-dispose-smoke",
    launch: readyEchoLaunch(),
  });

  let observedStates = 0;
  let resolveReady;
  const ready = new Promise((resolve) => {
    resolveReady = resolve;
  });
  const watchPromise = rendererClient.watchSessionState(created.session_id, {
    onState: async (state) => {
      observedStates += 1;
      if (observedStates === 1) {
        resolveReady(state);
      }
    },
  });

  const initialState = await withTimeout(
    ready,
    5000,
    "Timed out waiting for first Electron bridge state before dispose()",
  );
  assert.equal(initialState.session.session_id, created.session_id);

  bridge.dispose();

  await withTimeout(
    watchPromise,
    5000,
    "Timed out waiting for Electron bridge watch promise to resolve after dispose()",
  );
  await assert.rejects(
    rendererClient.bindingVersion(),
    /Missing fake Electron handler/,
  );
  assert.equal(observedStates >= 1, true);
}

async function runElectronBridgeRepeatedWatchCyclesSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-repeat-bridge-smoke",
    client,
    ipcMain,
  });
  const rendererClient = new sdk.ElectronTerminalNodeClient({
    channelPrefix: "terminal-platform-repeat-bridge-smoke",
    ipcRenderer,
  });
  const created = await rendererClient.createNativeSession({
    title: "electron-bridge-repeat-smoke",
    launch: readyEchoLaunch(),
  });
  const attached = await rendererClient.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;

  for (let cycle = 0; cycle < 4; cycle += 1) {
    const marker = `electron bridge repeat ${cycle}`;
    const abortController = new AbortController();
    let observedMatchingState = false;

    const watchPromise = rendererClient.watchSessionState(created.session_id, {
      signal: abortController.signal,
      onState: async (state) => {
        const expectedPaneId = focusedPaneIdFromTopology(state.topology);
        assert.equal(
          expectedPaneId ? state.focusedScreen?.pane_id === expectedPaneId : true,
          true,
        );
        if (
          state.focusedScreen?.surface.lines.some((line) =>
            line.text.includes(marker),
          )
        ) {
          observedMatchingState = true;
          abortController.abort();
        }
      },
    });

    await rendererClient.dispatchMuxCommand(created.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: `${marker}\r`,
    });

    await withTimeout(
      watchPromise,
      5000,
      `Timed out waiting for Electron bridge repeat watch cycle ${cycle}`,
    );
    assert.equal(observedMatchingState, true);
  }

  bridge.dispose();
}

async function runElectronBridgeStopDrainSmoke(sdk) {
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  let watchFinished = false;
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-stop-drain-smoke",
    client: {
      async watchSessionState(sessionId, options = {}) {
        await options.onState({
          session: {
            session_id: sessionId,
            route: {
              backend: "native",
              authority: "local_daemon",
              external: null,
            },
            title: "stop-drain",
          },
          topology: {
            session_id: sessionId,
            backend_kind: "native",
            focused_tab: null,
            tabs: [],
          },
          focusedScreen: null,
        });

        if (options.signal?.aborted) {
          await new Promise((resolve) => setTimeout(resolve, 50));
          watchFinished = true;
          return;
        }

        await new Promise((resolve) => {
          options.signal?.addEventListener("abort", resolve, { once: true });
        });
        await new Promise((resolve) => setTimeout(resolve, 50));
        watchFinished = true;
      },
    },
    ipcMain,
  });
  const rendererClient = new sdk.ElectronTerminalNodeClient({
    channelPrefix: "terminal-platform-stop-drain-smoke",
    ipcRenderer,
  });

  const abortController = new AbortController();
  const watchPromise = rendererClient.watchSessionState("stop-drain-session", {
    signal: abortController.signal,
    onState: async (state) => {
      assert.equal(state.session.session_id, "stop-drain-session");
      abortController.abort();
    },
  });

  await withTimeout(
    watchPromise,
    5000,
    "Timed out waiting for Electron bridge stop to drain active watcher",
  );
  assert.equal(watchFinished, true);

  bridge.dispose();
}

async function runElectronPreloadRepeatedSubscribeSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-repeat-preload-smoke",
    client,
    ipcMain,
  });
  const preloadApi = sdk.createElectronPreloadApi({
    channelPrefix: "terminal-platform-repeat-preload-smoke",
    ipcRenderer,
  });
  const created = await preloadApi.createNativeSession({
    title: "electron-preload-repeat-smoke",
    launch: readyEchoLaunch(),
  });
  const attached = await preloadApi.attachSession(created.session_id);
  const paneId = attached.focused_screen.pane_id;

  for (let cycle = 0; cycle < 4; cycle += 1) {
    const marker = `electron preload repeat ${cycle}`;
    let resolveState;
    let rejectState;
    const statePromise = new Promise((resolve, reject) => {
      resolveState = resolve;
      rejectState = reject;
    });
    const subscriptionId = await preloadApi.subscribeSessionState(
      created.session_id,
      async (state) => {
        const expectedPaneId = focusedPaneIdFromTopology(state.topology);
        assert.equal(
          expectedPaneId ? state.focusedScreen?.pane_id === expectedPaneId : true,
          true,
        );
        if (
          state.focusedScreen?.surface.lines.some((line) =>
            line.text.includes(marker),
          )
        ) {
          resolveState(state);
        }
      },
      async (error) => {
        rejectState(error);
      },
    );

    await preloadApi.dispatchMuxCommand(created.session_id, {
      kind: "send_input",
      pane_id: paneId,
      data: `${marker}\r`,
    });

    const observedState = await withTimeout(
      statePromise,
      5000,
      `Timed out waiting for Electron preload repeat cycle ${cycle}`,
    );
    assert.equal(observedState.session.session_id, created.session_id);
    assert.equal(observedState.focusedScreen?.pane_id, paneId);
    assert.equal(await preloadApi.unsubscribeSessionState(subscriptionId), true);
    assert.equal(await preloadApi.unsubscribeSessionState(subscriptionId), false);
  }

  await withTimeout(
    preloadApi.dispose(),
    5000,
    "Timed out waiting for Electron preload repeat dispose()",
  );
  bridge.dispose();
}

async function runElectronPreloadDisposeSmoke(createClient, sdk) {
  const client = createClient();
  const { ipcMain, ipcRenderer } = createFakeElectronIpc();
  const bridge = sdk.createElectronMainBridge({
    channelPrefix: "terminal-platform-preload-dispose-smoke",
    client,
    ipcMain,
  });
  const preloadApi = sdk.createElectronPreloadApi({
    channelPrefix: "terminal-platform-preload-dispose-smoke",
    ipcRenderer,
  });
  const created = await preloadApi.createNativeSession({
    title: "electron-preload-dispose-smoke",
    launch: readyEchoLaunch(),
  });

  const readyResolvers = new Map();
  const waitForReady = (key) =>
    new Promise((resolve) => {
      readyResolvers.set(key, resolve);
    });
  const firstState = waitForReady("first");
  const secondState = waitForReady("second");
  const firstSubscriptionId = await preloadApi.subscribeSessionState(
    created.session_id,
    async (state) => {
      const resolve = readyResolvers.get("first");
      if (resolve) {
        readyResolvers.delete("first");
        resolve(state);
      }
    },
  );
  const secondSubscriptionId = await preloadApi.subscribeSessionState(
    created.session_id,
    async (state) => {
      const resolve = readyResolvers.get("second");
      if (resolve) {
        readyResolvers.delete("second");
        resolve(state);
      }
    },
  );

  const [firstObservedState, secondObservedState] = await withTimeout(
    Promise.all([firstState, secondState]),
    5000,
    "Timed out waiting for preload subscriptions before dispose()",
  );
  assert.equal(firstObservedState.session.session_id, created.session_id);
  assert.equal(secondObservedState.session.session_id, created.session_id);

  await withTimeout(
    preloadApi.dispose(),
    5000,
    "Timed out waiting for createElectronPreloadApi().dispose() to drain subscriptions",
  );

  assert.equal(await preloadApi.unsubscribeSessionState(firstSubscriptionId), false);
  assert.equal(await preloadApi.unsubscribeSessionState(secondSubscriptionId), false);

  bridge.dispose();
}

module.exports = {
  runAddonShutdownSmoke,
  runPackageWatchSmoke,
  runRestartRecoverySmoke,
  runShutdownSmoke,
  runSmoke,
  runSubscriptionCycleSmoke,
};
