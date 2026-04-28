import { describe, expect, it } from "vitest";

import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  PaneId,
  ScreenDelta,
  ScreenSnapshot,
  SavedSessionSummary,
  SessionId,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";
import type { WorkspaceSubscription, WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

import { createWorkspaceKernel } from "./create-workspace-kernel.js";
import {
  DEFAULT_COMMAND_HISTORY_LIMIT,
  DEFAULT_WORKSPACE_THEME_ID,
} from "../read-models/workspace-snapshot.js";

describe("createWorkspaceKernel theme commands", () => {
  it("applies registered terminal platform themes", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 1000,
    });

    kernel.commands.setTheme(" terminal-platform-light ");

    expect(kernel.selectors.themeId()).toBe("terminal-platform-light");
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("rejects unknown themes without corrupting the snapshot", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 2000,
    });

    kernel.commands.setTheme("missing-theme");

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "theme_unsupported",
        message: "Theme \"missing-theme\" is not registered for this workspace",
        recoverable: true,
        severity: "warn",
        timestampMs: 2000,
      },
    ]);

    await kernel.dispose();
  });

  it("allows hosts to register custom theme ids", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      availableThemeIds: ["acme-terminal"],
    });

    kernel.commands.setTheme("acme-terminal");

    expect(kernel.selectors.themeId()).toBe("acme-terminal");

    await kernel.dispose();
  });

  it("starts with a registered initial theme", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialThemeId: " terminal-platform-light ",
    });

    expect(kernel.selectors.themeId()).toBe("terminal-platform-light");
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("falls back when an initial theme is not registered", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialThemeId: "stale-theme",
      now: () => 3000,
    });

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "theme_unsupported",
        message: "Initial theme \"stale-theme\" is not registered for this workspace",
        recoverable: true,
        severity: "warn",
        timestampMs: 3000,
      },
    ]);

    await kernel.dispose();
  });

  it("keeps the default theme available when hosts register custom themes", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      availableThemeIds: ["acme-terminal"],
      initialThemeId: DEFAULT_WORKSPACE_THEME_ID,
    });

    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    kernel.commands.setTheme("acme-terminal");
    expect(kernel.selectors.themeId()).toBe("acme-terminal");

    kernel.commands.setTheme(DEFAULT_WORKSPACE_THEME_ID);
    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel terminal display preferences", () => {
  it("starts with explicit terminal display preferences", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialTerminalFontScale: " large ",
      initialTerminalLineWrap: false,
    });

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "large",
      lineWrap: false,
    });
    expect(kernel.diagnostics.list()).toEqual([]);

    await kernel.dispose();
  });

  it("updates terminal display preferences without touching theme state", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
    });

    kernel.commands.setTerminalFontScale("compact");
    kernel.commands.setTerminalLineWrap(false);

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "compact",
      lineWrap: false,
    });
    expect(kernel.selectors.themeId()).toBe(DEFAULT_WORKSPACE_THEME_ID);

    await kernel.dispose();
  });

  it("rejects unknown terminal font scales without corrupting preferences", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      now: () => 4000,
    });

    kernel.commands.setTerminalFontScale("poster");

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "default",
      lineWrap: true,
    });
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "terminal_display_preference_unsupported",
        message: 'Terminal font scale "poster" is not supported',
        recoverable: true,
        severity: "warn",
        timestampMs: 4000,
      },
    ]);

    await kernel.dispose();
  });

  it("falls back when an initial terminal font scale is unsupported", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      initialTerminalFontScale: "poster",
      now: () => 5000,
    });

    expect(kernel.selectors.terminalDisplay()).toEqual({
      fontScale: "default",
      lineWrap: true,
    });
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "terminal_display_preference_unsupported",
        message: 'Initial terminal font scale "poster" is not supported',
        recoverable: true,
        severity: "warn",
        timestampMs: 5000,
      },
    ]);

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel command history", () => {
  it("records normalized deduped command history entries in the core snapshot", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      commandHistoryLimit: 3,
    });

    kernel.commands.recordCommandHistory(" pwd \n");
    kernel.commands.recordCommandHistory("git status");
    kernel.commands.recordCommandHistory("pwd");
    kernel.commands.recordCommandHistory("printf ok");
    kernel.commands.recordCommandHistory("   ");

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["git status", "pwd", "printf ok"],
      limit: 3,
    });

    kernel.commands.clearCommandHistory();

    expect(kernel.getSnapshot().commandHistory).toEqual({
      entries: [],
      limit: 3,
    });

    await kernel.dispose();
  });

  it("falls back to the default command history limit", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      commandHistoryLimit: 0,
    });

    expect(kernel.getSnapshot().commandHistory).toEqual({
      entries: [],
      limit: DEFAULT_COMMAND_HISTORY_LIMIT,
    });

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel bootstrap", () => {
  it("loads advertised backend capabilities into the workspace catalog", async () => {
    const requestedBackends: BackendKind[] = [];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        handshake: async () => createHandshake(["native", "tmux"]),
        listSessions: async () => [],
        listSavedSessions: async () => [],
        getBackendCapabilities: async (backend: BackendKind) => {
          requestedBackends.push(backend);
          return createCapabilities(backend);
        },
      } as WorkspaceTransportClient,
    });

    await kernel.bootstrap();

    expect(requestedBackends).toEqual(["native", "tmux"]);
    expect(kernel.getSnapshot().catalog.backendCapabilities.native?.backend).toBe("native");
    expect(kernel.getSnapshot().catalog.backendCapabilities.tmux?.backend).toBe("tmux");

    await kernel.dispose();
  });

  it("discovers advertised foreign backend sessions during bootstrap", async () => {
    const requestedBackends: BackendKind[] = [];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        handshake: async () => createHandshake(["native", "tmux", "zellij"]),
        listSessions: async () => [],
        listSavedSessions: async () => [],
        getBackendCapabilities: async (backend: BackendKind) => createCapabilities(backend),
        discoverSessions: async (backend: BackendKind) => {
          requestedBackends.push(backend);
          return [createDiscoveredSession(backend)];
        },
      } as WorkspaceTransportClient,
    });

    await kernel.bootstrap();

    expect(requestedBackends).toEqual(["tmux", "zellij"]);
    expect(kernel.getSnapshot().catalog.discoveredSessions.tmux?.[0]?.route.backend).toBe("tmux");
    expect(kernel.getSnapshot().catalog.discoveredSessions.zellij?.[0]?.route.backend).toBe("zellij");

    await kernel.dispose();
  });

  it("keeps bootstrap usable when one capability probe fails", async () => {
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        handshake: async () => createHandshake(["native", "zellij"]),
        listSessions: async () => [],
        listSavedSessions: async () => [],
        getBackendCapabilities: async (backend: BackendKind) => {
          if (backend === "zellij") {
            throw new Error("zellij unavailable");
          }

          return createCapabilities(backend);
        },
      } as WorkspaceTransportClient,
      now: () => 6000,
    });

    await kernel.bootstrap();

    expect(kernel.selectors.connection().state).toBe("ready");
    expect(kernel.getSnapshot().catalog.backendCapabilities.native?.backend).toBe("native");
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "transport_failed",
        message: "zellij unavailable",
        recoverable: true,
        severity: "error",
        timestampMs: 6000,
        cause: expect.any(Error),
      },
    ]);

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel live session subscriptions", () => {
  it("applies pane surface updates without another attach", async () => {
    const live = createLiveScreenTransport();
    const kernel = createWorkspaceKernel({
      transport: live.transport,
    });

    await kernel.commands.attachSession(live.sessionId);
    expect(attachedScreenText(kernel)).toBe("ready");

    await kernel.commands.dispatchMuxCommand(live.sessionId, {
      kind: "send_input",
      pane_id: live.paneId,
      data: "echo live\r",
    });

    await waitUntil(() => attachedScreenText(kernel).includes("live output"));

    expect(attachedScreenText(kernel)).toContain("live output");
    expect(live.attachCalls()).toBe(1);

    await kernel.dispose();
  });
});

