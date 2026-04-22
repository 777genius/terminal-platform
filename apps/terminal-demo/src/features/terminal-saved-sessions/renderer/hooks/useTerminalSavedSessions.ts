import { useEffect, useMemo, useState } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { createTerminalSavedSessionsCommands } from "../commands/createTerminalSavedSessionsCommands.js";
import { createTerminalSavedSessionsModel } from "../presenters/createTerminalSavedSessionsModel.js";

export function useTerminalSavedSessions(runtime: TerminalRuntimeWorkspaceFacade) {
  const [showAll, setShowAll] = useState(false);

  useEffect(() => {
    setShowAll(false);
  }, [runtime.state.savedSessions.length]);

  const model = useMemo(() => createTerminalSavedSessionsModel({
    runtime,
    showAll,
  }), [runtime, showAll]);

  const commands = useMemo(() => createTerminalSavedSessionsCommands({
    runtime,
    toggleVisibility: () => {
      setShowAll((current) => !current);
    },
  }), [runtime]);

  return {
    model,
    commands,
  };
}
