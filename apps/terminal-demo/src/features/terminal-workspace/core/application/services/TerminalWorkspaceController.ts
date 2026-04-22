import type {
  TerminalBackendCapabilitiesInfo,
  TerminalBackendKind,
  TerminalDegradedReason,
  TerminalDiscoveredSession,
  TerminalHandshakeInfo,
  TerminalImportSessionInput,
  TerminalMuxCommand,
  TerminalSavedSessionSummary,
  TerminalSessionSummary,
} from "../../../contracts/terminal-workspace-contracts.js";
import {
  buildCreateNativeSessionPayload,
  findRestoreBlockedReason,
  findUnsupportedActionDegradedReason,
  focusedPaneId,
  type TerminalWorkspaceActionKind,
} from "../../domain/index.js";
import type { TerminalWorkspaceControlGatewayPort } from "../ports/TerminalWorkspaceControlGatewayPort.js";
import type { TerminalWorkspaceSessionStateStreamPort } from "../ports/TerminalWorkspaceSessionStateStreamPort.js";
import type { TerminalWorkspaceStorePort } from "../ports/TerminalWorkspaceStorePort.js";
import type { TerminalWorkspaceSessionStreamHealth } from "../TerminalWorkspaceSessionStreamHealth.js";

const foreignBackends: TerminalBackendKind[] = ["tmux", "zellij"];

interface TerminalWorkspaceCatalog {
  sessions: TerminalSessionSummary[];
  savedSessions: TerminalSavedSessionSummary[];
  capabilities: Partial<Record<TerminalBackendKind, TerminalBackendCapabilitiesInfo>>;
  discoveredSessions: Partial<Record<TerminalBackendKind, TerminalDiscoveredSession[]>>;
}

export class TerminalWorkspaceController {
  readonly #controlPlane: TerminalWorkspaceControlGatewayPort;
  readonly #sessionStatePlane: TerminalWorkspaceSessionStateStreamPort;
  readonly #store: TerminalWorkspaceStorePort;
  #subscription: Awaited<ReturnType<TerminalWorkspaceSessionStateStreamPort["subscribeSessionState"]>> | null = null;

  constructor(
    controlPlane: TerminalWorkspaceControlGatewayPort,
    sessionStatePlane: TerminalWorkspaceSessionStateStreamPort,
    store: TerminalWorkspaceStorePort,
  ) {
    this.#controlPlane = controlPlane;
    this.#sessionStatePlane = sessionStatePlane;
    this.#store = store;
  }

