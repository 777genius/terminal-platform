import type { ReactElement } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { useTerminalInputComposer } from "./hooks/useTerminalInputComposer.js";
import { TerminalInputComposerPanelView } from "./ui/TerminalInputComposerPanelView.js";

export function TerminalInputComposerPanel(props: {
  runtime: TerminalRuntimeWorkspaceFacade;
}): ReactElement {
  const inputComposer = useTerminalInputComposer(props.runtime);
  return <TerminalInputComposerPanelView model={inputComposer.model} commands={inputComposer.commands} />;
}
