export interface TerminalRuntimeBootstrapConfig {
  controlPlaneUrl: string;
  demoDefaultWorkingDirectory?: string;
  demoDefaultShellProgram?: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
}

export const TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH = "/terminal-runtime-bootstrap.json";

export function buildTerminalRuntimeBrowserUrl(
  rendererUrl: string,
  config: TerminalRuntimeBootstrapConfig,
): string {
  const url = new URL(rendererUrl);
  url.searchParams.set("controlPlaneUrl", config.controlPlaneUrl);
  url.searchParams.set("sessionStreamUrl", config.sessionStreamUrl);
  url.searchParams.set("runtimeSlug", config.runtimeSlug);
  if (config.demoDefaultShellProgram) {
    url.searchParams.set("demoDefaultShellProgram", config.demoDefaultShellProgram);
  } else {
    url.searchParams.delete("demoDefaultShellProgram");
  }
  if (config.demoDefaultWorkingDirectory) {
    url.searchParams.set("demoDefaultWorkingDirectory", config.demoDefaultWorkingDirectory);
  } else {
    url.searchParams.delete("demoDefaultWorkingDirectory");
  }
  return url.toString();
}

export function sameTerminalRuntimeBootstrapConfig(
  left: TerminalRuntimeBootstrapConfig | null,
  right: TerminalRuntimeBootstrapConfig | null,
): boolean {
  if (!left || !right) {
    return left === right;
  }

  return (
    left.controlPlaneUrl === right.controlPlaneUrl
    && left.sessionStreamUrl === right.sessionStreamUrl
    && left.runtimeSlug === right.runtimeSlug
    && left.demoDefaultShellProgram === right.demoDefaultShellProgram
    && left.demoDefaultWorkingDirectory === right.demoDefaultWorkingDirectory
  );
}

export function deriveTerminalRuntimeSessionStreamUrl(controlPlaneUrl: string): string {
  const url = new URL(controlPlaneUrl);
  if (url.pathname === "/terminal-gateway" || url.pathname === "/terminal-gateway/control") {
    url.pathname = "/terminal-gateway/stream";
    return url.toString();
  }

  throw new Error(`Unsupported terminal gateway URL path: ${url.pathname}`);
}
