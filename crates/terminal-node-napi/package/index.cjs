"use strict";

const { randomUUID } = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

const defaultAddonPath = path.join(__dirname, "native", "terminal_node_napi.node");
const nativeManifestPath = path.join(__dirname, "native", "manifest.json");

function resolveNativeBindingPath(options = {}) {
  const manifestAddonPath = resolveManifestAddonPath();
  const candidates = [
    options.addonPath,
    process.env.TERMINAL_NODE_ADDON_PATH,
    manifestAddonPath,
    defaultAddonPath,
  ].filter(Boolean);

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  throw new Error(
    `terminal-node addon was not found. Tried: ${candidates.join(", ") || "<none>"}`,
  );
}

function resolveManifestAddonPath() {
  const manifest = readNativeManifest();
  if (!manifest) {
    return null;
  }

  const target = resolveCurrentTarget();
  const targets = Array.isArray(manifest.targets) ? manifest.targets : [];
  const exactMatch =
    targets.find((candidate) => {
      return (
        candidate.platform === target.platform &&
        candidate.arch === target.arch &&
        normalizeLibc(candidate.libc) === normalizeLibc(target.libc)
      );
    }) ??
    targets.find((candidate) => {
      return (
        candidate.platform === target.platform &&
        candidate.arch === target.arch &&
        candidate.libc == null
      );
    });

  if (!exactMatch?.file) {
    const availableTargets = targets
      .map((candidate) =>
        [candidate.platform, candidate.arch, candidate.libc].filter(Boolean).join("-"),
      )
      .join(", ");
    throw new Error(
      `terminal-node addon manifest does not contain a compatible target for ${formatTarget(target)}. Available targets: ${availableTargets || "<none>"}`,
    );
  }

  return path.join(__dirname, "native", exactMatch.file);
}

function readNativeManifest() {
  if (!fs.existsSync(nativeManifestPath)) {
    return null;
  }

  return JSON.parse(fs.readFileSync(nativeManifestPath, "utf8"));
}

function resolveCurrentTarget() {
  return {
    platform: process.platform,
    arch: process.arch,
    libc: detectLibc(),
  };
}

function detectLibc() {
  if (process.platform !== "linux") {
    return null;
  }

  const report = process.report?.getReport?.();
  return report?.header?.glibcVersionRuntime ? "gnu" : "musl";
}

function normalizeLibc(value) {
  return value == null ? null : value;
}

function formatTarget(target) {
  return [target.platform, target.arch, target.libc].filter(Boolean).join("-");
}

