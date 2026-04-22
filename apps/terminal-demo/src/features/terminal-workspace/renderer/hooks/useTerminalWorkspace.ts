import { useEffect, useMemo, useState } from "react";
import type { TerminalDemoBootstrapConfig } from "../../contracts/index.js";
import {
  TerminalWorkspaceController,
  type TerminalWorkspaceStorePort,
} from "../../core/application/index.js";
import {
  getHiddenSavedSessionsCount,
  getVisibleSavedSessions,
} from "../../core/domain/index.js";
import { installTerminalWorkspaceDebug } from "../adapters/debug.js";
import { createTerminalWorkspaceGateway } from "../adapters/createTerminalWorkspaceGateway.js";
import { createTerminalWorkspacePageCommands } from "../commands/createTerminalWorkspacePageCommands.js";
import { createTerminalWorkspacePageModel } from "../presenters/createTerminalWorkspacePageModel.js";
import { useTerminalWorkspaceStore } from "./terminal-workspace.store";

const terminalWorkspaceStorePort: TerminalWorkspaceStorePort = {
  getState: () => useTerminalWorkspaceStore.getState(),
  patch: (patch) => {
    useTerminalWorkspaceStore.setState(patch);
  },
};

export function useTerminalWorkspace(config: TerminalDemoBootstrapConfig) {
  const [showAllSavedSessions, setShowAllSavedSessions] = useState(false);
  const state = useTerminalWorkspaceStore();

  const transport = useMemo(() => {
    return createTerminalWorkspaceGateway({
      controlPlaneUrl: config.controlPlaneUrl,
      sessionStreamUrl: config.sessionStreamUrl,
    });
  }, [config.controlPlaneUrl, config.sessionStreamUrl]);

  const controller = useMemo(() => {
    return new TerminalWorkspaceController(
      transport.controlPlane,
      transport.sessionStatePlane,
      terminalWorkspaceStorePort,
    );
  }, [transport]);

  useEffect(() => {
    useTerminalWorkspaceStore.getState().reset();
    const cleanupDebug = installTerminalWorkspaceDebug({
      controller,
      getState: () => useTerminalWorkspaceStore.getState(),
      setInputDraft: (value) => {
        useTerminalWorkspaceStore.getState().setInputDraft(value);
      },
    });

    void controller.bootstrap();

    return () => {
      cleanupDebug();
      controller.dispose();
      useTerminalWorkspaceStore.getState().reset();
    };
  }, [controller, transport]);

  useEffect(() => {
    setShowAllSavedSessions(false);
  }, [state.savedSessions.length]);

  const visibleSavedSessions = useMemo(
    () => getVisibleSavedSessions(state.savedSessions, showAllSavedSessions),
    [showAllSavedSessions, state.savedSessions],
  );
  const hiddenSavedSessionsCount = useMemo(
    () => getHiddenSavedSessionsCount(state.savedSessions, visibleSavedSessions),
    [state.savedSessions, visibleSavedSessions],
  );
  const discoveredSessionIndex = useMemo(() => {
    const sessions = Object.values(state.discoveredSessions).flatMap((entries) => entries ?? []);
    return new Map(sessions.map((session) => [session.importHandle, session]));
  }, [state.discoveredSessions]);

  const model = createTerminalWorkspacePageModel({
    controlPlaneUrl: transport.controlPlaneUrl,
    sessionStreamUrl: transport.sessionStreamUrl,
    runtimeSlug: config.runtimeSlug,
    status: state.status,
    sessionStatus: state.sessionStatus,
    sessionStreamHealth: state.sessionStreamHealth,
    error: state.error,
    actionError: state.actionError,
    actionDegradedReason: state.actionDegradedReason,
    handshake: state.handshake,
    sessions: state.sessions,
    discoveredSessions: state.discoveredSessions,
    capabilities: state.capabilities,
    activeSessionId: state.activeSessionId,
    activeSessionState: state.activeSessionState,
    createTitleDraft: state.createTitleDraft,
    createProgramDraft: state.createProgramDraft,
    createArgsDraft: state.createArgsDraft,
    createCwdDraft: state.createCwdDraft,
    inputDraft: state.inputDraft,
    visibleSavedSessions,
    hiddenSavedSessionsCount,
    showAllSavedSessions,
  });

  const commands = useMemo(() => createTerminalWorkspacePageCommands({
    controller,
    setCreateField: (field, value) => {
      useTerminalWorkspaceStore.getState().setCreateField(field, value);
    },
    setInputDraft: (value) => {
      useTerminalWorkspaceStore.getState().setInputDraft(value);
    },
    toggleShowAllSavedSessions: () => {
      setShowAllSavedSessions((value) => !value);
    },
    lookupDiscoveredSession: (importHandle) => discoveredSessionIndex.get(importHandle) ?? null,
  }), [controller, discoveredSessionIndex]);

  return {
    model,
    commands,
  };
}
