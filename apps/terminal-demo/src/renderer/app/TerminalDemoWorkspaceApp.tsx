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
  const hasDiagnostics = snapshot.diagnostics.length > 0;
  const canSendInput = Boolean(activeSessionId && activePaneId && inputDraft.trim()) && !commandPending;
  const canIssueSessionCommand = Boolean(activeSessionId) && !commandPending;
  const activeTitle = activeSession?.title ?? snapshot.attachedSession?.session.title ?? "Pick a session to inspect";
  const connectionSummary = describeConnectionState(snapshot.connection.state);
  const diagnosticsPreview = snapshot.diagnostics.slice(0, 3);
  const advancedNoticeCount = diagnosticsPreview.length + (actionError ? 1 : 0);
  const quickCommands = useMemo(
    () => [
      { label: "pwd", value: "pwd" },
      { label: "ls -la", value: "ls -la" },
      { label: "git status", value: "git status" },
      { label: "hello demo", value: 'printf "hello from sdk demo\\n"' },
    ],
    [],
  );

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

  async function handleRestoreSavedSession(sessionId: SessionId) {
    setActionError(null);
    setCommandPending(true);
    try {
      await props.kernel.commands.restoreSavedSession(sessionId);
      await props.kernel.commands.refreshSessions();
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  async function handleDeleteSavedSession(sessionId: SessionId) {
    setActionError(null);
    setCommandPending(true);
    try {
      await props.kernel.commands.deleteSavedSession(sessionId);
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setCommandPending(false);
    }
  }

  return (
    <div className="shell">
      <aside className="shell__sidebar panel panel--sidebar">
        <section className="panel hero hero--demo">
          <div className="hero__content">
            <div className="section__eyebrow">Terminal Demo</div>
            <h1 className="hero__title">Start a shell and run commands</h1>
            <p className="hero__copy">
              Start a shell, pick the session in the workspace, then type commands in the dock below the
              terminal output.
            </p>

            <div className="hero__flow" aria-label="Demo flow">
              <span className="hero__flow-item">1. Start shell</span>
              <span className="hero__flow-item">2. Pick session</span>
              <span className="hero__flow-item">3. Send command</span>
            </div>
          </div>

          <div className="meta-stack">
            <span className={`badge ${badgeToneForConnection(snapshot.connection.state)}`}>
              {connectionSummary.label}
            </span>
            <span className="badge badge--neutral">{snapshot.catalog.sessions.length} running shells</span>
            <span className="badge badge--neutral">
              {activeSessionId ? "Shell selected" : "Pick a shell"}
            </span>
          </div>
        </section>

        <section className="panel panel--surface section">
          <div className="section__header">
            <div>
              <div className="section__eyebrow">Launch</div>
              <h2 className="section__title">Start a shell</h2>
            </div>
            <span className="section__meta">{createPending ? "starting" : "ready"}</span>
          </div>

          <p className="section__copy">
            Use the default shell for the fastest path, or open advanced options if you want another
            program or working directory.
          </p>

          <div className="button-row">
            <button
              className="button button--primary"
              disabled={createPending}
              onClick={() => void handleCreateNativeSession()}
            >
              {createPending ? "Starting shell..." : "Start default shell"}
            </button>
          </div>

          {activeSessionId ? (
            <div className="banner banner--subtle section-callout">
              <strong>Current focus</strong>
              <div>
                {activeTitle}
                {activePaneId ? ` - pane ${activePaneId}` : ""}
              </div>
            </div>
          ) : (
            <div className="banner banner--subtle section-callout">
              <strong>No shell selected yet</strong>
              <div>Start a shell, then pick it from the workspace rail to route terminal output and input.</div>
            </div>
          )}

          <details className="details-panel">
            <summary>
              Advanced tools
              {advancedNoticeCount > 0
                ? ` - ${advancedNoticeCount} notice${advancedNoticeCount === 1 ? "" : "s"}`
                : ""}
            </summary>

            <div className="advanced-stack">
              <div className="advanced-block">
                <div className="section__eyebrow">Maintenance</div>
                <div className="advanced-actions">
                  <button className="button" disabled={createPending} onClick={() => void handleRefreshCatalog()}>
                    Refresh shells
                  </button>
                  <button className="button" onClick={() => void handleRebootstrap()}>
                    Reconnect workspace
                  </button>
                </div>
              </div>

              {actionError || hasDiagnostics ? (
                <div className="advanced-block">
                  <div className="section__eyebrow">Notices</div>

                  {actionError ? (
                    <div className="banner banner--warning">
                      <strong>Last action failed</strong>
                      <div>{actionError}</div>
                    </div>
                  ) : null}

                  {hasDiagnostics ? (
                    <div className="degraded-list">
                      {diagnosticsPreview.map((diagnostic, index) => (
                        <div
                          className="degraded-list__item"
                          key={`${diagnostic.code}-${diagnostic.timestampMs}-${index}`}
                        >
                          <strong>{diagnostic.code}</strong>
                          <small>{diagnostic.message}</small>
                        </div>
                      ))}
                      {snapshot.diagnostics.length > diagnosticsPreview.length ? (
                        <small className="section__copy">
                          {snapshot.diagnostics.length - diagnosticsPreview.length} more notices are listed
                          inside the workspace tools panel.
                        </small>
                      ) : null}
                    </div>
                  ) : null}
                </div>
              ) : null}

              <div className="advanced-block">
                <div className="section__eyebrow">Advanced launch</div>

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
              </div>

              <div className="advanced-block">
                <div className="section__eyebrow">Runtime details</div>

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
                    <dt>Attached session</dt>
                    <dd>{attachedSessionId ?? "Not attached yet"}</dd>
                  </div>
                </dl>
              </div>

              {snapshot.catalog.savedSessions.length > 0 ? (
                <div className="advanced-block">
                  <div className="section__eyebrow">Saved layouts</div>
                  <div className="list-stack list-stack--scroll">
                    {snapshot.catalog.savedSessions.slice(0, 6).map((session) => (
                      <div className="list-card list-card--saved" key={session.session_id}>
                        <div>
                          <strong>{session.title ?? session.session_id}</strong>
                          <small>{session.compatibility.status}</small>
                        </div>
                        <div className="button-row">
                          <button
                            className="button"
                            disabled={commandPending}
                            onClick={() => void handleRestoreSavedSession(session.session_id)}
                          >
                            Restore
                          </button>
                          <button
                            className="button"
                            disabled={commandPending}
                            onClick={() => void handleDeleteSavedSession(session.session_id)}
                          >
                            Delete
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                  {snapshot.catalog.savedSessions.length > 6 ? (
                    <small className="section__copy">
                      Showing 6 of {snapshot.catalog.savedSessions.length} saved layouts.
                    </small>
                  ) : null}
                </div>
              ) : null}
            </div>
          </details>
        </section>
      </aside>

      <main className="shell__main">
        <section className="panel panel--surface panel--workspace">
          <div className="panel__header">
            <div>
              <div className="section__eyebrow">Workspace</div>
              <h2 className="section__title workspace-summary__title">{activeTitle}</h2>
              <p className="section__copy">
                Pick a running shell in the rail, watch output here, then send commands from the dock below.
              </p>
            </div>

            <div className="meta-stack meta-stack--inline">
              <span className={`badge ${badgeToneForConnection(snapshot.connection.state)}`}>
                {connectionSummary.label}
              </span>
              {activePaneId ? <span className="badge badge--neutral">Focused pane {activePaneId}</span> : null}
            </div>
          </div>

          <div className="workspace-stack">
            <TerminalWorkspace kernel={props.kernel} />

            <section className="terminal-dock" aria-label="Focused pane command lane">
              <div className="terminal-dock__header">
                <div>
                  <div className="section__eyebrow">Command Input</div>
                  <h3 className="terminal-dock__title">Send text to the focused pane</h3>
                </div>

                <div className="meta-stack meta-stack--inline">
                  <span className="badge badge--neutral">
                    {activePaneId ? `Pane ${activePaneId}` : "Pick a pane first"}
                  </span>
                  <span className="badge badge--neutral">
                    {commandPending ? "Sending..." : "Ready"}
                  </span>
                </div>
              </div>

              <div className="terminal-dock__presets">
                {quickCommands.map((command) => (
                  <button
                    key={command.label}
                    className="terminal-chip"
                    type="button"
                    onClick={() => {
                      setInputDraft(command.value);
                      setActionError(null);
                    }}
                  >
                    {command.label}
                  </button>
                ))}
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
                  {activePaneId
                    ? "Type a command and press send. The dock automatically appends a newline."
                    : "Start or select a session in the workspace, then focus a pane to type here."}
                </div>

                <div className="button-row button-row--dock">
                  <button
                    className="button button--primary"
                    disabled={!canSendInput}
                    onClick={() => void handleSendInput()}
                  >
                    Send command
                  </button>
                </div>
              </div>

              <details className="terminal-dock__more">
                <summary>Session tools</summary>
                <div className="button-row">
                  <button
                    className="button"
                    disabled={!canIssueSessionCommand}
                    onClick={() => void handleSaveSession()}
                  >
                    Save layout
                  </button>
                  <button
                    className="button"
                    disabled={!canIssueSessionCommand}
                    onClick={() => void handleResyncScreen()}
                  >
                    Refresh terminal
                  </button>
                </div>
              </details>
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

function describeConnectionState(state: WorkspaceSnapshot["connection"]["state"]): {
  label: string;
  copy: string;
} {
  if (state === "ready") {
    return {
      label: "Connected",
      copy: "The workspace is connected. You can start a shell, switch sessions, and send commands below.",
    };
  }

  if (state === "error") {
    return {
      label: "Connection issue",
      copy: "The runtime is reachable but the workspace hit an error. Use reconnect or check the alerts panel.",
    };
  }

  if (state === "disposed") {
    return {
      label: "Closed",
      copy: "The workspace controller was disposed. Reconnect the workspace to continue.",
    };
  }

  return {
    label: "Connecting",
    copy: "The demo is still attaching to the local runtime. Once ready, the workspace and command dock will wake up.",
  };
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
