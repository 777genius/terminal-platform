export interface TerminalRuntimeBootstrapConfig {
  controlPlaneUrl: string;
  demoAutoStartSession?: boolean;
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
  if (config.demoAutoStartSession) {
    url.searchParams.set("demoAutoStartSession", "1");
  } else {
    url.searchParams.delete("demoAutoStartSession");
  }
  return url.toString();
}

export function deriveTerminalRuntimeSessionStreamUrl(controlPlaneUrl: string): string {
  const url = new URL(controlPlaneUrl);
  if (url.pathname === "/terminal-gateway" || url.pathname === "/terminal-gateway/control") {
    url.pathname = "/terminal-gateway/stream";
    return url.toString();
  }

  throw new Error(`Unsupported terminal gateway URL path: ${url.pathname}`);
}
