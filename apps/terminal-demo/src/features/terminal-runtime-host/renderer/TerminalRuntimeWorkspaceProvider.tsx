import type { PropsWithChildren, ReactElement } from "react";
import { useEffect, useMemo, useRef } from "react";
import type {
  TerminalRuntimeBootstrapConfig,
} from "../contracts/index.js";
import type {
  TerminalRuntimeWorkspaceCommands,
  TerminalRuntimeWorkspaceFacade,
} from "@features/terminal-workspace-kernel/contracts";
import {
  TerminalRuntimeWorkspaceController,
  type TerminalWorkspaceStorePort,
} from "../core/application/index.js";
import { installTerminalRuntimeDebug } from "./adapters/debug.js";
import { createTerminalRuntimeGateway } from "./adapters/createTerminalRuntimeGateway.js";
import { useTerminalRuntimeWorkspaceStore } from "./hooks/runtime-workspace.store.js";
import { TerminalRuntimeWorkspaceContext } from "./useTerminalRuntimeWorkspace.js";

const terminalRuntimeStorePort: TerminalWorkspaceStorePort = {
  getState: () => useTerminalRuntimeWorkspaceStore.getState(),
  patch: (patch) => {
    useTerminalRuntimeWorkspaceStore.setState(patch);
  },
};

export function TerminalRuntimeWorkspaceProvider(
  props: PropsWithChildren<{
    config: TerminalRuntimeBootstrapConfig;
  }>,
): ReactElement {
  const state = useTerminalRuntimeWorkspaceStore();
  const controllerRef = useRef<TerminalRuntimeWorkspaceController | null>(null);

  const runWithController = <T,>(work: (controller: TerminalRuntimeWorkspaceController) => Promise<T>): Promise<T> => {
    const controller = controllerRef.current;
    if (!controller) {
      return Promise.reject(new Error("Terminal runtime workspace controller is not ready"));
    }

    return work(controller);
  };

  useEffect(() => {
    const transport = createTerminalRuntimeGateway({
      controlPlaneUrl: props.config.controlPlaneUrl,
      sessionStreamUrl: props.config.sessionStreamUrl,
    });
    const controller = new TerminalRuntimeWorkspaceController(
      transport.controlPlane,
      transport.sessionStatePlane,
      terminalRuntimeStorePort,
    );
    controllerRef.current = controller;
    useTerminalRuntimeWorkspaceStore.getState().reset();
    const cleanupDebug = installTerminalRuntimeDebug({
      controller,
      getState: () => useTerminalRuntimeWorkspaceStore.getState(),
    });

    void controller.bootstrap();

    return () => {
      if (controllerRef.current === controller) {
        controllerRef.current = null;
      }
      cleanupDebug();
      controller.dispose();
      useTerminalRuntimeWorkspaceStore.getState().reset();
      transport.dispose();
    };
  }, [props.config.controlPlaneUrl, props.config.sessionStreamUrl]);

  const commands = useMemo<TerminalRuntimeWorkspaceCommands>(() => ({
    refreshCatalog: () => runWithController((controller) => controller.refreshCatalog()),
    selectSession: (sessionId) => runWithController((controller) => controller.selectSession(sessionId)),
    createNativeSession: (input) => runWithController((controller) => controller.createNativeSession(input)),
    importSession: (input) => runWithController((controller) => controller.importSession(input)),
    restoreSavedSession: (sessionId) => runWithController((controller) => controller.restoreSavedSession(sessionId)),
    deleteSavedSession: (sessionId) => runWithController((controller) => controller.deleteSavedSession(sessionId)),
    focusPane: (paneId) => runWithController((controller) => controller.focusPane(paneId)),
    focusTab: (tabId) => runWithController((controller) => controller.focusTab(tabId)),
    splitFocusedPane: (direction) => runWithController((controller) => controller.splitFocusedPane(direction)),
    newTab: () => runWithController((controller) => controller.newTab()),
    saveSession: () => runWithController((controller) => controller.saveSession()),
    submitInput: (input) => runWithController((controller) => controller.submitInput(input)),
    sendShortcut: (data) => runWithController((controller) => controller.sendShortcut(data)),
  }), []);

  const facade = useMemo<TerminalRuntimeWorkspaceFacade>(() => ({
    transport: {
      controlPlaneUrl: props.config.controlPlaneUrl,
      sessionStreamUrl: props.config.sessionStreamUrl,
      runtimeSlug: props.config.runtimeSlug,
    },
    state,
    commands,
  }), [commands, props.config.controlPlaneUrl, props.config.runtimeSlug, props.config.sessionStreamUrl, state]);

  return (
    <TerminalRuntimeWorkspaceContext.Provider value={facade}>
      {props.children}
    </TerminalRuntimeWorkspaceContext.Provider>
  );
}