  async bootstrap(): Promise<void> {
    this.#store.patch({
      status: "loading",
      error: null,
      actionError: null,
      actionDegradedReason: null,
      sessionStreamHealth: {
        phase: "idle",
        reconnectAttempts: 0,
        lastError: null,
      },
    });

    try {
      const handshake = await this.#controlPlane.handshakeInfo();
      const catalog = await this.loadCatalog(handshake);
      const nextActiveSessionId =
        this.#store.getState().activeSessionId ?? catalog.sessions[0]?.session_id ?? null;

      this.#store.patch({
        status: "ready",
        sessionStatus: nextActiveSessionId ? "connecting" : "idle",
        handshake,
        sessions: catalog.sessions,
        savedSessions: catalog.savedSessions,
        capabilities: catalog.capabilities,
        discoveredSessions: catalog.discoveredSessions,
        activeSessionId: nextActiveSessionId,
        error: null,
      });

      if (nextActiveSessionId) {
        await this.selectSession(nextActiveSessionId);
      }
    } catch (error) {
      this.#store.patch({
        status: "error",
        error: toMessage(error),
      });
    }
  }

  async refreshCatalog(): Promise<void> {
    const handshake = this.#store.getState().handshake;
    if (!handshake) {
      return this.bootstrap();
    }

    await this.runAction(async () => {
      const catalog = await this.loadCatalog(handshake);
      const currentActiveSessionId = this.#store.getState().activeSessionId;
      const sessionStillExists = catalog.sessions.some(
        (session) => session.session_id === currentActiveSessionId,
      );

      this.#store.patch({
        sessions: catalog.sessions,
        savedSessions: catalog.savedSessions,
        capabilities: catalog.capabilities,
        discoveredSessions: catalog.discoveredSessions,
        activeSessionId: sessionStillExists
          ? currentActiveSessionId
          : catalog.sessions[0]?.session_id ?? null,
      });
    });
  }

  async selectSession(sessionId: string): Promise<void> {
    if (!sessionId) {
      return;
    }

    await this.#subscription?.dispose();
    this.#subscription = null;

    this.#store.patch({
      activeSessionId: sessionId,
      activeSessionState: null,
      sessionStatus: "connecting",
      actionError: null,
      actionDegradedReason: null,
      sessionStreamHealth: {
        phase: "connecting",
        reconnectAttempts: 0,
        lastError: null,
      },
    });

    try {
      this.#subscription = await this.#sessionStatePlane.subscribeSessionState(sessionId, {
        onState: (state) => {
          if (this.#store.getState().activeSessionId !== sessionId) {
            return;
          }

          this.#store.patch({
            sessionStatus: "ready",
            activeSessionState: state,
            sessionStreamHealth: {
              phase: "ready",
              reconnectAttempts: 0,
              lastError: null,
            },
          });
        },
        onStatusChange: (health) => {
          if (this.#store.getState().activeSessionId !== sessionId) {
            return;
          }

          this.patchSessionStreamHealth(health);
        },
        onError: (error) => {
          if (this.#store.getState().activeSessionId !== sessionId) {
            return;
          }

          this.#store.patch({
            sessionStatus: "error",
            sessionStreamHealth: {
              phase: "error",
              reconnectAttempts: 0,
              lastError: toMessage(error),
            },
          });
        },
        onClosed: () => {
          if (this.#store.getState().activeSessionId === sessionId) {
            this.#store.patch({
              sessionStatus: "idle",
              sessionStreamHealth: {
                phase: "idle",
                reconnectAttempts: 0,
                lastError: null,
              },
            });
          }
        },
      });
    } catch (error) {
      this.#store.patch({
        sessionStatus: "error",
        sessionStreamHealth: {
          phase: "error",
          reconnectAttempts: 0,
          lastError: toMessage(error),
        },
      });
    }
  }

  async createNativeSession(): Promise<void> {
    await this.runAction(async () => {
      const state = this.#store.getState();
      const created = await this.#controlPlane.createNativeSession(
        buildCreateNativeSessionPayload({
          title: state.createTitleDraft,
          program: state.createProgramDraft,
          args: state.createArgsDraft,
          cwd: state.createCwdDraft,
        }),
      );

      await this.refreshCatalog();
      await this.selectSession(created.session_id);
    });
  }

  async importSession(input: TerminalImportSessionInput): Promise<void> {
    await this.runAction(async () => {
      const imported = await this.#controlPlane.importSession(input);

      await this.refreshCatalog();
      await this.selectSession(imported.session_id);
    });
  }

  async restoreSavedSession(sessionId: string): Promise<void> {
    const savedSession = this.#store.getState().savedSessions.find((entry) => entry.session_id === sessionId) ?? null;
    const blockedReason = findRestoreBlockedReason(savedSession);
    if (blockedReason) {
      this.patchActionDegradedReason(blockedReason);
      return;
    }

    await this.runAction(async () => {
      const restored = await this.#controlPlane.restoreSavedSession(sessionId);
      await this.refreshCatalog();
      await this.selectSession(restored.session_id);
    });
  }

  async deleteSavedSession(sessionId: string): Promise<void> {
    await this.runAction(async () => {
      await this.#controlPlane.deleteSavedSession(sessionId);
      await this.refreshCatalog();
    });
  }

  async focusPane(paneId: string): Promise<void> {
    if (!this.ensureActionSupported("focus_pane")) {
      return;
    }

    await this.dispatch({ kind: "focus_pane", pane_id: paneId });
  }

  async focusTab(tabId: string): Promise<void> {
    if (!this.ensureActionSupported("focus_tab")) {
      return;
    }

    await this.dispatch({ kind: "focus_tab", tab_id: tabId });
  }

  async splitFocusedPane(direction: "horizontal" | "vertical"): Promise<void> {
    if (!this.ensureActionSupported("split_pane")) {
      return;
    }

    const paneId = focusedPaneId(this.#store.getState().activeSessionState);
    if (!paneId) {
      return;
    }

    await this.dispatch({ kind: "split_pane", pane_id: paneId, direction });
  }

  async newTab(): Promise<void> {
    if (!this.ensureActionSupported("new_tab")) {
      return;
    }

    await this.dispatch({ kind: "new_tab", title: null });
  }

  async saveSession(): Promise<void> {
    if (!this.ensureActionSupported("save_session")) {
      return;
    }

    if (await this.dispatch({ kind: "save_session" })) {
      await this.refreshCatalog();
    }
  }

  async sendShortcut(data: string): Promise<void> {
    await this.sendInput(data);
  }

  async submitInput(): Promise<void> {
    const payload = this.#store.getState().inputDraft;
    if (!payload.trim()) {
      return;
    }

    await this.runAction(async () => {
      await this.sendInput(payload);
      await this.sendInput("\r");
      this.#store.patch({ inputDraft: "" });
    });
  }

  async sendInput(data: string): Promise<void> {
    if (!this.ensureActionSupported("send_input")) {
      return;
    }

    const paneId = focusedPaneId(this.#store.getState().activeSessionState);
    if (!paneId) {
      return;
    }

    await this.dispatch({
      kind: "send_input",
      pane_id: paneId,
      data,
    });
  }

  dispose(): void {
    void this.#subscription?.dispose();
    this.#sessionStatePlane.dispose();
    this.#controlPlane.dispose();
  }

  private async dispatch(command: TerminalMuxCommand): Promise<boolean> {
    const activeSessionId = this.#store.getState().activeSessionId;
    if (!activeSessionId) {
      return false;
    }

    return this.runAction(async () => {
      await this.#controlPlane.dispatchMuxCommand(activeSessionId, command);
    });
  }

  private async loadCatalog(handshake: TerminalHandshakeInfo): Promise<TerminalWorkspaceCatalog> {
    const [sessions, savedSessions, capabilities, discoveredSessions] = await Promise.all([
      this.#controlPlane.listSessions(),
      this.#controlPlane.listSavedSessions(),
      this.loadCapabilities(handshake.handshake.available_backends),
      this.loadDiscoveredSessions(handshake.handshake.available_backends),
    ]);

    return {
      sessions,
      savedSessions,
      capabilities,
      discoveredSessions,
    };
  }

  private async loadCapabilities(availableBackends: TerminalBackendKind[]) {
    const entries = await Promise.all(
      availableBackends.map(async (backend) => {
        const info = await this.#controlPlane.backendCapabilities(backend);
        return [backend, info] as const;
      }),
    );

    return Object.fromEntries(entries) as Partial<
      Record<TerminalBackendKind, TerminalBackendCapabilitiesInfo>
    >;
  }

  private async loadDiscoveredSessions(availableBackends: TerminalBackendKind[]) {
    const entries = await Promise.all(
      foreignBackends
        .filter((backend) => availableBackends.includes(backend))
        .map(async (backend) => {
          const sessions = await this.#controlPlane.discoverSessions(backend);
          return [backend, sessions] as const;
        }),
    );

    return Object.fromEntries(entries) as Partial<
      Record<TerminalBackendKind, TerminalDiscoveredSession[]>
    >;
  }

  private async runAction(work: () => Promise<void>): Promise<boolean> {
    this.#store.patch({
      actionError: null,
      actionDegradedReason: null,
    });

    try {
      await work();
      return true;
    } catch (error) {
      this.#store.patch({
        actionError: toMessage(error),
      });
      return false;
    }
  }

  private ensureActionSupported(action: TerminalWorkspaceActionKind): boolean {
    const state = this.#store.getState();
    const activeSession = state.sessions.find((session) => session.session_id === state.activeSessionId) ?? null;
    if (!activeSession) {
      return false;
    }

    const capabilityInfo = state.capabilities[activeSession.origin.backend] ?? null;
    const degradedReason = findUnsupportedActionDegradedReason({
      action,
      backend: activeSession.origin.backend,
      capabilities: capabilityInfo?.capabilities,
    });

    if (!degradedReason) {
      return true;
    }

    this.patchActionDegradedReason(degradedReason);
    return false;
  }

  private patchActionDegradedReason(reason: TerminalDegradedReason): void {
    this.#store.patch({
      actionError: null,
      actionDegradedReason: reason,
    });
  }

  private patchSessionStreamHealth(health: TerminalWorkspaceSessionStreamHealth): void {
    const nextPatch: Partial<ReturnType<TerminalWorkspaceStorePort["getState"]>> = {
      sessionStreamHealth: health,
    };

    if (health.phase === "connecting") {
      nextPatch.sessionStatus = "connecting";
    }

    if (health.phase === "error") {
      nextPatch.sessionStatus = "error";
    }

    this.#store.patch(nextPatch);
  }
}

function toMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