describe("createWorkspaceKernel saved session maintenance", () => {
  it("returns prune results and refreshes the saved session catalog", async () => {
    let savedSessions = [
      createSavedSessionSummary("saved-1", 1_000n),
      createSavedSessionSummary("saved-2", 2_000n),
      createSavedSessionSummary("saved-3", 3_000n),
    ];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        listSavedSessions: async () => savedSessions,
        pruneSavedSessions: async (keepLatest: number) => {
          const sorted = [...savedSessions].sort((left, right) => Number(right.saved_at_ms - left.saved_at_ms));
          savedSessions = sorted.slice(0, keepLatest);
          return {
            deleted_count: sorted.length - savedSessions.length,
            kept_count: savedSessions.length,
          };
        },
      } as WorkspaceTransportClient,
    });

    await kernel.commands.refreshSavedSessions();
    const result = await kernel.commands.pruneSavedSessions(2);

    expect(result).toEqual({
      deleted_count: 1,
      kept_count: 2,
    });
    expect(kernel.getSnapshot().catalog.savedSessions.map((session) => session.session_id)).toEqual([
      "saved-3",
      "saved-2",
    ]);

    await kernel.dispose();
  });

  it("blocks incompatible saved session restore before calling transport", async () => {
    const savedSession = createSavedSessionSummary("saved-1", 1_000n);
    savedSession.compatibility = {
      can_restore: false,
      status: "protocol_minor_ahead",
    };
    let restoreCalls = 0;
    const kernel = createWorkspaceKernel({
      now: () => 3_000,
      transport: {
        ...createUnusedTransport(),
        listSavedSessions: async () => [savedSession],
        restoreSavedSession: async () => {
          restoreCalls += 1;
          throw new Error("transport should not be called");
        },
      } as WorkspaceTransportClient,
    });

    await kernel.commands.refreshSavedSessions();
    await expect(kernel.commands.restoreSavedSession("saved-1")).rejects.toMatchObject({
      code: "unsupported_capability",
      recoverable: false,
    });

    expect(restoreCalls).toBe(0);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "unsupported_capability",
        message: "saved session saved-1 is not restore-compatible: protocol_minor_ahead",
        recoverable: false,
        severity: "error",
        timestampMs: 3_000,
      },
    ]);

    await kernel.dispose();
  });

  it("blocks invalid saved session prune limits before calling transport", async () => {
    let pruneCalls = 0;
    const kernel = createWorkspaceKernel({
      now: () => 4_000,
      transport: {
        ...createUnusedTransport(),
        pruneSavedSessions: async () => {
          pruneCalls += 1;
          return {
            deleted_count: 0,
            kept_count: 0,
          };
        },
      } as WorkspaceTransportClient,
    });

    await expect(kernel.commands.pruneSavedSessions(-1)).rejects.toMatchObject({
      code: "protocol_error",
      recoverable: false,
    });
    await expect(kernel.commands.pruneSavedSessions(Number.POSITIVE_INFINITY)).rejects.toMatchObject({
      code: "protocol_error",
      recoverable: false,
    });

    expect(pruneCalls).toBe(0);
    expect(kernel.diagnostics.list().map((diagnostic) => diagnostic.code)).toEqual([
      "protocol_error",
      "protocol_error",
    ]);

    await kernel.dispose();
  });
});

