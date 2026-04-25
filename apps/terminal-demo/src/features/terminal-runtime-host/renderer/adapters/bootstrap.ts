import {
  buildTerminalRuntimeBrowserUrl,
  deriveTerminalRuntimeSessionStreamUrl,
  TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH,
  type TerminalRuntimeBootstrapConfig,
} from "../../contracts/index.js";

interface BootstrapResolution {
  config: TerminalRuntimeBootstrapConfig | null;
  error: string | null;
}

export async function resolveTerminalRuntimeBootstrapConfig(): Promise<BootstrapResolution> {
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
  const demoAutoStartSession = params.get("demoAutoStartSession")?.trim();

  const queryConfig = normalizeBootstrapConfig(
    runtimeSlug
      ? {
          controlPlaneUrl,
          demoAutoStartSession,
          sessionStreamUrl,
          gatewayUrl: legacyGatewayUrl,
          runtimeSlug,
        }
      : null,
  );

  const browserConfig = await loadBrowserBootstrapConfig();
  const preferredConfig = selectPreferredBootstrapConfig({
    browserConfig,
    queryConfig,
  });

  if (preferredConfig) {
    return {
      config: preferredConfig,
      error: null,
    };
  }

  return {
    config: null,
    error: "Bootstrap config is missing. Run Electron mode or open the browser URL emitted by the browser host runner.",
  };
}

export async function loadLatestTerminalRuntimeBootstrapConfig(): Promise<TerminalRuntimeBootstrapConfig | null> {
  const electronConfig = normalizeBootstrapConfig(window.terminalDemo?.config);
  if (electronConfig) {
    return electronConfig;
  }

  const params = new URLSearchParams(window.location.search);
  const queryConfig = normalizeBootstrapConfig({
    controlPlaneUrl: params.get("controlPlaneUrl")?.trim(),
    demoAutoStartSession: params.get("demoAutoStartSession")?.trim(),
    sessionStreamUrl: params.get("sessionStreamUrl")?.trim(),
    gatewayUrl: params.get("gatewayUrl")?.trim(),
    runtimeSlug: params.get("runtimeSlug")?.trim(),
  });

  const browserConfig = await loadBrowserBootstrapConfig();
  return selectPreferredBootstrapConfig({
    browserConfig,
    queryConfig,
  });
}

export function syncTerminalRuntimeBrowserLocation(config: TerminalRuntimeBootstrapConfig): void {
  if (window.terminalDemo?.config) {
    return;
  }

  const nextUrl = buildTerminalRuntimeBrowserUrl(window.location.href, config);
  if (nextUrl !== window.location.href) {
    window.history.replaceState(null, "", nextUrl);
  }
}

async function loadBrowserBootstrapConfig(): Promise<TerminalRuntimeBootstrapConfig | null> {
  try {
    const response = await fetch(TERMINAL_RUNTIME_BROWSER_BOOTSTRAP_PATH, {
      cache: "no-store",
    });
    if (!response.ok) {
      return null;
    }

    const payload = await response.json();
    return normalizeBootstrapConfig(payload);
  } catch {
    return null;
  }
}

function selectPreferredBootstrapConfig(input: {
  browserConfig: TerminalRuntimeBootstrapConfig | null;
  queryConfig: TerminalRuntimeBootstrapConfig | null;
}): TerminalRuntimeBootstrapConfig | null {
  if (input.browserConfig) {
    if (!input.queryConfig || input.queryConfig.runtimeSlug === input.browserConfig.runtimeSlug) {
      return input.browserConfig;
    }
  }

  return input.queryConfig;
}

function normalizeBootstrapConfig(
  raw:
    | {
        controlPlaneUrl?: string | null | undefined;
        demoAutoStartSession?: boolean | string | null | undefined;
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

  const runtimeSlug = normalizeBootstrapScalar(raw.runtimeSlug);
  const controlPlaneUrl = normalizeBootstrapScalar(raw.controlPlaneUrl) ?? normalizeBootstrapScalar(raw.gatewayUrl);
  const sessionStreamUrl = normalizeBootstrapScalar(raw.sessionStreamUrl)
    ?? (controlPlaneUrl ? deriveTerminalRuntimeSessionStreamUrl(controlPlaneUrl) : null);

  if (!runtimeSlug || !controlPlaneUrl || !sessionStreamUrl) {
    return null;
  }

  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl,
    sessionStreamUrl,
    runtimeSlug,
  };
  const demoAutoStartSession = normalizeBootstrapBoolean(raw.demoAutoStartSession);
  if (demoAutoStartSession !== null) {
    config.demoAutoStartSession = demoAutoStartSession;
  }

  return config;
}

function normalizeBootstrapScalar(value: string | null | undefined): string | null {
  const normalized = value?.trim();
  if (!normalized) {
    return null;
  }

  if (normalized === "undefined" || normalized === "null") {
    return null;
  }

  return normalized;
}

function normalizeBootstrapBoolean(value: boolean | string | null | undefined): boolean | null {
  if (typeof value === "boolean") {
    return value;
  }

  const normalized = normalizeBootstrapScalar(value);
  if (!normalized) {
    return null;
  }

  return ["1", "true", "yes"].includes(normalized.toLowerCase());
}