function loadNativeBinding(options = {}) {
  const addonPath = resolveNativeBindingPath(options);
  const binding = require(addonPath);

  if (!binding || !binding.TerminalNodeClient || !binding.TerminalNodeSubscription) {
    throw new Error(
      `terminal-node addon at ${addonPath} did not expose TerminalNodeClient/TerminalNodeSubscription`,
    );
  }

  return binding;
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

function focusedPaneId(topology) {
  const focusedTab =
    topology.tabs.find((tab) => tab.tab_id === topology.focused_tab) ?? topology.tabs[0];
  if (!focusedTab) {
    return null;
  }

  return focusedTab.focused_pane ?? firstPaneId(focusedTab.root);
}

function cloneScreenCursor(cursor) {
  return cursor ? { row: cursor.row, col: cursor.col } : null;
}

function cloneExternalSessionRef(external) {
  return external
    ? { namespace: external.namespace, value: external.value }
    : null;
}

function cloneSessionRoute(route) {
  return {
    backend: route.backend,
    authority: route.authority,
    external: cloneExternalSessionRef(route.external),
  };
}

function cloneSessionSummary(session) {
  return {
    session_id: session.session_id,
    route: cloneSessionRoute(session.route),
    title: session.title ?? null,
  };
}

function cloneScreenLine(line) {
  return { text: line.text };
}

function cloneScreenSurface(surface) {
  return {
    title: surface.title ?? null,
    cursor: cloneScreenCursor(surface.cursor),
    lines: Array.isArray(surface.lines) ? surface.lines.map(cloneScreenLine) : [],
  };
}

function cloneScreenSnapshot(snapshot) {
  return {
    pane_id: snapshot.pane_id,
    sequence: snapshot.sequence,
    rows: snapshot.rows,
    cols: snapshot.cols,
    source: snapshot.source,
    surface: cloneScreenSurface(snapshot.surface),
  };
}

function clonePaneTreeNode(node) {
  if (!node) {
    return null;
  }

  if (node.kind === "leaf") {
    return { kind: "leaf", pane_id: node.pane_id };
  }

  return {
    kind: "split",
    direction: node.direction,
    first: clonePaneTreeNode(node.first),
    second: clonePaneTreeNode(node.second),
  };
}

function cloneTopologySnapshot(topology) {
  return {
    session_id: topology.session_id,
    backend_kind: topology.backend_kind,
    focused_tab: topology.focused_tab ?? null,
    tabs: Array.isArray(topology.tabs)
      ? topology.tabs.map((tab) => ({
          tab_id: tab.tab_id,
          title: tab.title ?? null,
          root: clonePaneTreeNode(tab.root),
          focused_pane: tab.focused_pane ?? null,
        }))
      : [],
  };
}

function cloneSessionState(state) {
  return {
    session: cloneSessionSummary(state.session),
    topology: cloneTopologySnapshot(state.topology),
    focusedScreen: state.focusedScreen ? cloneScreenSnapshot(state.focusedScreen) : null,
  };
}

function createSessionState(attached) {
  return {
    session: cloneSessionSummary(attached.session),
    topology: cloneTopologySnapshot(attached.topology),
    focusedScreen: attached.focused_screen
      ? cloneScreenSnapshot(attached.focused_screen)
      : null,
  };
}

function applyScreenDelta(snapshot, delta) {
  if (delta.full_replace) {
    return {
      pane_id: delta.pane_id,
      sequence: delta.to_sequence,
      rows: delta.rows,
      cols: delta.cols,
      source: delta.source,
      surface: cloneScreenSurface(delta.full_replace),
    };
  }

  if (!snapshot) {
    throw new Error(
      `terminal-node cannot apply patch delta for pane ${delta.pane_id} without a base snapshot`,
    );
  }

  if (snapshot.pane_id !== delta.pane_id) {
    throw new Error(
      `terminal-node delta pane mismatch. Expected ${snapshot.pane_id}, got ${delta.pane_id}`,
    );
  }

  const next = cloneScreenSnapshot(snapshot);
  next.sequence = delta.to_sequence;
  next.rows = delta.rows;
  next.cols = delta.cols;
  next.source = delta.source;

  if (!delta.patch) {
    return next;
  }

  if (delta.patch.title_changed) {
    next.surface.title = delta.patch.title ?? null;
  }

  if (delta.patch.cursor_changed) {
    next.surface.cursor = cloneScreenCursor(delta.patch.cursor);
  }

  for (const update of delta.patch.line_updates ?? []) {
    while (next.surface.lines.length <= update.row) {
      next.surface.lines.push({ text: "" });
    }
    next.surface.lines[update.row] = cloneScreenLine(update.line);
  }

  return next;
}

function reduceSessionWatchEvent(state, event) {
  if (!state) {
    if (event.kind !== "attached") {
      throw new Error(
        `terminal-node session state requires an initial attached event, got ${event.kind}`,
      );
    }
    return createSessionState(event.attached);
  }

  switch (event.kind) {
    case "attached":
      return createSessionState(event.attached);
    case "topology_snapshot": {
      const nextFocusedPaneId = focusedPaneId(event.topology);
      const nextFocusedScreen =
        state.focusedScreen && state.focusedScreen.pane_id === nextFocusedPaneId
          ? cloneScreenSnapshot(state.focusedScreen)
          : null;

      return {
        session: cloneSessionSummary(state.session),
        topology: cloneTopologySnapshot(event.topology),
        focusedScreen: nextFocusedScreen,
      };
    }
    case "focused_screen":
      return {
        ...cloneSessionState(state),
        focusedScreen: cloneScreenSnapshot(event.screen),
      };
    case "screen_delta":
      if (
        !state.focusedScreen ||
        state.focusedScreen.pane_id !== event.delta.pane_id
      ) {
        return cloneSessionState(state);
      }
      return {
        ...cloneSessionState(state),
        focusedScreen: applyScreenDelta(state.focusedScreen, event.delta),
      };
    default:
      throw new Error(`terminal-node received unsupported session watch event: ${event.kind}`);
  }
}

const electronInvokeMethodNames = new Set([
  "attachSession",
  "backendCapabilities",
  "bindingVersion",
  "createNativeSession",
  "deleteSavedSession",
  "discoverSessions",
  "dispatchMuxCommand",
  "handshakeInfo",
  "importSession",
  "listSavedSessions",
  "listSessions",
  "pruneSavedSessions",
  "restoreSavedSession",
  "savedSession",
  "screenDelta",
  "screenSnapshot",
  "topologySnapshot",
]);

function buildElectronBridgeChannels(channelPrefix) {
  const prefix = channelPrefix || "terminal-platform";

  return {
    invoke: `${prefix}:invoke`,
    sessionStateEvent: `${prefix}:session-state:event`,
    sessionStateStart: `${prefix}:session-state:start`,
    sessionStateStop: `${prefix}:session-state:stop`,
  };
}

function assertElectronIpcMainLike(ipcMain) {
  if (!ipcMain || typeof ipcMain.handle !== "function" || typeof ipcMain.removeHandler !== "function") {
    throw new TypeError(
      "terminal-node electron main bridge requires ipcMain.handle/removeHandler",
    );
  }
}

function assertElectronIpcRendererLike(ipcRenderer) {
  if (
    !ipcRenderer ||
    typeof ipcRenderer.invoke !== "function" ||
    typeof ipcRenderer.on !== "function" ||
    typeof ipcRenderer.off !== "function"
  ) {
    throw new TypeError(
      "terminal-node electron renderer client requires ipcRenderer.invoke/on/off",
    );
  }
}

function assertElectronContextBridgeLike(contextBridge) {
  if (!contextBridge || typeof contextBridge.exposeInMainWorld !== "function") {
    throw new TypeError(
      "terminal-node electron preload bridge requires contextBridge.exposeInMainWorld",
    );
  }
}

function canSendElectronBridgeEnvelope(sender) {
  return (
    sender &&
    typeof sender.send === "function" &&
    !(typeof sender.isDestroyed === "function" && sender.isDestroyed())
  );
}

function serializeBridgeError(error) {
  if (error && typeof error === "object") {
    const payload = {
      message:
        typeof error.message === "string" ? error.message : String(error),
    };
    if (typeof error.code === "string") {
      payload.code = error.code;
    }
    return payload;
  }

  return { message: String(error) };
}

function deserializeBridgeError(payload) {
  const error = new Error(
    payload?.message ?? "terminal-node electron bridge request failed",
  );

  if (payload?.code) {
    error.code = payload.code;
  }

  return error;
}

async function invokeElectronClientMethod(client, method, args) {
  if (!electronInvokeMethodNames.has(method)) {
    throw new Error(`terminal-node electron bridge does not support method ${method}`);
  }

  const fn = client?.[method];
  if (typeof fn !== "function") {
    throw new Error(`terminal-node electron bridge client is missing method ${method}`);
  }

  return await fn.apply(client, args);
}

class TerminalNodeSubscription {
  #inner;
  #closed;

  constructor(inner) {
    this.#inner = inner;
    this.#closed = false;
  }

  get subscriptionId() {
    return this.#inner.subscriptionId;
  }

  async nextEvent() {
    const event = await this.#inner.nextEvent();
    if (event == null) {
      this.#closed = true;
    }
    return event;
  }

  async close() {
    if (this.#closed) {
      return;
    }

    this.#closed = true;
    await this.#inner.close();
  }

  async pump(options = {}) {
    const { signal, onEvent } = options;

    if (typeof onEvent !== "function") {
      throw new TypeError("TerminalNodeSubscription.pump requires an onEvent callback");
    }

    const abortListener = () => {
      void this.close();
    };

    if (signal) {
      if (signal.aborted) {
        await this.close();
        return;
      }

      signal.addEventListener("abort", abortListener, { once: true });
    }

    try {
      for await (const event of this) {
        await onEvent(event);
      }
    } finally {
      if (signal) {
        signal.removeEventListener("abort", abortListener);
      }
      await this.close().catch(() => {});
    }
  }

  async *[Symbol.asyncIterator]() {
    try {
      while (true) {
        const event = await this.nextEvent();
        if (event == null) {
          return;
        }
        yield event;
      }
    } finally {
      await this.close().catch(() => {});
    }
  }
}

class TerminalNodeClient {
  #inner;

  constructor(inner) {
    this.#inner = inner;
  }

  static fromRuntimeSlug(slug, options = {}) {
    const binding = loadNativeBinding(options);
    return new TerminalNodeClient(binding.TerminalNodeClient.fromRuntimeSlug(slug));
  }

  static fromNamespacedAddress(value, options = {}) {
    const binding = loadNativeBinding(options);
    return new TerminalNodeClient(
      binding.TerminalNodeClient.fromNamespacedAddress(value),
    );
  }

  static fromFilesystemPath(value, options = {}) {
    const binding = loadNativeBinding(options);
    return new TerminalNodeClient(
      binding.TerminalNodeClient.fromFilesystemPath(value),
    );
  }

  get address() {
    return this.#inner.address;
  }

  bindingVersion() {
    return this.#inner.bindingVersion();
  }

  handshakeInfo() {
    return this.#inner.handshakeInfo();
  }

  listSessions() {
    return this.#inner.listSessions();
  }

  listSavedSessions() {
    return this.#inner.listSavedSessions();
  }

  discoverSessions(backend) {
    return this.#inner.discoverSessions(backend);
  }

  backendCapabilities(backend) {
    return this.#inner.backendCapabilities(backend);
  }

  createNativeSession(request = {}) {
    return this.#inner.createNativeSession(request);
  }

  importSession(route, title = null) {
    return this.#inner.importSession(route, title);
  }

  savedSession(sessionId) {
    return this.#inner.savedSession(sessionId);
  }

  deleteSavedSession(sessionId) {
    return this.#inner.deleteSavedSession(sessionId);
  }

  pruneSavedSessions(keepLatest) {
    return this.#inner.pruneSavedSessions(keepLatest);
  }

  restoreSavedSession(sessionId) {
    return this.#inner.restoreSavedSession(sessionId);
  }

  attachSession(sessionId) {
    return this.#inner.attachSession(sessionId);
  }

  topologySnapshot(sessionId) {
    return this.#inner.topologySnapshot(sessionId);
  }

  screenSnapshot(sessionId, paneId) {
    return this.#inner.screenSnapshot(sessionId, paneId);
  }

  screenDelta(sessionId, paneId, fromSequence) {
    return this.#inner.screenDelta(sessionId, paneId, fromSequence);
  }

  dispatchMuxCommand(sessionId, command) {
    return this.#inner.dispatchMuxCommand(sessionId, command);
  }

  async openSubscription(sessionId, spec) {
    const subscription = await this.#inner.openSubscription(sessionId, spec);
    return new TerminalNodeSubscription(subscription);
  }

  subscribeTopology(sessionId) {
    return this.openSubscription(sessionId, { kind: "session_topology" });
  }

  subscribePane(sessionId, paneId) {
    return this.openSubscription(sessionId, {
      kind: "pane_surface",
      pane_id: paneId,
    });
  }

  async watchTopology(sessionId, options) {
    const subscription = await this.subscribeTopology(sessionId);
    return subscription.pump(options);
  }

  async watchPane(sessionId, paneId, options) {
    const subscription = await this.subscribePane(sessionId, paneId);
    return subscription.pump(options);
  }

  async watchSession(sessionId, options = {}) {
    const { signal, onEvent } = options;

    if (typeof onEvent !== "function") {
      throw new TypeError("TerminalNodeClient.watchSession requires an onEvent callback");
    }

    if (signal?.aborted) {
      return;
    }

    const bridgeAbort = new AbortController();
    const bridgeAbortListener = () => {
      bridgeAbort.abort();
    };
    if (signal) {
      signal.addEventListener("abort", bridgeAbortListener, { once: true });
    }

    let paneSubscription = null;
    let panePump = Promise.resolve();
    let paneError = null;
    let topologySubscription = null;
    let topologyPump = Promise.resolve();
    let topologyError = null;

    const stopPane = async () => {
      if (paneSubscription) {
        await paneSubscription.close().catch(() => {});
        paneSubscription = null;
      }
      await panePump.catch(() => {});
    };

    const stopTopology = async () => {
      if (topologySubscription) {
        await topologySubscription.close().catch(() => {});
        topologySubscription = null;
      }
      await topologyPump.catch(() => {});
    };

    const runPanePump = (paneId, emitFocusedScreen) => {
      panePump = paneSubscription
        .pump({
          signal: bridgeAbort.signal,
          onEvent: async (event) => {
            if (emitFocusedScreen) {
              const screen = await this.screenSnapshot(sessionId, paneId);
              emitFocusedScreen = false;
              await onEvent({ kind: "focused_screen", screen });
            }
            await onEvent({ kind: "screen_delta", delta: event });
          },
        })
        .catch((error) => {
          paneError = error;
          bridgeAbort.abort();
        });
    };

    const startPane = async (paneId, emitFocusedScreen, startPump = true) => {
      await stopPane();
      paneSubscription = await this.subscribePane(sessionId, paneId);
      if (startPump) {
        runPanePump(paneId, emitFocusedScreen);
      }
    };

    try {
      const attached = await this.attachSession(sessionId);

      let currentPaneId =
        attached.focused_screen?.pane_id ?? focusedPaneId(attached.topology);
      if (currentPaneId) {
        await startPane(currentPaneId, false, false);
      }

      topologySubscription = await this.subscribeTopology(sessionId);

      await onEvent({ kind: "attached", attached });
      if (currentPaneId) {
        runPanePump(currentPaneId, false);
      }
      topologyPump = topologySubscription
        .pump({
          signal: bridgeAbort.signal,
          onEvent: async (event) => {
            await onEvent({ kind: "topology_snapshot", topology: event });

            const nextPaneId = focusedPaneId(event);
            if (nextPaneId && nextPaneId !== currentPaneId) {
              currentPaneId = nextPaneId;
              await startPane(nextPaneId, true);
            }
          },
        })
        .catch((error) => {
          topologyError = error;
          bridgeAbort.abort();
        });

      await topologyPump;
      await panePump;
      if (topologyError) {
        throw topologyError;
      }
      if (paneError) {
        throw paneError;
      }
    } finally {
      if (signal) {
        signal.removeEventListener("abort", bridgeAbortListener);
      }
      bridgeAbort.abort();
      await stopTopology();
      await stopPane();
    }
  }

  async watchSessionState(sessionId, options = {}) {
    const { signal, onState } = options;

    if (typeof onState !== "function") {
      throw new TypeError("TerminalNodeClient.watchSessionState requires an onState callback");
    }

    let state = null;

    await this.watchSession(sessionId, {
      signal,
      onEvent: async (event) => {
        state = reduceSessionWatchEvent(state, event);
        const expectedFocusedPaneId = focusedPaneId(state.topology);
        // Do not surface a session state until the focused screen matches the
        // focused pane implied by topology. Topology can advance before the new
        // pane watcher is attached, and exposing that gap leaks stale/null
        // focusedScreen values to JS/Electron callers.
        if (
          expectedFocusedPaneId &&
          (!state.focusedScreen || state.focusedScreen.pane_id !== expectedFocusedPaneId)
        ) {
          return;
        }
        await onState(cloneSessionState(state));
      },
    });
  }
}

class ElectronTerminalNodeClient {
  #channels;
  #ipcRenderer;

  constructor(options = {}) {
    const { ipcRenderer, channelPrefix } = options;

    assertElectronIpcRendererLike(ipcRenderer);

    this.#ipcRenderer = ipcRenderer;
    this.#channels = buildElectronBridgeChannels(channelPrefix);
  }

  #invoke(method, ...args) {
    return this.#ipcRenderer.invoke(this.#channels.invoke, { method, args });
  }

  bindingVersion() {
    return this.#invoke("bindingVersion");
  }

  handshakeInfo() {
    return this.#invoke("handshakeInfo");
  }

  listSessions() {
    return this.#invoke("listSessions");
  }

  listSavedSessions() {
    return this.#invoke("listSavedSessions");
  }

  discoverSessions(backend) {
    return this.#invoke("discoverSessions", backend);
  }

  backendCapabilities(backend) {
    return this.#invoke("backendCapabilities", backend);
  }

  createNativeSession(request) {
    return this.#invoke("createNativeSession", request);
  }

  importSession(route, title = null) {
    return this.#invoke("importSession", route, title);
  }

  savedSession(sessionId) {
    return this.#invoke("savedSession", sessionId);
  }

  deleteSavedSession(sessionId) {
    return this.#invoke("deleteSavedSession", sessionId);
  }

  pruneSavedSessions(keepLatest) {
    return this.#invoke("pruneSavedSessions", keepLatest);
  }

  restoreSavedSession(sessionId) {
    return this.#invoke("restoreSavedSession", sessionId);
  }

  attachSession(sessionId) {
    return this.#invoke("attachSession", sessionId);
  }

  topologySnapshot(sessionId) {
    return this.#invoke("topologySnapshot", sessionId);
  }

  screenSnapshot(sessionId, paneId) {
    return this.#invoke("screenSnapshot", sessionId, paneId);
  }

  screenDelta(sessionId, paneId, fromSequence) {
    return this.#invoke("screenDelta", sessionId, paneId, fromSequence);
  }

  dispatchMuxCommand(sessionId, command) {
    return this.#invoke("dispatchMuxCommand", sessionId, command);
  }

  async watchSessionState(sessionId, options = {}) {
    const { signal, onState } = options;

    if (typeof onState !== "function") {
      throw new TypeError("ElectronTerminalNodeClient.watchSessionState requires an onState callback");
    }

    if (signal?.aborted) {
      return;
    }

    const subscriptionId = randomUUID();
    let finished = false;
    let resolveDone = () => {};
    let rejectDone = () => {};
    const done = new Promise((resolve, reject) => {
      resolveDone = resolve;
      rejectDone = reject;
    });

    const cleanup = () => {
      this.#ipcRenderer.off(this.#channels.sessionStateEvent, listener);
      if (signal) {
        signal.removeEventListener("abort", abortListener);
      }
    };

    const finishResolve = () => {
      if (finished) {
        return;
      }
      finished = true;
      cleanup();
      resolveDone();
    };

    const finishReject = (error) => {
      if (finished) {
        return;
      }
      finished = true;
      cleanup();
      rejectDone(error);
    };

    const stop = async () => {
      await this.#ipcRenderer
        .invoke(this.#channels.sessionStateStop, { subscriptionId })
        .catch(() => {});
    };

    const abortListener = () => {
      void stop().finally(() => {
        finishResolve();
      });
    };

    const listener = (_event, envelope) => {
      if (!envelope || envelope.subscriptionId !== subscriptionId) {
        return;
      }

      if (envelope.kind === "state") {
        void Promise.resolve(onState(envelope.state)).catch((error) => {
          void stop();
          finishReject(error);
        });
        return;
      }

      if (envelope.kind === "error") {
        finishReject(deserializeBridgeError(envelope.error));
        return;
      }

      if (envelope.kind === "closed") {
        finishResolve();
      }
    };

    this.#ipcRenderer.on(this.#channels.sessionStateEvent, listener);
    if (signal) {
      signal.addEventListener("abort", abortListener, { once: true });
    }

    try {
      await this.#ipcRenderer.invoke(this.#channels.sessionStateStart, {
        sessionId,
        subscriptionId,
      });
    } catch (error) {
      finishReject(error);
    }

    if (signal?.aborted) {
      void stop().finally(() => {
        finishResolve();
      });
    }

    return done;
  }
}

