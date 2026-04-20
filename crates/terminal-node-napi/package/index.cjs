"use strict";

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
      return {
        ...cloneSessionState(state),
        focusedScreen: applyScreenDelta(state.focusedScreen, event.delta),
      };
    default:
      throw new Error(`terminal-node received unsupported session watch event: ${event.kind}`);
  }
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

    const stopPane = async () => {
      if (paneSubscription) {
        await paneSubscription.close().catch(() => {});
        paneSubscription = null;
      }
      await panePump.catch(() => {});
    };

    const startPane = async (paneId, emitFocusedScreen) => {
      await stopPane();
      paneSubscription = await this.subscribePane(sessionId, paneId);

      if (emitFocusedScreen) {
        const screen = await this.screenSnapshot(sessionId, paneId);
        await onEvent({ kind: "focused_screen", screen });
      }

      panePump = paneSubscription
        .pump({
          signal: bridgeAbort.signal,
          onEvent: async (event) => {
            await onEvent({ kind: "screen_delta", delta: event });
          },
        })
        .catch((error) => {
          paneError = error;
          bridgeAbort.abort();
        });
    };

    try {
      const attached = await this.attachSession(sessionId);
      await onEvent({ kind: "attached", attached });

      let currentPaneId =
        attached.focused_screen?.pane_id ?? focusedPaneId(attached.topology);
      if (currentPaneId) {
        await startPane(currentPaneId, false);
      }

      await this.watchTopology(sessionId, {
        signal: bridgeAbort.signal,
        onEvent: async (event) => {
          await onEvent({ kind: "topology_snapshot", topology: event });

          const nextPaneId = focusedPaneId(event);
          if (nextPaneId && nextPaneId !== currentPaneId) {
            currentPaneId = nextPaneId;
            await startPane(nextPaneId, true);
          }
        },
      });

      await panePump;
      if (paneError) {
        throw paneError;
      }
    } finally {
      if (signal) {
        signal.removeEventListener("abort", bridgeAbortListener);
      }
      bridgeAbort.abort();
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
        if (
          event.kind === "topology_snapshot" &&
          focusedPaneId(event.topology) &&
          !state.focusedScreen
        ) {
          return;
        }
        await onState(cloneSessionState(state));
      },
    });
  }
}

module.exports = {
  applyScreenDelta,
  createSessionState,
  loadNativeBinding,
  reduceSessionWatchEvent,
  resolveNativeBindingPath,
  TerminalNodeClient,
  TerminalNodeSubscription,
};
