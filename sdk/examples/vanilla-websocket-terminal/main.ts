import {
  TERMINAL_PLATFORM_THEME_ATTRIBUTE,
  terminalPlatformDefaultThemeCssText,
} from "@terminal-platform/design-tokens/css";
import { createWorkspaceWebSocketTransport } from "@terminal-platform/workspace-adapter-websocket";
import {
  createWorkspaceHost,
  type WorkspaceHost,
} from "@terminal-platform/workspace-core/bootstrap";
import {
  defineTerminalPlatformElements,
  type TerminalWorkspaceElement,
} from "@terminal-platform/workspace-elements";

const DEFAULT_CONTROL_URL = "ws://127.0.0.1:34115/workspace/control";
const WORKSPACE_SELECTOR = "tp-terminal-workspace";

interface WorkspaceGatewayOptions {
  controlUrl: string;
  streamUrl?: string;
}

type ConnectionState = "connecting" | "ready" | "failed";

const statusElement = requireElement<HTMLElement>("#connection-status");
const workspaceElement = requireElement<TerminalWorkspaceElement>(WORKSPACE_SELECTOR);
let activeHost: WorkspaceHost | null = null;

defineTerminalPlatformElements();
installTheme();

void mountWorkspace()
  .catch((error) => {
    setConnectionStatus("failed", formatStartupError(error));
  });

window.addEventListener("pagehide", () => {
  void disposeWorkspace();
}, { once: true });

async function mountWorkspace(): Promise<void> {
  setConnectionStatus("connecting", "Connecting");

  const host = createProductionWorkspaceHost(readGatewayOptions());
  activeHost = host;
  workspaceElement.kernel = host.kernel;

  try {
    await host.bootstrap();
    setConnectionStatus("ready", "Ready");
  } catch (error) {
    await disposeWorkspace();
    throw error;
  }
}

function createProductionWorkspaceHost(options: WorkspaceGatewayOptions): WorkspaceHost {
  return createWorkspaceHost({
    transport: createWorkspaceWebSocketTransport({
      controlUrl: options.controlUrl,
      ...(options.streamUrl ? { streamUrl: options.streamUrl } : {}),
    }),
  });
}

function readGatewayOptions(search = window.location.search): WorkspaceGatewayOptions {
  const params = new URLSearchParams(search);
  const streamUrl = readOptionalParam(params, "streamUrl");

  return {
    controlUrl: readOptionalParam(params, "controlUrl") ?? DEFAULT_CONTROL_URL,
    ...(streamUrl ? { streamUrl } : {}),
  };
}

function readOptionalParam(params: URLSearchParams, name: string): string | null {
  const value = params.get(name)?.trim();
  return value && value.length > 0 ? value : null;
}

async function disposeWorkspace(): Promise<void> {
  const host = activeHost;
  activeHost = null;
  workspaceElement.kernel = null;
  await host?.dispose();
}

function installTheme(): void {
  document.documentElement.setAttribute(TERMINAL_PLATFORM_THEME_ATTRIBUTE, "terminal-platform-default");

  if (document.querySelector("style[data-terminal-platform-theme]")) {
    return;
  }

  const style = document.createElement("style");
  style.dataset.terminalPlatformTheme = "true";
  style.textContent = terminalPlatformDefaultThemeCssText;
  document.head.append(style);
}

function requireElement<ElementType extends Element>(selector: string): ElementType {
  const element = document.querySelector<ElementType>(selector);

  if (!element) {
    throw new Error(`missing required element: ${selector}`);
  }

  return element;
}

function setConnectionStatus(state: ConnectionState, label: string): void {
  statusElement.dataset.state = state;
  statusElement.textContent = label;
}

function formatStartupError(error: unknown): string {
  return error instanceof Error ? error.message : "Failed to start workspace";
}
