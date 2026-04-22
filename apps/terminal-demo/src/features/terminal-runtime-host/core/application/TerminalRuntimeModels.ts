import type { TerminalBackendKind, TerminalRouteAuthority } from "@features/terminal-workspace-kernel/contracts";

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
