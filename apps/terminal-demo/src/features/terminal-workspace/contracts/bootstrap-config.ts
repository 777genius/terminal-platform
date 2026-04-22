export interface TerminalDemoBootstrapConfig {
  controlPlaneUrl: string;
  sessionStreamUrl: string;
  runtimeSlug: string;
}

export function buildTerminalDemoBrowserUrl(
  rendererUrl: string,
  config: TerminalDemoBootstrapConfig,
): string {
  const url = new URL(rendererUrl);
  url.searchParams.set("controlPlaneUrl", config.controlPlaneUrl);
  url.searchParams.set("sessionStreamUrl", config.sessionStreamUrl);
  url.searchParams.set("runtimeSlug", config.runtimeSlug);
  return url.toString();
}

export function deriveTerminalDemoSessionStreamUrl(controlPlaneUrl: string): string {
  const url = new URL(controlPlaneUrl);
  if (url.pathname === "/terminal-gateway" || url.pathname === "/terminal-gateway/control") {
    url.pathname = "/terminal-gateway/stream";
    return url.toString();
  }

  throw new Error(`Unsupported terminal gateway URL path: ${url.pathname}`);
}