function createElectronPreloadApi(options = {}) {
  const { ipcRenderer, channelPrefix } = options;
  const client = new ElectronTerminalNodeClient({ ipcRenderer, channelPrefix });
  const sessionStateSubscriptions = new Map();

  const api = {
    bindingVersion() {
      return client.bindingVersion();
    },

    handshakeInfo() {
      return client.handshakeInfo();
    },

    listSessions() {
      return client.listSessions();
    },

    listSavedSessions() {
      return client.listSavedSessions();
    },

    discoverSessions(backend) {
      return client.discoverSessions(backend);
    },

    backendCapabilities(backend) {
      return client.backendCapabilities(backend);
    },

    createNativeSession(request) {
      return client.createNativeSession(request);
    },

    importSession(route, title = null) {
      return client.importSession(route, title);
    },

    savedSession(sessionId) {
      return client.savedSession(sessionId);
    },

    deleteSavedSession(sessionId) {
      return client.deleteSavedSession(sessionId);
    },

    pruneSavedSessions(keepLatest) {
      return client.pruneSavedSessions(keepLatest);
    },

    restoreSavedSession(sessionId) {
      return client.restoreSavedSession(sessionId);
    },

    attachSession(sessionId) {
      return client.attachSession(sessionId);
    },

    topologySnapshot(sessionId) {
      return client.topologySnapshot(sessionId);
    },

    screenSnapshot(sessionId, paneId) {
      return client.screenSnapshot(sessionId, paneId);
    },

    screenDelta(sessionId, paneId, fromSequence) {
      return client.screenDelta(sessionId, paneId, fromSequence);
    },

    dispatchMuxCommand(sessionId, command) {
      return client.dispatchMuxCommand(sessionId, command);
    },

    async subscribeSessionState(sessionId, onState, onError) {
      if (typeof onState !== "function") {
        throw new TypeError(
          "terminal-node electron preload api requires an onState callback for subscribeSessionState",
        );
      }
      if (onError != null && typeof onError !== "function") {
        throw new TypeError(
          "terminal-node electron preload api requires onError to be a function when provided",
        );
      }

      const subscriptionId = randomUUID();
      const abortController = new AbortController();
      const record = {
        abortController,
        watchPromise: Promise.resolve(),
      };
      sessionStateSubscriptions.set(subscriptionId, record);

      record.watchPromise = client
        .watchSessionState(sessionId, {
          signal: abortController.signal,
          onState,
        })
        .catch(async (error) => {
          if (!abortController.signal.aborted && typeof onError === "function") {
            await onError(error);
          }
        })
        .finally(() => {
          if (sessionStateSubscriptions.get(subscriptionId) === record) {
            sessionStateSubscriptions.delete(subscriptionId);
          }
        });

      return subscriptionId;
    },

    async unsubscribeSessionState(subscriptionId) {
      const record = sessionStateSubscriptions.get(subscriptionId);
      if (!record) {
        return false;
      }

      sessionStateSubscriptions.delete(subscriptionId);
      record.abortController.abort();
      await record.watchPromise;
      return true;
    },

    async dispose() {
      const subscriptionIds = [...sessionStateSubscriptions.keys()];
      for (const subscriptionId of subscriptionIds) {
        await api.unsubscribeSessionState(subscriptionId);
      }
    },
  };

  return Object.freeze(api);
}

