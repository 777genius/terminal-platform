import "./styles.css";

import { StrictMode, startTransition, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import {
  loadLatestTerminalRuntimeBootstrapConfig,
  resolveTerminalRuntimeBootstrapConfig,
  syncTerminalRuntimeBrowserLocation,
  TerminalRuntimeBootstrapErrorView,
} from "@features/terminal-runtime-host/renderer";
import {
  sameTerminalRuntimeBootstrapConfig,
  type TerminalRuntimeBootstrapConfig,
} from "@features/terminal-runtime-host/contracts";
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
  const mountedRef = useRef(false);
  const refreshGenerationRef = useRef(0);
  const lastConnectionIssueRefreshAtRef = useRef(0);

  const refreshBootstrap = useCallback(async (initial = false, generation = refreshGenerationRef.current) => {
    if (initial) {
      const resolved = await resolveTerminalRuntimeBootstrapConfig();
      if (!mountedRef.current || generation !== refreshGenerationRef.current) {
        return;
      }

      if (resolved.config) {
        syncTerminalRuntimeBrowserLocation(resolved.config);
      }

      startTransition(() => {
        setConfig((current) => (
          sameTerminalRuntimeBootstrapConfig(current, resolved.config) ? current : resolved.config
        ));
        setError(resolved.error);
        setResolvedOnce(true);
      });
      return;
    }

    const bootstrap = await loadLatestTerminalRuntimeBootstrapConfig();

    if (!mountedRef.current || generation !== refreshGenerationRef.current || !bootstrap) {
      return;
    }

    syncTerminalRuntimeBrowserLocation(bootstrap);
    startTransition(() => {
      setConfig((current) => (
        sameTerminalRuntimeBootstrapConfig(current, bootstrap) ? current : bootstrap
      ));
      setError(null);
      setResolvedOnce(true);
    });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    const generation = refreshGenerationRef.current + 1;
    refreshGenerationRef.current = generation;
    void refreshBootstrap(true, generation);

    const intervalId = window.setInterval(() => {
      void refreshBootstrap(false, generation);
    }, 2000);

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void refreshBootstrap(false, generation);
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      mountedRef.current = false;
      refreshGenerationRef.current += 1;
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [refreshBootstrap]);

  const handleRuntimeConnectionIssue = useCallback(() => {
    const now = Date.now();
    if (now - lastConnectionIssueRefreshAtRef.current < 1_000) {
      return;
    }

    lastConnectionIssueRefreshAtRef.current = now;
    void refreshBootstrap(false, refreshGenerationRef.current);
  }, [refreshBootstrap]);

  const appKey = useMemo(
    () => (config
      ? [
          config.runtimeSlug,
          config.controlPlaneUrl,
          config.sessionStreamUrl,
          config.demoDefaultShellProgram ?? "default-shell",
          config.demoDefaultWorkingDirectory ?? "default-cwd",
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

  return (
    <TerminalDemoWorkspaceApp
      key={appKey}
      config={config}
      onRuntimeConnectionIssue={handleRuntimeConnectionIssue}
    />
  );
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
    return "cmd.exe";
  }

  if (typeof navigator !== "undefined" && /macintosh|mac os x/i.test(navigator.userAgent)) {
    return "zsh";
  }

  return "bash";
}
