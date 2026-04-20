"use strict";

const fs = require("node:fs");
const path = require("node:path");

const defaultAddonPath = path.join(__dirname, "native", "terminal_node_napi.node");

function resolveNativeBindingPath(options = {}) {
  const candidates = [
    options.addonPath,
    process.env.TERMINAL_NODE_ADDON_PATH,
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

function loadNativeBinding(options = {}) {
  const addonPath = resolveNativeBindingPath(options);
  const binding = require(addonPath);

  if (!binding || !binding.TerminalNodeClient) {
    throw new Error(
      `terminal-node addon at ${addonPath} did not expose TerminalNodeClient`,
    );
  }

  return binding;
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

  createNativeSession(request = {}) {
    return this.#inner.createNativeSession(request);
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
}

module.exports = {
  loadNativeBinding,
  resolveNativeBindingPath,
  TerminalNodeClient,
};
