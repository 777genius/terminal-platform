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
