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
}

module.exports = {
  loadNativeBinding,
  resolveNativeBindingPath,
  TerminalNodeClient,
  TerminalNodeSubscription,
};
