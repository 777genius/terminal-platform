import { useMemo, useState } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import {
  initialTerminalWorkspaceCatalogFormState,
  type TerminalWorkspaceCatalogFormState,
} from "../../core/application/index.js";
import { createTerminalWorkspaceCatalogCommands } from "../commands/createTerminalWorkspaceCatalogCommands.js";
import { createTerminalWorkspaceCatalogModel } from "../presenters/createTerminalWorkspaceCatalogModel.js";

export function useTerminalWorkspaceCatalog(runtime: TerminalRuntimeWorkspaceFacade) {
  const [form, setFormState] = useState<TerminalWorkspaceCatalogFormState>(initialTerminalWorkspaceCatalogFormState);

  const discoveredSessionIndex = useMemo(() => {
    const sessions = Object.values(runtime.state.discoveredSessions).flatMap((entries) => entries ?? []);
    return new Map(sessions.map((session) => [session.importHandle, session]));
  }, [runtime.state.discoveredSessions]);

  const model = useMemo(() => createTerminalWorkspaceCatalogModel({
    runtime,
    form,
  }), [form, runtime]);

  const commands = useMemo(() => createTerminalWorkspaceCatalogCommands({
    runtime,
    form,
    setForm: (patch) => {
      setFormState((current) => ({
        ...current,
        ...patch,
      }));
    },
    lookupDiscoveredSession: (importHandle) => discoveredSessionIndex.get(importHandle) ?? null,
  }), [discoveredSessionIndex, form, runtime]);

  return {
    model,
    commands,
  };
}
