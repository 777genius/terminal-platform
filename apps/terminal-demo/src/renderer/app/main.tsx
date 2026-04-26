import "./styles.css";

import { StrictMode, startTransition, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import {
  loadLatestTerminalRuntimeBootstrapConfig,
  resolveTerminalRuntimeBootstrapConfig,
  syncTerminalRuntimeBrowserLocation,
  TerminalRuntimeBootstrapErrorView,
} from "@features/terminal-runtime-host/renderer";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import {
  TerminalDemoWorkspaceApp,
  TerminalDemoWorkspaceScreen,
  createDemoPreviewWorkspaceSnapshot,
  createStaticWorkspaceKernel,
} from "./TerminalDemoWorkspaceApp.js";

declare global {
  interface Window {
    __terminalDemoReactRoot?: Root;
  }
}

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Terminal demo root element was not found.");
}

const root = window.__terminalDemoReactRoot ?? createRoot(rootElement);
window.__terminalDemoReactRoot = root;

root.render(
  <StrictMode>
    <TerminalDemoBootstrapBoundary />
  </StrictMode>,
);

function TerminalDemoBootstrapBoundary() {
  const staticPreview = useMemo(resolveStaticPreviewWorkspace, []);

  if (staticPreview) {
    return <TerminalDemoWorkspaceScreen config={staticPreview.config} kernel={staticPreview.kernel} />;
  }

  return <TerminalDemoRuntimeBootstrapBoundary />;
}

function TerminalDemoRuntimeBootstrapBoundary() {
  const [config, setConfig] = useState<TerminalRuntimeBootstrapConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resolvedOnce, setResolvedOnce] = useState(false);

  useEffect(() => {
    let disposed = false;

    async function refreshBootstrap(initial = false) {
      if (initial) {
        const resolved = await resolveTerminalRuntimeBootstrapConfig();
        if (disposed) {
          return;
        }

        if (resolved.config) {
          syncTerminalRuntimeBrowserLocation(resolved.config);
        }

        startTransition(() => {
          setConfig((current) => (sameBootstrapConfig(current, resolved.config) ? current : resolved.config));
          setError(resolved.error);
          setResolvedOnce(true);
        });
        return;
      }

      const bootstrap = await loadLatestTerminalRuntimeBootstrapConfig();

      if (disposed) {
        return;
      }

      if (bootstrap) {
        syncTerminalRuntimeBrowserLocation(bootstrap);
        startTransition(() => {
          setConfig((current) => (sameBootstrapConfig(current, bootstrap) ? current : bootstrap));
          setError(null);
          setResolvedOnce(true);
        });
        return;
      }
    }

    void refreshBootstrap(true);

    const intervalId = window.setInterval(() => {
      void refreshBootstrap(false);
    }, 2000);

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void refreshBootstrap(false);
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, []);

  const appKey = useMemo(
    () => (config
      ? [
          config.runtimeSlug,
          config.controlPlaneUrl,
          config.sessionStreamUrl,
          config.demoDefaultShellProgram ?? "default-shell",
        ].join("|")
      : "bootstrap"),
    [config],
  );

  if (!resolvedOnce) {
    return (
      <main className="shell shell--error">
        <section className="panel panel--surface panel--error">
          <div className="section__eyebrow">SDK Bootstrap</div>
          <h1 className="section__title">Terminal Platform Demo</h1>
          <p className="section__copy">Resolving runtime host and latest workspace gateway...</p>
        </section>
      </main>
    );
  }

  if (!config) {
    return <TerminalRuntimeBootstrapErrorView error={error ?? "Unknown bootstrap error"} />;
  }

  return <TerminalDemoWorkspaceApp key={appKey} config={config} />;
}

function resolveStaticPreviewWorkspace(): {
  config: TerminalRuntimeBootstrapConfig;
  kernel: ReturnType<typeof createStaticWorkspaceKernel>;
} | null {
  const params = new URLSearchParams(window.location.search);
  if (params.get("demoStaticWorkspace") !== "1") {
    return null;
  }

  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl: "ws://127.0.0.1:0/terminal-gateway/control?token=static-preview",
    demoDefaultShellProgram: resolveStaticPreviewShellProgram(),
    sessionStreamUrl: "ws://127.0.0.1:0/terminal-gateway/stream?token=static-preview",
    runtimeSlug: "terminal-demo-static-preview",
  };

  return {
    config,
    kernel: createStaticWorkspaceKernel(createDemoPreviewWorkspaceSnapshot(config)),
  };
}

function resolveStaticPreviewShellProgram(): string {
  if (typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent)) {
    return "pwsh.exe";
  }

  if (typeof navigator !== "undefined" && /macintosh|mac os x/i.test(navigator.userAgent)) {
    return "zsh";
  }

  return "bash";
}

function sameBootstrapConfig(
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
  );
}
