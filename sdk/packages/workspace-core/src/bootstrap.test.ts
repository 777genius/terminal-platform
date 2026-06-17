import { describe, expect, it } from "vitest";

import type {
  BackendCapabilitiesInfo,
  BackendKind,
  Handshake,
  MuxCommandResult,
} from "@terminal-platform/runtime-types";
import type { WorkspaceTransportClient, WorkspaceTransportFactory } from "@terminal-platform/workspace-contracts";

import { createWorkspaceHost } from "./bootstrap.js";

describe("createWorkspaceHost", () => {
  it("keeps transport bootstrap manual by default", async () => {
    const calls: string[] = [];
    const transport = createBootstrapTransport({
      onHandshake: () => calls.push("handshake"),
      onClose: () => calls.push("close"),
    });

    const host = createWorkspaceHost({ transport });

    expect(host.kernel.selectors.connection().state).toBe("idle");
    expect(calls).toEqual([]);

    await expect(host.bootstrap()).resolves.toBe(host.kernel);

    expect(calls).toContain("handshake");
    expect(host.kernel.selectors.connection().state).toBe("ready");

    await host.dispose();
    await host.dispose();

    expect(calls.filter((call) => call === "close")).toHaveLength(1);
  });

  it("single-flights eager bootstrap for production composition roots", async () => {
    let createCount = 0;
    let handshakeCount = 0;
    const factory: WorkspaceTransportFactory = {
      create() {
        createCount += 1;
        return createBootstrapTransport({
          async handshake() {
            handshakeCount += 1;
            await Promise.resolve();
            return createHandshake(["native"]);
          },
        });
      },
    };

    const host = createWorkspaceHost({
      autoBootstrap: true,
      transport: factory,
    });
    const [firstKernel, secondKernel] = await Promise.all([host.bootstrap(), host.bootstrap()]);

    expect(firstKernel).toBe(host.kernel);
    expect(secondKernel).toBe(host.kernel);
    expect(createCount).toBe(1);
    expect(handshakeCount).toBe(1);
    expect(host.kernel.selectors.connection().state).toBe("ready");

    await host.dispose();
  });

  it("allows bootstrap retry after a transient handshake failure", async () => {
    let shouldFail = true;
    let handshakeCount = 0;
    const transport = createBootstrapTransport({
      async handshake() {
        handshakeCount += 1;
        if (shouldFail) {
          throw new Error("daemon offline");
        }

        return createHandshake(["native"]);
      },
    });
    const host = createWorkspaceHost({ transport });

    await expect(host.bootstrap()).rejects.toMatchObject({
      code: "bootstrap_failed",
      message: "daemon offline",
    });

    shouldFail = false;

    await expect(host.bootstrap()).resolves.toBe(host.kernel);

    expect(handshakeCount).toBe(2);
    expect(host.kernel.selectors.connection().state).toBe("ready");

    await host.dispose();
  });

  it("rejects bootstrap after disposal with the public workspace error shape", async () => {
    const host = createWorkspaceHost({
      transport: createBootstrapTransport(),
    });

    await host.dispose();

    await expect(host.bootstrap()).rejects.toMatchObject({
      code: "disposed",
      message: "workspace host has been disposed",
      recoverable: false,
    });
  });
});

interface CreateBootstrapTransportOptions {
  handshake?: () => Promise<Handshake>;
  onHandshake?: () => void;
  onClose?: () => void;
}

function createBootstrapTransport(
  options: CreateBootstrapTransportOptions = {},
): WorkspaceTransportClient {
  let closed = false;

  const assertOpen = () => {
    if (closed) {
      throw new Error("transport closed");
    }
  };

  return {
    async handshake() {
      assertOpen();
      options.onHandshake?.();
      return options.handshake?.() ?? createHandshake(["native"]);
    },
    async listSessions() {
      assertOpen();
      return [];
    },
    async listSavedSessions() {
      assertOpen();
      return [];
    },
    async listCommandHistory() {
      assertOpen();
      return [];
    },
    async discoverSessions() {
      assertOpen();
      return [];
    },
    async getBackendCapabilities(backend) {
      assertOpen();
      return createCapabilities(backend);
    },
    async createSession() {
      return unusedTransportMethod();
    },
    async importSession() {
      return unusedTransportMethod();
    },
    async getSavedSession() {
      return unusedTransportMethod();
    },
    async deleteSavedSession() {
      return unusedTransportMethod();
    },
    async pruneSavedSessions() {
      return unusedTransportMethod();
    },
    async restoreSavedSession() {
      return unusedTransportMethod();
    },
    async attachSession() {
      return unusedTransportMethod();
    },
    async getTopologySnapshot() {
      return unusedTransportMethod();
    },
    async getScreenSnapshot() {
      return unusedTransportMethod();
    },
    async getScreenDelta() {
      return unusedTransportMethod();
    },
    async dispatchMuxCommand(): Promise<MuxCommandResult> {
      return unusedTransportMethod();
    },
    async openSubscription() {
      return unusedTransportMethod();
    },
    async close() {
      closed = true;
      options.onClose?.();
    },
  };
}

function createHandshake(availableBackends: BackendKind[]): Handshake {
  return {
    protocol_version: {
      major: 0,
      minor: 2,
    },
    binary_version: "0.1.0-test",
    daemon_phase: "ready",
    capabilities: {
      request_reply: true,
      topology_subscriptions: true,
      pane_subscriptions: true,
      backend_discovery: true,
      backend_capability_queries: true,
      saved_sessions: true,
      session_restore: true,
      degraded_error_reasons: true,
      session_health: true,
    },
    available_backends: availableBackends,
    session_scope: "bootstrap-test",
  };
}

function createCapabilities(backend: BackendKind): BackendCapabilitiesInfo {
  return {
    backend,
    capabilities: {
      tiled_panes: true,
      floating_panes: false,
      split_resize: true,
      tab_create: true,
      tab_close: true,
      tab_focus: true,
      tab_rename: true,
      session_scoped_tab_refs: true,
      session_scoped_pane_refs: true,
      pane_split: true,
      pane_close: true,
      pane_focus: true,
      pane_input_write: true,
      pane_paste_write: true,
      raw_output_stream: false,
      rendered_viewport_stream: true,
      rendered_viewport_snapshot: true,
      rendered_scrollback_snapshot: false,
      rich_screen_surface: false,
      layout_dump: true,
      layout_override: true,
      read_only_client_mode: false,
      explicit_session_save: true,
      explicit_session_restore: true,
      plugin_panes: false,
      advisory_metadata_subscriptions: true,
      independent_resize_authority: true,
    },
  };
}

function unusedTransportMethod(): never {
  throw new Error("unused transport method");
}
