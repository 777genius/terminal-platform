import {
  deriveTerminalRuntimeSessionStreamUrl,
  type TerminalRuntimeBootstrapConfig,
} from "../../contracts/index.js";

interface BootstrapResolution {
  config: TerminalRuntimeBootstrapConfig | null;
  error: string | null;
}

export function resolveTerminalRuntimeBootstrapConfig(): BootstrapResolution {
  const electronConfig = normalizeBootstrapConfig(window.terminalDemo?.config);
  if (electronConfig) {
    return {
      config: electronConfig,
      error: null,
    };
  }

  const params = new URLSearchParams(window.location.search);
  const controlPlaneUrl = params.get("controlPlaneUrl")?.trim();
  const sessionStreamUrl = params.get("sessionStreamUrl")?.trim();
  const legacyGatewayUrl = params.get("gatewayUrl")?.trim();
  const runtimeSlug = params.get("runtimeSlug")?.trim();

  const config = normalizeBootstrapConfig(
    runtimeSlug
      ? {
          controlPlaneUrl,
          sessionStreamUrl,
          gatewayUrl: legacyGatewayUrl,
          runtimeSlug,
        }
      : null,
  );

  if (config) {
    return {
      config,
      error: null,
    };
  }

  return {
    config: null,
    error: "Bootstrap config is missing. Run Electron mode or open the browser URL emitted by the browser host runner.",
  };
}

function normalizeBootstrapConfig(
  raw:
    | {
        controlPlaneUrl?: string | null | undefined;
        sessionStreamUrl?: string | null | undefined;
        runtimeSlug?: string | null | undefined;
        gatewayUrl?: string | null | undefined;
      }
    | null
    | undefined,
): TerminalRuntimeBootstrapConfig | null {
  if (!raw) {
    return null;
  }

  const runtimeSlug = raw.runtimeSlug?.trim();
  const controlPlaneUrl = raw.controlPlaneUrl?.trim() ?? raw.gatewayUrl?.trim() ?? null;
  const sessionStreamUrl = raw.sessionStreamUrl?.trim()
    ?? (controlPlaneUrl ? deriveTerminalRuntimeSessionStreamUrl(controlPlaneUrl) : null);

  if (!runtimeSlug || !controlPlaneUrl || !sessionStreamUrl) {
    return null;
  }

  return {
    controlPlaneUrl,
    sessionStreamUrl,
    runtimeSlug,
  };
}
