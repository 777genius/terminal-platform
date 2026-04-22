import { describe, expect, it } from "vitest";

import { createMemoryWorkspaceTransport } from "@terminal-platform/workspace-adapter-memory";
import type { SubscriptionEvent, SubscriptionMeta, SubscriptionSpec } from "@terminal-platform/runtime-types";

import {
  createWorkspacePreloadTransport,
  type WorkspacePreloadBridge,
  type WorkspacePreloadSubscriptionBridge,
} from "./index.js";

describe("workspace preload adapter", () => {
  it("delegates transport calls to the preload bridge", async () => {
    const transport = createMemoryWorkspaceTransport();
    const bridge = createBridgeFromTransport(transport);
    const preloadTransport = createWorkspacePreloadTransport({ bridge });

    const handshake = await preloadTransport.handshake();
    const sessions = await preloadTransport.listSessions();
    const attached = await preloadTransport.attachSession(sessions[0]!.session_id);

    expect(handshake.binary_version).toContain("0.1.0");
    expect(sessions).toHaveLength(1);
    expect(attached.session.session_id).toBe(sessions[0]!.session_id);

    await preloadTransport.close();
  });

  it("wraps preload subscriptions as workspace subscriptions", async () => {
    const transport = createMemoryWorkspaceTransport();
    const bridge = createBridgeFromTransport(transport);
    const preloadTransport = createWorkspacePreloadTransport({ bridge });
    const sessionId = (await preloadTransport.listSessions())[0]!.session_id;
    const attached = await preloadTransport.attachSession(sessionId);
    const spec: SubscriptionSpec = {
      kind: "pane_surface",
      pane_id: attached.focused_screen!.pane_id,
    };

    const subscription = await preloadTransport.openSubscription(sessionId, spec);
    const event = await subscription.nextEvent();

    expect(subscription.meta()).toEqual({
      subscription_id: "preload-memory-subscription",
    } satisfies SubscriptionMeta);
    expect(event?.kind).toBe("screen_delta");

    await subscription.close();
    await preloadTransport.close();
  });
});

function createBridgeFromTransport(
  transport: ReturnType<typeof createMemoryWorkspaceTransport>,
): WorkspacePreloadBridge {
  return {
    handshake: () => transport.handshake(),
    listSessions: () => transport.listSessions(),
    listSavedSessions: () => transport.listSavedSessions(),
    discoverSessions: (backend) => transport.discoverSessions(backend),
    getBackendCapabilities: (backend) => transport.getBackendCapabilities(backend),
    createSession: (backend, request) => transport.createSession(backend, request),
    importSession: (route, title) => transport.importSession(route, title),
    getSavedSession: (sessionId) => transport.getSavedSession(sessionId),
    deleteSavedSession: (sessionId) => transport.deleteSavedSession(sessionId),
    pruneSavedSessions: (keepLatest) => transport.pruneSavedSessions(keepLatest),
    restoreSavedSession: (sessionId) => transport.restoreSavedSession(sessionId),
    attachSession: (sessionId) => transport.attachSession(sessionId),
    getTopologySnapshot: (sessionId) => transport.getTopologySnapshot(sessionId),
    getScreenSnapshot: (sessionId, paneId) => transport.getScreenSnapshot(sessionId, paneId),
    getScreenDelta: (sessionId, paneId, fromSequence) =>
      transport.getScreenDelta(sessionId, paneId, fromSequence),
    dispatchMuxCommand: (sessionId, command) => transport.dispatchMuxCommand(sessionId, command),
    async openSubscription(sessionId, spec) {
      const subscription = await transport.openSubscription(sessionId, spec);
      return new PreloadBridgeSubscription(subscription.meta(), async () => subscription.nextEvent(), () =>
        subscription.close(),
      );
    },
    close: () => transport.close(),
  };
}

class PreloadBridgeSubscription implements WorkspacePreloadSubscriptionBridge {
  readonly #meta: SubscriptionMeta;
  readonly #nextEventFactory: () => Promise<SubscriptionEvent | null>;
  readonly #closeFactory: () => Promise<void>;

  constructor(
    meta: SubscriptionMeta,
    nextEventFactory: () => Promise<SubscriptionEvent | null>,
    closeFactory: () => Promise<void>,
  ) {
    this.#meta = meta;
    this.#nextEventFactory = nextEventFactory;
    this.#closeFactory = closeFactory;
  }

  meta(): SubscriptionMeta {
    return {
      ...this.#meta,
      subscription_id: "preload-memory-subscription",
    };
  }

  nextEvent(): Promise<SubscriptionEvent | null> {
    return this.#nextEventFactory();
  }

  close(): Promise<void> {
    return this.#closeFactory();
  }
}
