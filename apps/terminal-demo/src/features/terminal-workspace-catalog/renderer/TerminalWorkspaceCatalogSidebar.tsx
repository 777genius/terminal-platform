import type { ReactElement } from "react";
import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import { useTerminalWorkspaceCatalog } from "./hooks/useTerminalWorkspaceCatalog.js";
import { TerminalWorkspaceCatalogSidebarView } from "./ui/TerminalWorkspaceCatalogSidebarView.js";

export function TerminalWorkspaceCatalogSidebar(props: {
  runtime: TerminalRuntimeWorkspaceFacade;
}): ReactElement {
  const catalog = useTerminalWorkspaceCatalog(props.runtime);
  return <TerminalWorkspaceCatalogSidebarView model={catalog.model} commands={catalog.commands} />;
}
