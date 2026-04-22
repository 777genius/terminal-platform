import type { TerminalBackendKind, TerminalRouteAuthority } from "../../contracts/terminal-workspace-contracts.js";

export interface TerminalRuntimeExternalSessionRef {
  namespace: string;
  value: string;
}

export interface TerminalRuntimeSessionRoute {
  backend: TerminalBackendKind;
  authority: TerminalRouteAuthority;
  external: TerminalRuntimeExternalSessionRef | null;
}

export interface TerminalRuntimeDiscoveredSession {
  route: TerminalRuntimeSessionRoute;
  title: string | null;
}
