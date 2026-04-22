import { useEffect, useRef, useState, type ReactElement } from "react";
import type { TerminalDemoBootstrapConfig } from "../contracts/index.js";
import { createWorkspaceWebSocketTransport } from "@terminal-platform/workspace-adapter-websocket";
import { createWorkspaceKernel, type WorkspaceKernel } from "@terminal-platform/workspace-core";
import { defineTerminalPlatformElements } from "@terminal-platform/workspace-elements";

defineTerminalPlatformElements();

type TerminalWorkspaceHostElement = HTMLElement & {
  kernel: WorkspaceKernel | null;
};

export function TerminalWorkspaceApp(props: {
  config: TerminalDemoBootstrapConfig;
}): ReactElement {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const [kernel, setKernel] = useState<WorkspaceKernel | null>(null);
  const [bootstrapError, setBootstrapError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    const transport = createWorkspaceWebSocketTransport({
      controlUrl: props.config.controlPlaneUrl,
      streamUrl: props.config.sessionStreamUrl,
    });
    const nextKernel = createWorkspaceKernel({ transport });

    setBootstrapError(null);
    setKernel(nextKernel);

    void nextKernel.bootstrap().catch(async (error) => {
      if (!active) {
        return;
      }

      setBootstrapError(error instanceof Error ? error.message : String(error));
      await nextKernel.dispose().catch(() => {});
    });

    return () => {
      active = false;
      setKernel((current) => (current === nextKernel ? null : current));
      void nextKernel.dispose().catch(() => {});
    };
  }, [props.config.controlPlaneUrl, props.config.sessionStreamUrl]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) {
      return;
    }

    host.replaceChildren();
    if (!kernel) {
      return;
    }

    const element = document.createElement("tp-terminal-workspace") as TerminalWorkspaceHostElement;
    element.kernel = kernel;
    host.appendChild(element);

    return () => {
      if (element.kernel === kernel) {
        element.kernel = null;
      }
      element.remove();
    };
  }, [kernel]);

  return (
    <main className="shell">
      <section className="panel panel--surface">
        <div className="section__eyebrow">SDK Consumer</div>
        <h1 className="section__title">Terminal Platform Demo</h1>
        <p className="section__copy">
          This renderer now consumes the public SDK surface instead of the feature-local React state machine.
        </p>
        <div className="section__copy">
          Runtime: <strong>{props.config.runtimeSlug}</strong>
        </div>
        {bootstrapError ? (
          <div className="panel panel--error" style={{ marginTop: "1rem" }}>
            <div className="section__eyebrow">Bootstrap Error</div>
            <p className="section__copy">{bootstrapError}</p>
          </div>
        ) : null}
      </section>
      <div ref={hostRef} />
    </main>
  );
}
