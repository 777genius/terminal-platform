import { describe, expect, it } from "vitest";

import type {
  AttachedSession,
  BackendCapabilitiesInfo,
  BackendKind,
  CommandHistoryEntry,
  DiscoveredSession,
  Handshake,
  MuxCommand,
  PaneHistory,
  PaneId,
  ScreenDelta,
  ScreenLine,
  ScreenLineSemanticMark,
  ScreenLineSpan,
  ScreenSnapshot,
  ScreenSurfacePalette,
  SavedSessionRecord,
  SavedSessionSummary,
  SessionId,
  SubscriptionEvent,
  SubscriptionMeta,
  SubscriptionSpec,
  TopologySnapshot,
} from "@terminal-platform/runtime-types";
import type {
  WorkspaceSubscription,
  WorkspaceTransportClient,
} from "@terminal-platform/workspace-contracts";

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
        message: 'Theme "missing-theme" is not registered for this workspace',
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
        message:
          'Initial theme "stale-theme" is not registered for this workspace',
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

  it("starts with normalized persisted command history entries", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      commandHistoryLimit: 3,
      initialCommandHistoryEntries: [
        "echo one",
        "git status\r\n",
        "echo one",
        "   ",
        "printf ok",
        "node -v",
      ],
    });

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["echo one", "printf ok", "node -v"],
      limit: 3,
    });

    kernel.commands.recordCommandHistory("printf ok ");

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["echo one", "node -v", "printf ok"],
      limit: 3,
    });

    await kernel.dispose();
  });

  it("strips shell prompt prefixes from command history entries", async () => {
    const kernel = createWorkspaceKernel({
      transport: createUnusedTransport(),
      commandHistoryLimit: 5,
      initialCommandHistoryEntries: [
        "(venv312) (base) belief@MacBook-Pro-belief terminal-ui-smoke % pnpm test",
        "belief@MacBook-Pro-belief terminal-ui-smoke %",
        "git status",
      ],
    });

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["pnpm test", "git status"],
      limit: 5,
    });

    kernel.commands.recordCommandHistory(
      'belief@MacBook-Pro-belief terminal-ui-smoke % printf "ok\\n"',
    );
    kernel.commands.recordCommandHistory("terminal-ui-smoke %");

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["pnpm test", "git status", 'printf "ok\\n"'],
      limit: 5,
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
    expect(
      kernel.getSnapshot().catalog.backendCapabilities.native?.backend,
    ).toBe("native");
    expect(kernel.getSnapshot().catalog.backendCapabilities.tmux?.backend).toBe(
      "tmux",
    );

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
        getBackendCapabilities: async (backend: BackendKind) =>
          createCapabilities(backend),
        discoverSessions: async (backend: BackendKind) => {
          requestedBackends.push(backend);
          return [createDiscoveredSession(backend)];
        },
      } as WorkspaceTransportClient,
    });

    await kernel.bootstrap();

    expect(requestedBackends).toEqual(["tmux", "zellij"]);
    expect(
      kernel.getSnapshot().catalog.discoveredSessions.tmux?.[0]?.route.backend,
    ).toBe("tmux");
    expect(
      kernel.getSnapshot().catalog.discoveredSessions.zellij?.[0]?.route
        .backend,
    ).toBe("zellij");

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
    expect(
      kernel.getSnapshot().catalog.backendCapabilities.native?.backend,
    ).toBe("native");
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

  it("preserves rich line spans from live pane surface updates", async () => {
    const sessionId = "live-rich-session";
    const paneId = "live-rich-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-rich-pane-sub");
    const richSpans: ScreenLineSpan[] = [
      {
        text: "green",
        style: {
          foreground: { kind: "named", name: "green" },
          background: null,
          underline_color: null,
          bold: true,
          dim: false,
          italic: false,
          blink: false,
          underline: null,
          overline: false,
          border: null,
          inverse: false,
          hidden: false,
          strikethrough: false,
          hyperlink: null,
        },
      },
    ];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push(
      createRichFullReplaceDelta(paneId, 1n, 2n, [
        { text: "green", spans: richSpans },
      ]),
    );

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface.lines[0]
          ?.text === "green",
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.lines[0]
        ?.spans,
    ).toEqual(richSpans);

    await kernel.dispose();
  });

  it("preserves shell integration semantic marks from live pane surface updates", async () => {
    const sessionId = "live-semantic-session";
    const paneId = "live-semantic-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-semantic-pane-sub");
    const semanticMarks: ScreenLineSemanticMark[] = [
      {
        kind: "command_finished",
        col: 0,
        exit_code: 127,
      },
    ];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push(
      createRichFullReplaceDelta(paneId, 1n, 2n, [
        { text: "", spans: [], semantic_marks: semanticMarks },
      ]),
    );

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface.lines[0]
          ?.semantic_marks?.[0]?.kind === "command_finished",
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.lines[0]
        ?.semantic_marks,
    ).toEqual(semanticMarks);

    await kernel.dispose();
  });

  it("applies working directory URI metadata from live pane deltas", async () => {
    const sessionId = "live-working-directory-session";
    const paneId = "live-working-directory-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-working-directory-pane-sub");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 1n,
      to_sequence: 2n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: true,
        working_directory_uri: "file://localhost/tmp/project",
        user_variables_changed: false,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: false,
        palette: null,
        bell_count_changed: false,
        bell_count: null,
        progress_changed: false,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface
          .working_directory_uri === "file://localhost/tmp/project",
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface
        .working_directory_uri,
    ).toBe("file://localhost/tmp/project");

    await kernel.dispose();
  });

  it("applies terminal user variables from live pane deltas", async () => {
    const sessionId = "live-user-variables-session";
    const paneId = "live-user-variables-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-user-variables-pane-sub");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 1n,
      to_sequence: 2n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: true,
        user_variables: {
          WEZTERM_PROG: "cargo test",
          WEZTERM_USER: "belief",
        },
        cursor_changed: false,
        cursor: null,
        palette_changed: false,
        palette: null,
        bell_count_changed: false,
        bell_count: null,
        progress_changed: false,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface
          .user_variables?.WEZTERM_PROG === "cargo test",
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.user_variables,
    ).toEqual({
      WEZTERM_PROG: "cargo test",
      WEZTERM_USER: "belief",
    });

    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 2n,
      to_sequence: 3n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: true,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: false,
        palette: null,
        bell_count_changed: false,
        bell_count: null,
        progress_changed: false,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.sequence === 3n,
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.user_variables,
    ).toBeUndefined();

    await kernel.dispose();
  });

  it("preserves terminal progress metadata from live pane deltas", async () => {
    const sessionId = "live-progress-session";
    const paneId = "live-progress-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-progress-pane-sub");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 1n,
      to_sequence: 2n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: false,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: false,
        palette: null,
        bell_count_changed: false,
        bell_count: null,
        progress_changed: true,
        progress: { state: "normal", value: 73 },
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface.progress
          ?.value === 73,
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.progress,
    ).toEqual({ state: "normal", value: 73 });

    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 2n,
      to_sequence: 3n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: false,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: false,
        palette: null,
        bell_count_changed: false,
        bell_count: null,
        progress_changed: true,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.sequence === 3n,
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.progress,
    ).toBeUndefined();

    await kernel.dispose();
  });

  it("applies terminal palette and bell metadata from live pane deltas", async () => {
    const sessionId = "live-palette-bell-session";
    const paneId = "live-palette-bell-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-palette-bell-pane-sub");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 1n,
      to_sequence: 2n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: false,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: true,
        palette: {
          foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
          background: { kind: "rgb", r: 4, g: 5, b: 6 },
          cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
        },
        bell_count_changed: true,
        bell_count: 3n,
        progress_changed: false,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.surface.palette
          ?.foreground?.kind === "rgb",
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.palette,
    ).toEqual({
      foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
      background: { kind: "rgb", r: 4, g: 5, b: 6 },
      cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
    });
    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.bell_count,
    ).toBe(3n);

    subscription.push({
      kind: "screen_delta",
      pane_id: paneId,
      from_sequence: 2n,
      to_sequence: 3n,
      rows: 24,
      cols: 80,
      source: "native_emulator",
      patch: {
        title_changed: false,
        title: "Live shell",
        working_directory_uri_changed: false,
        working_directory_uri: null,
        user_variables_changed: false,
        user_variables: null,
        cursor_changed: false,
        cursor: null,
        palette_changed: true,
        palette: null,
        bell_count_changed: true,
        bell_count: null,
        progress_changed: false,
        progress: null,
        line_updates: [],
      },
      full_replace: null,
    });

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.sequence === 3n,
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.palette,
    ).toBeUndefined();
    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.surface.bell_count,
    ).toBeUndefined();

    await kernel.dispose();
  });

  it("preserves alternate screen buffer kind from live pane deltas", async () => {
    const sessionId = "live-alt-screen-session";
    const paneId = "live-alt-screen-pane";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "ready",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("live-alt-pane-sub");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
      } as WorkspaceTransportClient,
    });

    await kernel.commands.attachSession(sessionId);
    subscription.push(
      createFullReplaceDelta(paneId, 1n, 2n, "vim screen", "alternate"),
    );

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.buffer_kind
        === "alternate",
    );

    expect(kernel.getSnapshot().attachedSession?.focused_screen.buffer_kind).toBe(
      "alternate",
    );

    subscription.push(createFullReplaceDelta(paneId, 2n, 3n, "shell"));

    await waitUntil(
      () =>
        kernel.getSnapshot().attachedSession?.focused_screen.sequence === 3n,
    );

    expect(
      kernel.getSnapshot().attachedSession?.focused_screen.buffer_kind,
    ).toBeUndefined();

    await kernel.dispose();
  });

  it("hydrates pane history from persistence when attaching a session", async () => {
    const sessionId = "history-session-1";
    const paneId = "history-pane-1";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-pane-subscription",
    );
    let historyCalls = 0;
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () => {
          historyCalls += 1;
          return createPaneHistory(
            sessionId,
            paneId,
            "\x1B]52;c;ZmFrZS1jbGlwYm9hcmQ=\x07\x1B]0;fake-title\x07\x1B[31mgit status\x1B[0m\r\nfatal\r\n",
          );
        },
      } as WorkspaceTransportClient,
      now: () => 7000,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () =>
        kernel
          .getSnapshot()
          .historicalPanes?.[paneId]?.lines.includes("fatal") ?? false,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      sessionId,
      paneId,
      source: "v2_pane_history",
      replayStrategy: "raw_vt_stream",
      restoreGuaranteeLevel: "basic_history",
      lines: ["git status", "fatal"],
      hasGaps: false,
      hasMoreSegments: false,
    });
    const restoredHistoryText =
      kernel.getSnapshot().historicalPanes?.[paneId]?.lines.join("\n") ?? "";
    expect(restoredHistoryText).not.toContain("fake-title");
    expect(restoredHistoryText).not.toContain("fake-clipboard");
    expect(historyCalls).toBeGreaterThan(0);

    await kernel.dispose();
  });

  it("loads pane history pages from the persisted cursor", async () => {
    const sessionId = "history-session-pages";
    const paneId = "history-pane-pages";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-pane-pages-subscription",
    );
    const historyRequests: Array<{ fromEventSeq?: number | bigint | null }> =
      [];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async (_sessionId, _paneId, options) => {
          historyRequests.push({ fromEventSeq: options?.fromEventSeq });
          if (options?.fromEventSeq === 2n) {
            return createPaneHistory(sessionId, paneId, "second page\r\n", {
              fromEventSeq: 2n,
              eventSeqLow: 2n,
              eventSeqHigh: 2n,
              hasMoreSegments: false,
              nextEventSeq: null,
              segmentId: "segment-2",
            });
          }

          return createPaneHistory(sessionId, paneId, "first page\r\n", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: true,
            nextEventSeq: 2n,
            segmentId: "segment-1",
          });
        },
      } as WorkspaceTransportClient,
      now: () => 7_500,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () =>
        kernel.getSnapshot().historicalPanes?.[paneId]?.hasMoreSegments ??
        false,
    );

    await expect(kernel.commands.loadMorePaneHistory(paneId)).resolves.toBe(
      true,
    );

    const requestedPages = historyRequests.map(
      (request) => request.fromEventSeq,
    );
    expect(requestedPages[0]).toBe(1n);
    expect(requestedPages.at(-1)).toBe(2n);
    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["first page", "second page"],
      hasMoreSegments: false,
      nextEventSeq: null,
      segmentCount: 2,
    });

    await kernel.dispose();
  });

  it("does not mix snapshot fallback into partial v2 history pages", async () => {
    const sessionId = "history-session-snapshot-page";
    const paneId = "history-pane-snapshot-page";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-snapshot-page-subscription",
    );
    const historyRequests: Array<{ fromEventSeq?: number | bigint | null }> =
      [];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async (_sessionId, _paneId, options) => {
          historyRequests.push({ fromEventSeq: options?.fromEventSeq });
          if (options?.fromEventSeq === 2n) {
            return createPaneHistory(sessionId, paneId, "journal page\r\n", {
              fromEventSeq: 2n,
              eventSeqLow: 2n,
              eventSeqHigh: 2n,
              hasMoreSegments: false,
              nextEventSeq: null,
              segmentId: "segment-2",
            });
          }

          return createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: true,
            includeSegment: false,
            latestScreenText: "latest snapshot fallback",
            nextEventSeq: 2n,
          });
        },
      } as WorkspaceTransportClient,
      now: () => 7_600,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () =>
        kernel.getSnapshot().historicalPanes?.[paneId]?.hasMoreSegments ??
        false,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: [],
      capturedAtMs: 7_600n,
      hasMoreSegments: true,
      nextEventSeq: 2n,
    });

    await expect(kernel.commands.loadMorePaneHistory(paneId)).resolves.toBe(
      true,
    );

    expect(historyRequests.map((request) => request.fromEventSeq)).toEqual([
      1n,
      2n,
    ]);
    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["journal page"],
      hasMoreSegments: false,
      nextEventSeq: null,
    });
    expect(kernel.getSnapshot().historicalPanes?.[paneId]?.lines).not.toContain(
      "latest snapshot fallback",
    );

    await kernel.dispose();
  });

  it("hydrates rich rendered snapshot fallback from v2 pane history", async () => {
    const sessionId = "history-session-rich-snapshot";
    const paneId = "history-pane-rich-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const richStyle = {
      foreground: { kind: "named", name: "red" },
      background: null,
      underline_color: null,
      bold: true,
      dim: false,
      italic: false,
      blink: true,
      underline: null,
      overline: true,
      border: "framed",
      inverse: false,
      hidden: false,
      strikethrough: false,
      hyperlink: null,
    } satisfies ScreenLineSpan["style"];
    const richSpans: ScreenLineSpan[] = [
      {
        text: "red",
        style: richStyle,
      },
      {
        text: "   ",
        style: {
          foreground: null,
          background: null,
          underline_color: null,
          bold: false,
          dim: false,
          italic: false,
          blink: false,
          underline: null,
          overline: false,
          border: null,
          inverse: false,
          hidden: false,
          strikethrough: false,
          hyperlink: null,
        },
      },
    ];
    const subscription = new TestWorkspaceSubscription(
      "history-rich-snapshot-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenLine: { text: "red   ", spans: richSpans },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_600,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.lines[0] === "red",
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["red"],
      richLines: [
        {
          text: "red",
          spans: [{ text: "red", style: richStyle }],
        },
      ],
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("does not store plain span-only rendered snapshot fallback as rich history", async () => {
    const sessionId = "history-session-plain-span-snapshot";
    const paneId = "history-pane-plain-span-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const plainStyle = {
      foreground: null,
      background: null,
      underline_color: null,
      bold: false,
      dim: false,
      italic: false,
      blink: false,
      underline: null,
      overline: false,
      border: null,
      inverse: false,
      hidden: false,
      strikethrough: false,
      hyperlink: null,
    } satisfies ScreenLineSpan["style"];
    const subscription = new TestWorkspaceSubscription(
      "history-plain-span-snapshot-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenLine: {
              text: "plain",
              spans: [
                { text: "pla", style: plainStyle },
                { text: "in", style: plainStyle },
              ],
            },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_650,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.lines[0] === "plain",
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["plain"],
      replayStrategy: "raw_vt_stream",
    });
    expect(kernel.getSnapshot().historicalPanes?.[paneId]?.richLines).toBeUndefined();

    await kernel.dispose();
  });

  it("hydrates side-effect-only rendered snapshot fallback from v2 pane history", async () => {
    const sessionId = "history-session-side-effect-snapshot";
    const paneId = "history-pane-side-effect-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-side-effect-snapshot-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenLine: {
              text: "",
              spans: [],
              side_effects: [
                {
                  kind: "desktop_notification",
                  disposition: "blocked",
                  target: "desktop_notification",
                  message: "Build finished",
                },
              ],
            },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_700,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.richLines?.length === 1,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: [""],
      richLines: [
        {
          text: "",
          spans: [],
          side_effects: [
            {
              kind: "desktop_notification",
              disposition: "blocked",
              target: "desktop_notification",
              message: "Build finished",
            },
          ],
        },
      ],
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("hydrates media-only rendered snapshot fallback from v2 pane history", async () => {
    const sessionId = "history-session-media-snapshot";
    const paneId = "history-pane-media-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription("history-media-snapshot-subscription");
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenLine: {
              text: "",
              spans: [],
              media: [
                {
                  kind: "iterm2_image",
                  name: "tiny.png",
                  byte_size: 68,
                  width: "2",
                  height: "1",
                  preserve_aspect_ratio: true,
                  inline: true,
                  mime_type: "image/png",
                  data_base64: "iVBORw0KGgo=",
                },
              ],
            },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_700,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.richLines?.length === 1,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: [""],
      richLines: [
        {
          text: "",
          spans: [],
          media: [
            {
              kind: "iterm2_image",
              name: "tiny.png",
              byte_size: 68,
              width: "2",
              height: "1",
              preserve_aspect_ratio: true,
              inline: true,
              mime_type: "image/png",
              data_base64: "iVBORw0KGgo=",
            },
          ],
        },
      ],
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("hydrates semantic-only rendered snapshot fallback from v2 pane history", async () => {
    const sessionId = "history-session-semantic-snapshot";
    const paneId = "history-pane-semantic-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-semantic-snapshot-subscription",
    );
    const semanticMarks: ScreenLineSemanticMark[] = [
      {
        kind: "command_finished",
        col: 0,
        exit_code: 2,
      },
    ];
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenLine: {
              text: "",
              spans: [],
              semantic_marks: semanticMarks,
            },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_700,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.richLines?.length === 1,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: [""],
      richLines: [
        {
          text: "",
          spans: [],
          semantic_marks: semanticMarks,
        },
      ],
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("hydrates surface palette from rendered snapshot fallback", async () => {
    const sessionId = "history-session-palette-snapshot";
    const paneId = "history-pane-palette-snapshot";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const surfacePalette: ScreenSurfacePalette = {
      foreground: { kind: "rgb", r: 1, g: 2, b: 3 },
      background: { kind: "rgb", r: 4, g: 5, b: 6 },
      cursor: { kind: "rgb", r: 7, g: 8, b: 9 },
    };
    const subscription = new TestWorkspaceSubscription(
      "history-palette-snapshot-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            includeSegment: false,
            latestScreenText: "plain",
            latestScreenPalette: surfacePalette,
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_650,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.lines[0] === "plain",
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["plain"],
      surfacePalette,
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("keeps rich snapshot lines when they match raw v2 pane history text", async () => {
    const sessionId = "history-session-rich-raw-aligned";
    const paneId = "history-pane-rich-raw-aligned";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const richStyle = {
      foreground: { kind: "rgb", r: 12, g: 34, b: 56 },
      background: null,
      underline_color: null,
      bold: true,
      dim: false,
      italic: false,
      blink: false,
      underline: null,
      overline: false,
      border: null,
      inverse: false,
      hidden: false,
      strikethrough: false,
      hyperlink: null,
    } satisfies ScreenLineSpan["style"];
    const richSpans: ScreenLineSpan[] = [{ text: "red", style: richStyle }];
    const subscription = new TestWorkspaceSubscription(
      "history-rich-raw-aligned-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async () =>
          createPaneHistory(sessionId, paneId, "red\r\n", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: false,
            latestScreenLine: { text: "red", spans: richSpans },
            nextEventSeq: null,
          }),
      } as WorkspaceTransportClient,
      now: () => 7_700,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () => kernel.getSnapshot().historicalPanes?.[paneId]?.lines[0] === "red",
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["red"],
      richLines: [{ text: "red", spans: richSpans }],
      replayStrategy: "raw_vt_stream",
    });

    await kernel.dispose();
  });

  it("keeps pane history paging stable across stream line boundaries", async () => {
    const sessionId = "history-session-line-boundary";
    const paneId = "history-pane-line-boundary";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-line-boundary-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async (_sessionId, _paneId, options) => {
          if (options?.fromEventSeq === 2n) {
            return createPaneHistory(sessionId, paneId, " line\r\nnext\r\n", {
              fromEventSeq: 2n,
              eventSeqLow: 2n,
              eventSeqHigh: 2n,
              hasMoreSegments: false,
              nextEventSeq: null,
              segmentId: "segment-2",
            });
          }

          return createPaneHistory(sessionId, paneId, "partial", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: true,
            nextEventSeq: 2n,
            segmentId: "segment-1",
          });
        },
      } as WorkspaceTransportClient,
      now: () => 7_700,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () =>
        kernel.getSnapshot().historicalPanes?.[paneId]?.hasMoreSegments ??
        false,
    );

    await expect(kernel.commands.loadMorePaneHistory(paneId)).resolves.toBe(
      true,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["partial line", "next"],
      hasMoreSegments: false,
      nextEventSeq: null,
      segmentCount: 2,
    });

    await kernel.dispose();
  });

  it("closes pane history cursor when the final page has no visible lines", async () => {
    const sessionId = "history-session-empty-final";
    const paneId = "history-pane-empty-final";
    const topology = createLiveTopology(sessionId, paneId);
    const attachedSession = createAttachedSession(
      sessionId,
      paneId,
      topology,
      "live prompt",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "history-empty-final-subscription",
    );
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachSession: async () => structuredClone(attachedSession),
        openSubscription: async () => subscription,
        getPaneHistory: async (_sessionId, _paneId, options) => {
          if (options?.fromEventSeq === 2n) {
            return createPaneHistory(sessionId, paneId, "\x1B[?25l", {
              fromEventSeq: 2n,
              eventSeqLow: 2n,
              eventSeqHigh: 2n,
              hasMoreSegments: false,
              nextEventSeq: null,
              segmentId: "segment-2",
            });
          }

          return createPaneHistory(sessionId, paneId, "first page\r\n", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
            hasMoreSegments: true,
            nextEventSeq: 2n,
            segmentId: "segment-1",
          });
        },
      } as WorkspaceTransportClient,
      now: () => 7_800,
    });

    await kernel.commands.attachSession(sessionId);
    await waitUntil(
      () =>
        kernel.getSnapshot().historicalPanes?.[paneId]?.hasMoreSegments ??
        false,
    );

    await expect(kernel.commands.loadMorePaneHistory(paneId)).resolves.toBe(
      true,
    );

    expect(kernel.getSnapshot().historicalPanes?.[paneId]).toMatchObject({
      lines: ["first page"],
      hasMoreSegments: false,
      nextEventSeq: null,
      segmentCount: 2,
    });

    await kernel.dispose();
  });

  it("hydrates command history from persistence during bootstrap", async () => {
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        handshake: async () => createHandshake(["native"]),
        listSessions: async () => [],
        listSavedSessions: async () => [],
        getBackendCapabilities: async (backend: BackendKind) =>
          createCapabilities(backend),
        listCommandHistory: async () => [
          createCommandHistoryEntry("git status", 2_000n),
          createCommandHistoryEntry("pwd", 1_000n),
        ],
      } as WorkspaceTransportClient,
      commandHistoryLimit: 4,
      initialCommandHistoryEntries: ["node -v"],
    });

    await kernel.bootstrap();

    expect(kernel.selectors.commandHistory()).toEqual({
      entries: ["node -v", "pwd", "git status"],
      limit: 4,
    });

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
          const sorted = [...savedSessions].sort((left, right) =>
            Number(right.saved_at_ms - left.saved_at_ms),
          );
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
    expect(
      kernel
        .getSnapshot()
        .catalog.savedSessions.map((session) => session.session_id),
    ).toEqual(["saved-3", "saved-2"]);

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
    await expect(
      kernel.commands.restoreSavedSession("saved-1"),
    ).rejects.toMatchObject({
      code: "unsupported_capability",
      recoverable: false,
    });

    expect(restoreCalls).toBe(0);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "unsupported_capability",
        message:
          "saved session saved-1 is not restore-compatible: protocol_minor_ahead",
        recoverable: false,
        severity: "error",
        timestampMs: 3_000,
      },
    ]);

    await kernel.dispose();
  });

  it("attaches restored sessions with saved visible history", async () => {
    const savedRecord = createSavedSessionRecord(
      "saved-1",
      "saved-pane-1",
      "historical output",
    );
    const restoredSessionId = "restored-session-1";
    const livePaneId = "live-pane-1";
    const liveTopology = createLiveTopology(restoredSessionId, livePaneId);
    const attachedSession = createAttachedSession(
      restoredSessionId,
      livePaneId,
      liveTopology,
      "new live prompt",
      1n,
    );

    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        listSavedSessions: async () => [
          savedSessionRecordToSummary(savedRecord),
        ],
        getSavedSession: async () => savedRecord,
        restoreSavedSession: async () => ({
          saved_session_id: savedRecord.session_id,
          manifest: savedRecord.manifest,
          compatibility: savedRecord.compatibility,
          session: {
            session_id: restoredSessionId,
            route: savedRecord.route,
            title: savedRecord.title,
          },
          restore_semantics: savedRecord.restore_semantics,
        }),
        attachSession: async () => attachedSession,
      } as WorkspaceTransportClient,
      now: () => 5_000,
    });

    await kernel.commands.refreshSavedSessions();
    await kernel.commands.restoreSavedSession(savedRecord.session_id);

    const snapshot = kernel.getSnapshot();
    expect(snapshot.attachedSession?.session.session_id).toBe(
      restoredSessionId,
    );
    expect(snapshot.selection).toEqual({
      activeSessionId: restoredSessionId,
      activePaneId: livePaneId,
    });
    expect(snapshot.historicalPanes?.[livePaneId]).toMatchObject({
      sessionId: restoredSessionId,
      paneId: livePaneId,
      sourceSessionId: savedRecord.session_id,
      sourcePaneId: "saved-pane-1",
      lines: ["historical output"],
      replayStrategy: "rendered_snapshot",
      restoreGuaranteeLevel: "visual_snapshot_only",
    });

    await kernel.dispose();
  });

  it("attaches restored sessions with saved rich visible history", async () => {
    const savedRecord = createSavedSessionRecord(
      "saved-rich-history",
      "saved-pane-1",
      "red",
    );
    const richSpans: ScreenLineSpan[] = [
      {
        text: "red",
        style: {
          foreground: { kind: "named", name: "red" },
          background: null,
          underline_color: null,
          bold: true,
          dim: false,
          italic: false,
          blink: false,
          underline: null,
          overline: false,
          border: null,
          inverse: false,
          hidden: false,
          strikethrough: false,
          hyperlink: null,
        },
      },
    ];
    savedRecord.screens[0]!.surface.lines = [{ text: "red", spans: richSpans, wrapped: true }];
    const restoredSessionId = "restored-rich-history";
    const livePaneId = "live-pane-1";
    const liveTopology = createLiveTopology(restoredSessionId, livePaneId);
    const attachedSession = createAttachedSession(
      restoredSessionId,
      livePaneId,
      liveTopology,
      "new live prompt",
      1n,
    );

    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        listSavedSessions: async () => [
          savedSessionRecordToSummary(savedRecord),
        ],
        getSavedSession: async () => savedRecord,
        restoreSavedSession: async () => ({
          saved_session_id: savedRecord.session_id,
          manifest: savedRecord.manifest,
          compatibility: savedRecord.compatibility,
          session: {
            session_id: restoredSessionId,
            route: savedRecord.route,
            title: savedRecord.title,
          },
          restore_semantics: savedRecord.restore_semantics,
        }),
        attachSession: async () => attachedSession,
      } as WorkspaceTransportClient,
      now: () => 5_000,
    });

    await kernel.commands.refreshSavedSessions();
    await kernel.commands.restoreSavedSession(savedRecord.session_id);

    expect(kernel.getSnapshot().historicalPanes?.[livePaneId]).toMatchObject({
      lines: ["red"],
      richLines: [{ text: "red", spans: richSpans, wrapped: true }],
    });

    await kernel.dispose();
  });

  it("hydrates restored sessions from v2 journal before falling back to saved screens", async () => {
    const savedRecord = createSavedSessionRecord(
      "saved-v2-history",
      "saved-pane-1",
      "snapshot fallback",
    );
    const restoredSessionId = "restored-v2-history";
    const livePaneId = "live-pane-1";
    const liveTopology = createLiveTopology(restoredSessionId, livePaneId);
    const attachedSession = createAttachedSession(
      restoredSessionId,
      livePaneId,
      liveTopology,
      "new live prompt",
      1n,
    );
    const historyRequests: Array<{
      sessionId: string;
      paneId: string;
      fromEventSeq?: number | bigint | null;
    }> = [];

    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        historyRequests,
        listSavedSessions: async () => [
          savedSessionRecordToSummary(savedRecord),
        ],
        getSavedSession: async () => savedRecord,
        getPaneHistory: async function (
          this: { historyRequests: typeof historyRequests },
          sessionId,
          paneId,
          options,
        ) {
          this.historyRequests.push({
            sessionId,
            paneId,
            fromEventSeq: options?.fromEventSeq,
          });
          return createPaneHistory(sessionId, paneId, "journal output\r\n", {
            fromEventSeq: 1n,
            eventSeqLow: 1n,
            eventSeqHigh: 1n,
          });
        },
        restoreSavedSession: async () => ({
          saved_session_id: savedRecord.session_id,
          manifest: savedRecord.manifest,
          compatibility: savedRecord.compatibility,
          session: {
            session_id: restoredSessionId,
            route: savedRecord.route,
            title: savedRecord.title,
          },
          restore_semantics: savedRecord.restore_semantics,
        }),
        attachSession: async () => attachedSession,
      } as WorkspaceTransportClient,
      now: () => 5_500,
    });

    await kernel.commands.refreshSavedSessions();
    await kernel.commands.restoreSavedSession(savedRecord.session_id);

    expect(historyRequests).toEqual([
      {
        sessionId: savedRecord.session_id,
        paneId: "saved-pane-1",
        fromEventSeq: 1n,
      },
    ]);
    expect(kernel.getSnapshot().historicalPanes?.[livePaneId]).toMatchObject({
      sessionId: restoredSessionId,
      paneId: livePaneId,
      sourceSessionId: savedRecord.session_id,
      sourcePaneId: "saved-pane-1",
      source: "v2_pane_history",
      lines: ["journal output"],
      replayStrategy: "raw_vt_stream",
      restoreGuaranteeLevel: "basic_history",
    });

    await kernel.dispose();
  });

  it("keeps restore successful when immediate attach for history fails", async () => {
    const savedRecord = createSavedSessionRecord(
      "saved-attach-fails",
      "saved-pane-1",
      "historical output",
    );
    const restoredSessionId = "restored-attach-fails";
    const previousSessionId = "previous-live-session";
    const previousPaneId = "previous-live-pane";
    const previousTopology = createLiveTopology(
      previousSessionId,
      previousPaneId,
    );
    const previousAttachedSession = createAttachedSession(
      previousSessionId,
      previousPaneId,
      previousTopology,
      "previous live output",
      1n,
    );
    const subscription = new TestWorkspaceSubscription(
      "previous-live-subscription",
    );
    let openSubscriptionCalls = 0;
    const kernel = createWorkspaceKernel({
      transport: {
        ...createUnusedTransport(),
        attachCalls: 0,
        listSavedSessions: async () => [
          savedSessionRecordToSummary(savedRecord),
        ],
        getSavedSession: async () => savedRecord,
        restoreSavedSession: async () => ({
          saved_session_id: savedRecord.session_id,
          manifest: savedRecord.manifest,
          compatibility: savedRecord.compatibility,
          session: {
            session_id: restoredSessionId,
            route: savedRecord.route,
            title: savedRecord.title,
          },
          restore_semantics: savedRecord.restore_semantics,
        }),
        attachSession: async function (this: { attachCalls: number }) {
          this.attachCalls += 1;
          if (this.attachCalls === 1) {
            return previousAttachedSession;
          }
          throw new Error("attach unavailable");
        },
        openSubscription: async () => {
          openSubscriptionCalls += 1;
          return subscription;
        },
      } as WorkspaceTransportClient,
      now: () => 6_000,
    });

    await kernel.commands.attachSession(previousSessionId);
    await waitUntil(() => openSubscriptionCalls > 0);
    await kernel.commands.refreshSavedSessions();
    await kernel.commands.restoreSavedSession(savedRecord.session_id);

    expect(kernel.getSnapshot().selection).toEqual({
      activeSessionId: restoredSessionId,
      activePaneId: null,
    });
    expect(kernel.getSnapshot().attachedSession).toBeNull();
    expect(
      kernel
        .getSnapshot()
        .catalog.sessions.map((session) => session.session_id),
    ).toContain(restoredSessionId);
    expect(kernel.diagnostics.list()).toEqual([
      {
        code: "restored_session_attach_failed",
        message: `failed to attach restored session ${restoredSessionId}`,
        recoverable: true,
        severity: "warn",
        timestampMs: 6_000,
        cause: expect.any(Error),
      },
    ]);
    expect(subscription.isClosed()).toBe(true);

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
    await expect(
      kernel.commands.pruneSavedSessions(Number.POSITIVE_INFINITY),
    ).rejects.toMatchObject({
      code: "protocol_error",
      recoverable: false,
    });

    expect(pruneCalls).toBe(0);
    expect(
      kernel.diagnostics.list().map((diagnostic) => diagnostic.code),
    ).toEqual(["protocol_error", "protocol_error"]);

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
  const attachedSession = createAttachedSession(
    sessionId,
    paneId,
    topology,
    "ready",
    1n,
  );
  const topologySubscription = new TestWorkspaceSubscription(
    "live-topology-subscription",
  );
  const paneSubscription = new TestWorkspaceSubscription(
    "live-pane-subscription",
  );
  let attachCount = 0;

  const transport: WorkspaceTransportClient = {
    ...createUnusedTransport(),
    attachSession: async () => {
      attachCount += 1;
      return structuredClone(attachedSession);
    },
    dispatchMuxCommand: async (_sessionId: SessionId, command: MuxCommand) => {
      if (command.kind === "send_input") {
        paneSubscription.push(
          createFullReplaceDelta(paneId, 1n, 2n, "ready\nlive output"),
        );
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

  isClosed(): boolean {
    return this.#closed;
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

function createLiveTopology(
  sessionId: SessionId,
  paneId: PaneId,
): TopologySnapshot {
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

function createLiveScreen(
  paneId: PaneId,
  line: string,
  sequence: bigint,
): ScreenSnapshot {
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

interface CreatePaneHistoryOptions {
  readonly fromEventSeq?: bigint;
  readonly eventSeqLow?: bigint;
  readonly eventSeqHigh?: bigint;
  readonly hasMoreSegments?: boolean;
  readonly includeSegment?: boolean;
  readonly latestScreenPalette?: ScreenSurfacePalette;
  readonly latestScreenLine?: ScreenLine;
  readonly latestScreenText?: string;
  readonly nextEventSeq?: bigint | null;
  readonly segmentId?: string;
  readonly createdAtMs?: bigint;
}

function createPaneHistory(
  sessionId: SessionId,
  paneId: PaneId,
  text: string,
  options: CreatePaneHistoryOptions = {},
): PaneHistory {
  const encoded = new TextEncoder().encode(text);
  const fromEventSeq = options.fromEventSeq ?? 1n;
  const eventSeqLow = options.eventSeqLow ?? fromEventSeq;
  const eventSeqHigh = options.eventSeqHigh ?? eventSeqLow;

  return {
    session_id: sessionId,
    pane_id: paneId,
    from_event_seq: fromEventSeq,
    max_segments: 256n,
    max_bytes: 1_048_576n,
    restore_plan: {
      session_id: sessionId,
      restore_guarantee_level: "basic_history",
      latest_screen_snapshot_id: null,
      latest_topology_snapshot_id: null,
      high_water_commit_seq: eventSeqHigh,
      evidence: [
        {
          kind: "stream_segment_count",
          value: "1",
        },
      ],
    },
    latest_screen_snapshot:
      options.latestScreenText == null && options.latestScreenLine == null
        ? null
        : {
            id: "snapshot-1",
            pane_id: paneId,
            projection_source: "native_emulator",
            buffer_kind: "screen",
            rows: 24,
            cols: 80,
            base_event_seq: fromEventSeq,
            high_water_event_seq: eventSeqHigh,
            high_water_byte_seq: null,
            screen_json: JSON.stringify({
              surface: {
                ...(options.latestScreenPalette
                  ? { palette: options.latestScreenPalette }
                  : {}),
                lines: [
                  options.latestScreenLine ?? {
                    text: options.latestScreenText ?? "",
                    spans: [],
                  },
                ],
              },
            }),
            parser_version: "test-parser",
            projection_version: "test-projection",
            checksum: "test-snapshot-checksum",
            created_at_ms: options.createdAtMs ?? 7_000n,
          },
    segments:
      options.includeSegment === false
        ? []
        : [
            {
              id: options.segmentId ?? "segment-1",
              event_seq_low: eventSeqLow,
              event_seq_high: eventSeqHigh,
              byte_low: 0n,
              byte_high: BigInt(encoded.byteLength),
              payload: Array.from(encoded),
              checksum: "test-checksum",
              capture_semantics: "raw_vt_stream",
              created_at_ms: options.createdAtMs ?? 7_000n,
            },
          ],
    gaps: [],
    replay_strategy: "raw_vt_stream",
    has_more_segments: options.hasMoreSegments ?? false,
    next_event_seq: options.nextEventSeq ?? null,
    total_payload_bytes: BigInt(encoded.byteLength),
  };
}

function createFullReplaceDelta(
  paneId: PaneId,
  fromSequence: bigint,
  toSequence: bigint,
  line: string,
  bufferKind?: ScreenDelta["buffer_kind"],
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
    ...(bufferKind ? { buffer_kind: bufferKind } : {}),
    patch: null,
    full_replace: screen.surface,
  } satisfies Extract<SubscriptionEvent, { kind: "screen_delta" }>;
}

function createRichFullReplaceDelta(
  paneId: PaneId,
  fromSequence: bigint,
  toSequence: bigint,
  lines: Array<{
    readonly text: string;
    readonly spans: ScreenLineSpan[];
    readonly semantic_marks?: ScreenLineSemanticMark[];
    readonly wrapped?: boolean;
  }>,
): Extract<SubscriptionEvent, { kind: "screen_delta" }> {
  return {
    kind: "screen_delta",
    pane_id: paneId,
    from_sequence: fromSequence,
    to_sequence: toSequence,
    rows: 24,
    cols: 80,
    source: "native_emulator",
    patch: null,
    full_replace: {
      title: "Live shell",
      cursor: {
        row: 0,
        col: lines.at(-1)?.text.length ?? 0,
      },
      lines: lines.map((line) => ({
        text: line.text,
        spans: line.spans,
        ...(line.semantic_marks && line.semantic_marks.length > 0
          ? { semantic_marks: line.semantic_marks }
          : {}),
        ...(line.wrapped === true ? { wrapped: true } : {}),
      })),
    },
  } satisfies Extract<SubscriptionEvent, { kind: "screen_delta" }>;
}

function attachedScreenText(
  kernel: ReturnType<typeof createWorkspaceKernel>,
): string {
  return (
    kernel
      .getSnapshot()
      .attachedSession?.focused_screen?.surface.lines.map((line) => line.text)
      .join("\n") ?? ""
  );
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

function createCommandHistoryEntry(
  displayText: string,
  lastUsedAtMs: bigint,
): CommandHistoryEntry {
  return {
    id: `history-${displayText}`,
    session_id: "session-1",
    pane_id: "pane-1",
    display_text: displayText,
    last_used_at_ms: lastUsedAtMs,
    use_count: 1n,
  };
}

function createSavedSessionRecord(
  sessionId: string,
  paneId: string,
  line: string,
): SavedSessionRecord {
  const summary = createSavedSessionSummary(sessionId, 1_000n);
  const topology = createLiveTopology(sessionId, paneId);
  const screen = createLiveScreen(paneId, line, 10n);

  return {
    session_id: summary.session_id,
    route: summary.route,
    title: summary.title,
    launch: {
      program: "cmd.exe",
      args: [],
      cwd: null,
    },
    manifest: summary.manifest,
    compatibility: summary.compatibility,
    topology,
    screens: [screen],
    saved_at_ms: summary.saved_at_ms,
    restore_semantics: summary.restore_semantics,
  };
}

function savedSessionRecordToSummary(
  record: SavedSessionRecord,
): SavedSessionSummary {
  return {
    session_id: record.session_id,
    route: record.route,
    title: record.title,
    saved_at_ms: record.saved_at_ms,
    manifest: record.manifest,
    compatibility: record.compatibility,
    has_launch: record.launch !== null,
    tab_count: record.topology.tabs.length,
    pane_count: record.screens.length,
    restore_semantics: record.restore_semantics,
  };
}

function createSavedSessionSummary(
  sessionId: string,
  savedAtMs: bigint,
): SavedSessionSummary {
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
