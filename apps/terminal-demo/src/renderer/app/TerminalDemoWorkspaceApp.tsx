import { useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import { TerminalRuntimeBootstrapErrorView } from "@features/terminal-runtime-host/renderer";
import type { CreateSessionRequest, SessionId } from "@terminal-platform/runtime-types";
import { createWorkspaceWebSocketTransport } from "@terminal-platform/workspace-adapter-websocket";
import {
  createWorkspaceKernel,
  type WorkspaceCommands,
  type WorkspaceDiagnostics,
  type WorkspaceKernel,
  type WorkspaceSelectors,
  type WorkspaceSnapshot,
} from "@terminal-platform/workspace-core";
import { TerminalWorkspace, useWorkspaceSnapshot } from "@terminal-platform/workspace-react";

interface NativeSessionFormState {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

const initialNativeSessionFormState: NativeSessionFormState = {
  title: "SDK Workspace",
  program: resolveDefaultShellProgram(),
  args: "",
  cwd: "",
};

export function TerminalDemoWorkspaceApp(props: {
  config: TerminalRuntimeBootstrapConfig;
}): ReactElement {
  const kernel = useDemoWorkspaceKernel(props.config);

  if (!kernel) {
    return (
      <main className="shell shell--error">
        <section className="panel panel--surface panel--error">
          <div className="section__eyebrow">SDK Bootstrap</div>
          <h1 className="section__title">Terminal Platform Demo</h1>
          <p className="section__copy">Preparing workspace kernel and transport adapters...</p>
        </section>
      </main>
    );
  }

  return <TerminalDemoWorkspaceScreen config={props.config} kernel={kernel} />;
}

export function TerminalDemoWorkspaceScreen(props: {
  config: TerminalRuntimeBootstrapConfig;
  kernel: WorkspaceKernel;
}): ReactElement {
  const snapshot = useWorkspaceSnapshot(props.kernel);
  const [createForm, setCreateForm] = useState(initialNativeSessionFormState);
  const [inputDraft, setInputDraft] = useState("");
  const [createPending, setCreatePending] = useState(false);
  const [commandPending, setCommandPending] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const autoAttachAttemptRef = useRef<SessionId | null>(null);

  const activeSessionId = snapshot.selection.activeSessionId ?? snapshot.catalog.sessions[0]?.session_id ?? null;
  const attachedSessionId = snapshot.attachedSession?.session.session_id ?? null;
  const activeSession = useMemo(
    () => snapshot.catalog.sessions.find((session) => session.session_id === activeSessionId) ?? null,
    [activeSessionId, snapshot.catalog.sessions],
  );
  const activePaneId = snapshot.selection.activePaneId ?? snapshot.attachedSession?.focused_screen?.pane_id ?? null;
  const focusedSequence = snapshot.attachedSession?.focused_screen?.sequence != null
    ? String(snapshot.attachedSession.focused_screen.sequence)
    : null;
  const hasDiagnostics = snapshot.diagnostics.length > 0;
  const canSendInput = Boolean(activeSessionId && activePaneId && inputDraft.trim()) && !commandPending;
  const canIssueSessionCommand = Boolean(activeSessionId) && !commandPending;
  const activeTitle = activeSession?.title ?? snapshot.attachedSession?.session.title ?? "Pick a session to inspect";

  useEffect(() => {
    autoAttachAttemptRef.current = null;
    void props.kernel.bootstrap().catch(() => {
      // Transport failures are recorded in kernel diagnostics.
    });
  }, [props.kernel]);

  useEffect(() => {
    const targetSessionId = snapshot.selection.activeSessionId ?? snapshot.catalog.sessions[0]?.session_id ?? null;

    if (snapshot.connection.state !== "ready" || !targetSessionId) {
      autoAttachAttemptRef.current = null;
      return;
    }

    if (!snapshot.selection.activeSessionId) {
      props.kernel.commands.setActiveSession(targetSessionId);
    }

    if (snapshot.attachedSession?.session.session_id === targetSessionId) {
      autoAttachAttemptRef.current = targetSessionId;
      return;
    }

    if (autoAttachAttemptRef.current === targetSessionId) {
      return;
    }

    autoAttachAttemptRef.current = targetSessionId;
    void props.kernel.commands.attachSession(targetSessionId).catch(() => {
      if (autoAttachAttemptRef.current === targetSessionId) {
        autoAttachAttemptRef.current = null;
      }
    });
  }, [
    props.kernel,
    snapshot.attachedSession,
    snapshot.catalog.sessions,
    snapshot.connection.state,
    snapshot.selection.activeSessionId,
  ]);

  useEffect(() => {
    const debug = {
      controller: props.kernel,
      getState: () => props.kernel.getSnapshot(),
      setInputDraft,
    };
    window.terminalDemoDebug = debug;

    return () => {
      if (window.terminalDemoDebug === debug) {
        delete window.terminalDemoDebug;
      }
    };
  }, [props.kernel]);

  async function handleCreateNativeSession() {
    setActionError(null);
    setCreatePending(true);
    try {
      await props.kernel.commands.createSession("native", buildNativeSessionRequest(createForm));
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCreatePending(false);
    }
  }

  async function handleRefreshCatalog() {
    setActionError(null);
    try {
      await props.kernel.commands.refreshSessions();
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    }
  }

  async function handleRebootstrap() {
    setActionError(null);
    try {
      await props.kernel.commands.bootstrap();
    } catch (error) {
      setActionError(getErrorMessage(error));
    }
  }

  async function handleSendInput() {
    if (!activeSessionId || !activePaneId) {
      return;
    }

    setActionError(null);
    setCommandPending(true);
    try {
      await props.kernel.commands.dispatchMuxCommand(activeSessionId, {
        kind: "send_input",
        pane_id: activePaneId,
        data: `${inputDraft}\n`,
      });
      setInputDraft("");
      autoAttachAttemptRef.current = null;
      await props.kernel.commands.attachSession(activeSessionId);
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  async function handleSaveSession() {
    if (!activeSessionId) {
      return;
    }

    setActionError(null);
    setCommandPending(true);
    try {
      await props.kernel.commands.dispatchMuxCommand(activeSessionId, { kind: "save_session" });
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  async function handleResyncScreen() {
    if (!activeSessionId) {
      return;
    }

    setActionError(null);
    setCommandPending(true);
    try {
      autoAttachAttemptRef.current = null;
      await props.kernel.commands.attachSession(activeSessionId);
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  return (
    <div className="shell">
      <aside className="shell__sidebar panel panel--sidebar">
        <section className="panel hero">
          <div>
            <div className="section__eyebrow">SDK Consumer Path</div>
            <h1 className="hero__title">Terminal Platform Demo</h1>
            <p className="hero__copy">
              This shell now mounts <code>@terminal-platform/workspace-react</code> over the headless
              workspace kernel.
            </p>
          </div>

          <div className="meta-stack meta-stack--inline">
            <span className={`badge ${badgeToneForConnection(snapshot.connection.state)}`}>
              {snapshot.connection.state}
            </span>
            <span className="badge badge--neutral">{props.config.runtimeSlug}</span>
            <span className="badge badge--neutral">{snapshot.catalog.sessions.length} sessions</span>
            <span className="badge badge--neutral">{snapshot.catalog.savedSessions.length} saved</span>
            {focusedSequence ? <span className="badge badge--brand">seq {focusedSequence}</span> : null}
            {hasDiagnostics ? (
              <span className="badge badge--danger">{snapshot.diagnostics.length} diagnostics</span>
            ) : null}
          </div>
        </section>

        <section className="panel panel--surface section">
          <div className="section__header">
            <div>
              <div className="section__eyebrow">Runtime</div>
              <h2 className="section__title">Connection Surface</h2>
            </div>
            <span className="section__meta">
              {snapshot.connection.handshake?.daemon_phase ?? "pending handshake"}
            </span>
          </div>

          <dl className="definition-list">
            <div>
              <dt>Control plane</dt>
              <dd>{props.config.controlPlaneUrl}</dd>
            </div>
            <div>
              <dt>Session stream</dt>
              <dd>{props.config.sessionStreamUrl}</dd>
            </div>
            <div>
              <dt>Active session</dt>
              <dd>{activeTitle}</dd>
            </div>
          </dl>

          <div className="button-row">
            <button className="button" onClick={() => void handleRebootstrap()}>
              Re-bootstrap
            </button>
            <button className="button" onClick={() => void handleRefreshCatalog()}>
              Refresh Catalog
            </button>
            <button className="button" onClick={() => props.kernel.commands.clearDiagnostics()}>
              Clear Diagnostics
            </button>
          </div>
        </section>

        <section className="panel panel--surface section">
          <div className="section__header">
            <div>
              <div className="section__eyebrow">Launch</div>
              <h2 className="section__title">Create Native Session</h2>
            </div>
          </div>

          <div className="form-grid">
            <label>
              <span>Title</span>
              <input
                value={createForm.title}
                onChange={(event) => {
                  setCreateForm((current) => ({
                    ...current,
                    title: event.target.value,
                  }));
                }}
              />
            </label>
            <label>
              <span>Program</span>
              <input
                value={createForm.program}
                onChange={(event) => {
                  setCreateForm((current) => ({
                    ...current,
                    program: event.target.value,
                  }));
                }}
              />
            </label>
            <label>
              <span>Args</span>
              <input
                value={createForm.args}
                onChange={(event) => {
                  setCreateForm((current) => ({
                    ...current,
                    args: event.target.value,
                  }));
                }}
              />
            </label>
            <label>
              <span>Working directory</span>
              <input
                value={createForm.cwd}
                onChange={(event) => {
                  setCreateForm((current) => ({
                    ...current,
                    cwd: event.target.value,
                  }));
                }}
              />
            </label>
          </div>

          <div className="button-row">
            <button
              className="button button--primary"
              disabled={createPending}
              onClick={() => void handleCreateNativeSession()}
            >
              Create Native Session
            </button>
          </div>
        </section>

        {hasDiagnostics ? (
          <section className="panel panel--surface section">
            <div className="section__header">
              <div>
                <div className="section__eyebrow">Diagnostics</div>
                <h2 className="section__title">Workspace Issues</h2>
              </div>
            </div>
            <div className="degraded-list">
              {snapshot.diagnostics.map((diagnostic, index) => (
                <div
                  className="degraded-list__item"
                  key={`${diagnostic.code}-${diagnostic.timestampMs}-${index}`}
                >
                  <strong>{diagnostic.code}</strong>
                  <small>{diagnostic.message}</small>
                </div>
              ))}
            </div>
          </section>
        ) : null}

        {actionError ? (
          <section className="banner banner--warning">
            <strong>Last action failed</strong>
            <div>{actionError}</div>
          </section>
        ) : null}
      </aside>

      <main className="shell__main">
        <section className="panel panel--surface panel--workspace">
          <div className="panel__header">
            <div>
              <div className="section__eyebrow">Workspace Surface</div>
              <h2 className="section__title workspace-summary__title">{activeTitle}</h2>
              <p className="section__copy">
                React mounts the portable SDK component tree here. The UI below is rendered by
                <code> @terminal-platform/workspace-react </code>
                and the underlying Web Components layer.
              </p>
            </div>

            <div className="meta-stack meta-stack--inline">
              {attachedSessionId ? <span className="badge badge--brand">attached {attachedSessionId}</span> : null}
              {activePaneId ? <span className="badge badge--neutral">pane {activePaneId}</span> : null}
            </div>
          </div>

          <div className="workspace-stack">
            <TerminalWorkspace kernel={props.kernel} />

            <section className="terminal-dock" aria-label="Focused pane command lane">
              <div className="terminal-dock__header">
                <div>
                  <div className="section__eyebrow">Command Lane</div>
                  <h3 className="terminal-dock__title">Focused Pane Input</h3>
                </div>

                <div className="meta-stack meta-stack--inline">
                  <span className="badge badge--neutral">
                    {activePaneId ? `pane ${activePaneId}` : "no focused pane"}
                  </span>
                  <span className="badge badge--neutral">
                    {commandPending ? "dispatching" : "ready to send"}
                  </span>
                </div>
              </div>

              <label className="terminal-dock__composer">
                <span className="terminal-dock__prompt" aria-hidden="true">
                  &gt;_
                </span>
                <textarea
                  className="terminal-dock__textarea"
                  value={inputDraft}
                  disabled={!activePaneId || commandPending}
                  onChange={(event) => {
                    setInputDraft(event.target.value);
                  }}
                  placeholder={"printf \"hello from sdk demo\\n\""}
                />
              </label>

              <div className="terminal-dock__footer">
                <div className="terminal-dock__hint">
                  Input is injected into the focused pane and sent with a trailing newline.
                </div>

                <div className="button-row button-row--dock">
                  <button
                    className="button button--primary"
                    disabled={!canSendInput}
                    onClick={() => void handleSendInput()}
                  >
                    Send + Enter
                  </button>
                  <button
                    className="button"
                    disabled={!canIssueSessionCommand}
                    onClick={() => void handleSaveSession()}
                  >
                    Save Session
                  </button>
                  <button
                    className="button"
                    disabled={!canIssueSessionCommand}
                    onClick={() => void handleResyncScreen()}
                  >
                    Re-sync Screen
                  </button>
                </div>
              </div>
            </section>
          </div>
        </section>
      </main>
    </div>
  );
}

function useDemoWorkspaceKernel(config: TerminalRuntimeBootstrapConfig): WorkspaceKernel | null {
  const [kernel, setKernel] = useState<WorkspaceKernel | null>(null);

  useEffect(() => {
    const nextKernel = createWorkspaceKernel({
      transport: createWorkspaceWebSocketTransport({
        controlUrl: config.controlPlaneUrl,
        streamUrl: config.sessionStreamUrl,
      }),
    });

    setKernel(nextKernel);

    return () => {
      setKernel((current) => (current === nextKernel ? null : current));
      void nextKernel.dispose();
    };
  }, [config.controlPlaneUrl, config.sessionStreamUrl]);

  return kernel;
}

function buildNativeSessionRequest(form: NativeSessionFormState): CreateSessionRequest {
  const title = form.title.trim();
  const program = form.program.trim();
  const cwd = form.cwd.trim();

  return {
    title: title || null,
    launch: program
      ? {
          program,
          args: parseLaunchArgs(form.args),
          cwd: cwd || null,
        }
      : null,
  };
}

function parseLaunchArgs(value: string): string[] {
  const matches = value.match(/(?:[^\s"]+|"[^"]*")+/g);
  if (!matches) {
    return [];
  }

  return matches.map((entry) => entry.replace(/^"|"$/g, ""));
}

function badgeToneForConnection(state: WorkspaceSnapshot["connection"]["state"]): string {
  if (state === "ready") {
    return "badge--brand";
  }

  if (state === "error" || state === "disposed") {
    return "badge--danger";
  }

  return "badge--neutral";
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace command failed";
}

function resolveDefaultShellProgram(): string {
  if (typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent)) {
    return "pwsh.exe";
  }

  return "bash";
}

export function createStaticWorkspaceKernel(snapshot: WorkspaceSnapshot): WorkspaceKernel {
  const noopAsync = async () => {};
  const noopDiagnostics: WorkspaceDiagnostics = {
    list: () => snapshot.diagnostics,
    clear: () => {},
  };
  const noopCommands: WorkspaceCommands = {
    bootstrap: noopAsync,
    refreshSessions: noopAsync,
    refreshSavedSessions: noopAsync,
    discoverSessions: noopAsync,
    getBackendCapabilities: async () => {
      throw new Error("not implemented in static kernel");
    },
    createSession: noopAsync,
    importSession: noopAsync,
    attachSession: noopAsync,
    restoreSavedSession: noopAsync,
    deleteSavedSession: noopAsync,
    pruneSavedSessions: noopAsync,
    dispatchMuxCommand: async () => {
      throw new Error("not implemented in static kernel");
    },
    openSubscription: async () => {
      throw new Error("not implemented in static kernel");
    },
    setActiveSession: () => {},
    setActivePane: () => {},
    updateDraft: () => {},
    clearDraft: () => {},
    setTheme: () => {},
    clearDiagnostics: () => {},
  };
  const noopSelectors: WorkspaceSelectors = {
    connection: () => snapshot.connection,
    sessions: () => snapshot.catalog.sessions,
    savedSessions: () => snapshot.catalog.savedSessions,
    activeSession: () => snapshot.catalog.sessions.find((item) => item.session_id === snapshot.selection.activeSessionId) ?? null,
    activePaneId: () => snapshot.selection.activePaneId,
    attachedSession: () => snapshot.attachedSession,
    diagnostics: () => snapshot.diagnostics,
    themeId: () => snapshot.theme.themeId,
  };

  return {
    getSnapshot: () => snapshot,
    subscribe: () => () => {},
    bootstrap: noopAsync,
    dispose: noopAsync,
    commands: noopCommands,
    selectors: noopSelectors,
    diagnostics: noopDiagnostics,
  };
}

export function renderWorkspaceFatalError(error: string): ReactElement {
  return <TerminalRuntimeBootstrapErrorView error={error} />;
}
