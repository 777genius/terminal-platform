import type { ReactElement, ReactNode } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { useTerminalActiveSession } from "./hooks/useTerminalActiveSession.js";
import { TerminalActiveSessionSurfaceView } from "./ui/TerminalActiveSessionSurfaceView.js";

export function TerminalActiveSessionSurface(props: {
  runtime: TerminalRuntimeWorkspaceFacade;
  inputPanel?: ReactNode;
}): ReactElement {
  const activeSession = useTerminalActiveSession(props.runtime);
  return (
    <TerminalActiveSessionSurfaceView
      model={activeSession.model}
      commands={activeSession.commands}
      inputPanel={props.inputPanel}
    />
  );
}