function createUnusedTransport(): WorkspaceTransportClient {
  return {
    close: async () => {},
    discoverSessions: async () => [],
  } as unknown as WorkspaceTransportClient;
}

function createLiveScreenTransport(): {
  transport: WorkspaceTransportClient;
  sessionId: SessionId;
  paneId: PaneId;
  attachCalls(): number;
} {
  const sessionId = "live-session-1";
  const paneId = "live-pane-1";
  const topology = createLiveTopology(sessionId, paneId);
  const attachedSession = createAttachedSession(sessionId, paneId, topology, "ready", 1n);
  const topologySubscription = new TestWorkspaceSubscription("live-topology-subscription");
  const paneSubscription = new TestWorkspaceSubscription("live-pane-subscription");
  let attachCount = 0;

  const transport: WorkspaceTransportClient = {
    ...createUnusedTransport(),
    attachSession: async () => {
      attachCount += 1;
      return structuredClone(attachedSession);
    },
    dispatchMuxCommand: async (_sessionId: SessionId, command: MuxCommand) => {
      if (command.kind === "send_input") {
        paneSubscription.push(createFullReplaceDelta(paneId, 1n, 2n, "ready\nlive output"));
      }

      return { changed: true };
    },
    openSubscription: async (_sessionId: SessionId, spec: SubscriptionSpec) => {
      if (spec.kind === "session_topology") {
        return topologySubscription;
      }

      return paneSubscription;
    },
  } as WorkspaceTransportClient;

  return {
    transport,
    sessionId,
    paneId,
    attachCalls: () => attachCount,
  };
}

class TestWorkspaceSubscription implements WorkspaceSubscription {
  readonly #subscriptionId: string;
  #closed = false;
  #events: SubscriptionEvent[] = [];
  #waiters: Array<(event: SubscriptionEvent | null) => void> = [];

  constructor(subscriptionId: string) {
    this.#subscriptionId = subscriptionId;
  }

  meta(): SubscriptionMeta {
    return {
      subscription_id: this.#subscriptionId,
    };
  }