function installElectronPreloadBridge(options = {}) {
  const { contextBridge, exposeKey = "terminalPlatform" } = options;

  assertElectronContextBridgeLike(contextBridge);

  if (typeof exposeKey !== "string" || exposeKey.length === 0) {
    throw new TypeError(
      "terminal-node electron preload bridge requires a non-empty exposeKey",
    );
  }

  const api = createElectronPreloadApi(options);
  contextBridge.exposeInMainWorld(exposeKey, api);
  return api;
}

function createElectronMainBridge(options = {}) {
  const { ipcMain, client, channelPrefix } = options;

  assertElectronIpcMainLike(ipcMain);

  if (!client || typeof client.watchSessionState !== "function") {
    throw new TypeError(
      "terminal-node electron main bridge requires a TerminalNodeClient-compatible instance",
    );
  }

  const channels = buildElectronBridgeChannels(channelPrefix);
  const subscriptionsBySender = new Map();

  const ensureSenderSubscriptions = (sender) => {
    let senderSubscriptions = subscriptionsBySender.get(sender);
    if (!senderSubscriptions) {
      senderSubscriptions = new Map();
      subscriptionsBySender.set(sender, senderSubscriptions);
    }
    return senderSubscriptions;
  };

  const releaseSenderSubscription = (sender, subscriptionId) => {
    const senderSubscriptions = subscriptionsBySender.get(sender);
    if (!senderSubscriptions) {
      return;
    }

    senderSubscriptions.delete(subscriptionId);
    if (senderSubscriptions.size === 0) {
      subscriptionsBySender.delete(sender);
    }
  };

  const handleInvoke = async (_event, payload = {}) => {
    const { method } = payload;
    const args = Array.isArray(payload.args) ? payload.args : [];
    return await invokeElectronClientMethod(client, method, args);
  };

  const handleSessionStateStart = async (event, payload = {}) => {
    const { sessionId, subscriptionId } = payload;

    if (typeof sessionId !== "string" || sessionId.length === 0) {
      throw new TypeError("terminal-node electron bridge requires a non-empty sessionId");
    }
    if (typeof subscriptionId !== "string" || subscriptionId.length === 0) {
      throw new TypeError("terminal-node electron bridge requires a non-empty subscriptionId");
    }

    const sender = event?.sender;
    if (!sender || typeof sender.send !== "function") {
      throw new TypeError("terminal-node electron bridge requires an event sender with send()");
    }

    const senderSubscriptions = ensureSenderSubscriptions(sender);
    if (senderSubscriptions.has(subscriptionId)) {
      throw new Error(
        `terminal-node electron bridge already tracks subscription ${subscriptionId}`,
      );
    }

    const abortController = new AbortController();
    senderSubscriptions.set(subscriptionId, abortController);

    const sendEnvelope = (envelope) => {
      if (!canSendElectronBridgeEnvelope(sender)) {
        abortController.abort();
        return false;
      }

      sender.send(channels.sessionStateEvent, envelope);
      return true;
    };

    void client
      .watchSessionState(sessionId, {
        signal: abortController.signal,
        onState: async (state) => {
          sendEnvelope({ subscriptionId, kind: "state", state });
        },
      })
      .catch((error) => {
        if (!abortController.signal.aborted) {
          sendEnvelope({
            subscriptionId,
            kind: "error",
            error: serializeBridgeError(error),
          });
        }
      })
      .finally(() => {
        sendEnvelope({ subscriptionId, kind: "closed" });
        releaseSenderSubscription(sender, subscriptionId);
      });

    return { subscriptionId };
  };

  const handleSessionStateStop = async (event, payload = {}) => {
    const { subscriptionId } = payload;

    if (typeof subscriptionId !== "string" || subscriptionId.length === 0) {
      throw new TypeError("terminal-node electron bridge requires a non-empty subscriptionId");
    }

    const senderSubscriptions = subscriptionsBySender.get(event?.sender);
    const abortController = senderSubscriptions?.get(subscriptionId);
    if (!abortController) {
      return { stopped: false, subscriptionId };
    }

    abortController.abort();
    return { stopped: true, subscriptionId };
  };

  ipcMain.handle(channels.invoke, handleInvoke);
  ipcMain.handle(channels.sessionStateStart, handleSessionStateStart);
  ipcMain.handle(channels.sessionStateStop, handleSessionStateStop);

  return {
    channels,
    dispose() {
      ipcMain.removeHandler(channels.invoke);
      ipcMain.removeHandler(channels.sessionStateStart);
      ipcMain.removeHandler(channels.sessionStateStop);

      for (const senderSubscriptions of subscriptionsBySender.values()) {
        for (const abortController of senderSubscriptions.values()) {
          abortController.abort();
        }
      }
      subscriptionsBySender.clear();
    },
  };
}

module.exports = {
  applyScreenDelta,
  createElectronMainBridge,
  createElectronPreloadApi,
  createSessionState,
  ElectronTerminalNodeClient,
  installElectronPreloadBridge,
  loadNativeBinding,
  reduceSessionWatchEvent,
  resolveNativeBindingPath,
  TerminalNodeClient,
  TerminalNodeSubscription,
};
