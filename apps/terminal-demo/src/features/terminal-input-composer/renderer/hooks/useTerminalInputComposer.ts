import { useMemo, useState } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { createTerminalInputComposerCommands } from "../commands/createTerminalInputComposerCommands.js";
import { createTerminalInputComposerModel } from "../presenters/createTerminalInputComposerModel.js";

export function useTerminalInputComposer(runtime: TerminalRuntimeWorkspaceFacade) {
  const [draft, setDraft] = useState("");

  const model = useMemo(() => createTerminalInputComposerModel({
    runtime,
    draft,
  }), [draft, runtime]);

  const commands = useMemo(() => createTerminalInputComposerCommands({
    runtime,
    draft,
    setDraft,
  }), [draft, runtime]);

  return {
    model,
    commands,
  };
}