  push(event: SubscriptionEvent): void {
    if (this.#closed) {
      return;
    }

    const waiter = this.#waiters.shift();
    if (waiter) {
      waiter(event);
      return;
    }

    this.#events.push(event);
  }

  async nextEvent(): Promise<SubscriptionEvent | null> {
    if (this.#closed) {
      return null;
    }

    const event = this.#events.shift();
    if (event) {
      return event;
    }

    return new Promise((resolve) => {
      this.#waiters.push(resolve);
    });
  }

  async close(): Promise<void> {
    if (this.#closed) {
      return;
    }

    this.#closed = true;
    this.#events = [];
    const waiters = this.#waiters.splice(0);
    for (const waiter of waiters) {
      waiter(null);
    }
  }
}

function createAttachedSession(
  sessionId: SessionId,
  paneId: PaneId,
  topology: TopologySnapshot,
  line: string,
  sequence: bigint,
): AttachedSession {
  return {
    session: {
      session_id: sessionId,
      route: {
        backend: "native",
        authority: "local_daemon",
        external: {
          namespace: "native_session",
          value: sessionId,
        },
      },
      title: "Live shell",
    },
    health: {
      session_id: sessionId,
      phase: "ready",
      can_attach: true,
      invalidated: false,
      reason: null,
      detail: null,
    },
    topology,
    focused_screen: createLiveScreen(paneId, line, sequence),
  };
}

function createLiveTopology(sessionId: SessionId, paneId: PaneId): TopologySnapshot {
  return {
    session_id: sessionId,
    backend_kind: "native",
    focused_tab: "live-tab-1",
    tabs: [
      {
        tab_id: "live-tab-1",
        title: "Live shell",
        root: {
          kind: "leaf",
          pane_id: paneId,
        },
        focused_pane: paneId,
      },
    ],
  };
}

function createLiveScreen(paneId: PaneId, line: string, sequence: bigint): ScreenSnapshot {
  return {
    pane_id: paneId,
    sequence,
    rows: 24,
    cols: 80,
    source: "native_emulator",
    surface: {
      title: "Live shell",
      cursor: {
        row: 0,
        col: line.length,
      },
      lines: line.split("\n").map((text) => ({ text })),
    },
  };
}

function createFullReplaceDelta(
  paneId: PaneId,
  fromSequence: bigint,
  toSequence: bigint,
  line: string,
): Extract<SubscriptionEvent, { kind: "screen_delta" }> {
  const screen = createLiveScreen(paneId, line, toSequence);
  return {
    kind: "screen_delta",
    pane_id: paneId,
    from_sequence: fromSequence,
    to_sequence: toSequence,
    rows: screen.rows,
    cols: screen.cols,
    source: screen.source,
    patch: null,
    full_replace: screen.surface,
  } satisfies Extract<SubscriptionEvent, { kind: "screen_delta" }>;
}

function attachedScreenText(kernel: ReturnType<typeof createWorkspaceKernel>): string {
  return kernel.getSnapshot().attachedSession?.focused_screen?.surface.lines
    .map((line) => line.text)
    .join("\n") ?? "";
}

async function waitUntil(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 1_000;

  while (Date.now() < deadline) {
    if (predicate()) {
      return;
    }

    await new Promise((resolve) => setTimeout(resolve, 10));
  }

  throw new Error("timed out waiting for workspace snapshot update");
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
    session_scope: "test",
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

function createDiscoveredSession(backend: BackendKind): DiscoveredSession {
  return {
    route: {
      backend,
      authority: "imported_foreign",
      external: {
        namespace: `${backend}_session`,
        value: "session=workspace",
      },
    },
    title: `${backend} workspace`,
  };
}

function createSavedSessionSummary(sessionId: string, savedAtMs: bigint): SavedSessionSummary {
  return {
    session_id: sessionId,
    route: {
      backend: "native",
      authority: "local_daemon",
      foreign_reference: null,
    },
    title: sessionId,
    saved_at_ms: savedAtMs,
    manifest: {
      format_version: 1,
      binary_version: "0.1.0-test",
      protocol_major: 0,
      protocol_minor: 2,
    },
    compatibility: {
      can_restore: true,
      status: "compatible",
    },
    has_launch: true,
    tab_count: 1,
    pane_count: 1,
    restore_semantics: {
      restores_topology: true,
      restores_focus_state: true,
      restores_tab_titles: true,
      uses_saved_launch_spec: true,
      replays_saved_screen_buffers: false,
      preserves_process_state: false,
    },
  };
}
