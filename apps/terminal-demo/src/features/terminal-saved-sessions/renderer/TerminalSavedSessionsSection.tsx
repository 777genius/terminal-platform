import type { ReactElement } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { useTerminalSavedSessions } from "./hooks/useTerminalSavedSessions.js";
import { TerminalSavedSessionsSectionView } from "./ui/TerminalSavedSessionsSectionView.js";

export function TerminalSavedSessionsSection(props: {
  runtime: TerminalRuntimeWorkspaceFacade;
}): ReactElement {
  const savedSessions = useTerminalSavedSessions(props.runtime);
  return <TerminalSavedSessionsSectionView model={savedSessions.model} commands={savedSessions.commands} />;
}
