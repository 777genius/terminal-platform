import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import { terminalPlatformThemeManifests } from "@terminal-platform/design-tokens";
import type { CreateSessionRequest, SessionId } from "@terminal-platform/runtime-types";
import { createWorkspaceWebSocketTransport } from "@terminal-platform/workspace-adapter-websocket";
import {
  createWorkspaceKernel,
  type WorkspaceKernel,
  type WorkspaceSnapshot,
} from "@terminal-platform/workspace-core";
import {
  TerminalWorkspace,
  compactTerminalId,
  findRestorableSavedSession,
  hasSavedSession,
  resolveTerminalSavedSessionsControlState,
  useWorkspaceSnapshot,
  type TerminalCommandQuickCommand,
  type TerminalSavedSessionPendingAction,
  type TerminalSavedSessionRestoreSemanticsTone,
} from "@terminal-platform/workspace-react";
import {
  DEFAULT_TERMINAL_DEMO_DISPLAY,
  createDemoPreviewBackendCapabilities,
  createDemoPreviewWorkspaceSnapshot,
  createStaticWorkspaceKernel,
} from "./terminal-demo-static-workspace.js";

export {
  createDemoPreviewBackendCapabilities,
  createDemoPreviewWorkspaceSnapshot,
  createStaticWorkspaceKernel,
} from "./terminal-demo-static-workspace.js";

interface NativeSessionFormState {
  title: string;
  program: string;
  args: string;
  cwd: string;
}

const TERMINAL_DEMO_THEME_STORAGE_KEY = "terminal-platform-demo.theme";
const TERMINAL_DEMO_FONT_SCALE_STORAGE_KEY = "terminal-platform-demo.terminal-font-scale";
const TERMINAL_DEMO_LINE_WRAP_STORAGE_KEY = "terminal-platform-demo.terminal-line-wrap";
const ADVANCED_SAVED_LAYOUT_VISIBLE_COUNT = 6;
const terminalDemoThemeIds = terminalPlatformThemeManifests.map((theme) => theme.id);
const terminalDemoQuickCommands = [
  {
    label: "pwd",
    value: "pwd",
    description: "Show the current working directory",
  },
  {
    label: "ls -la",
    value: "ls -la",
    description: "List files with metadata",
  },
  {
    label: "git status",
    value: "git status",
    description: "Inspect the current git worktree",
  },
  {
    label: "node -v",
    value: "node -v",
    description: "Print the active Node.js version",
  },
  {
    label: "hello",
    value: 'printf "hello from Terminal Platform\\n"',
    description: "Print a Terminal Platform greeting",
  },
] satisfies TerminalCommandQuickCommand[];
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
  const [createForm, setCreateForm] = useState(() => createInitialNativeSessionFormState(props.config));
  const [createPending, setCreatePending] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [advancedSavedSessionAction, setAdvancedSavedSessionAction] = useState<{
    sessionId: SessionId;
    action: TerminalSavedSessionPendingAction;
  } | null>(null);
  const [advancedSavedSessionDeleteConfirmationId, setAdvancedSavedSessionDeleteConfirmationId] =
    useState<SessionId | null>(null);
  const autoAttachAttemptRef = useRef<SessionId | null>(null);

  const activeSessionId = snapshot.selection.activeSessionId ?? snapshot.catalog.sessions[0]?.session_id ?? null;
  const attachedSessionId = snapshot.attachedSession?.session.session_id ?? null;
  const activeSession = useMemo(
    () => snapshot.catalog.sessions.find((session) => session.session_id === activeSessionId) ?? null,
    [activeSessionId, snapshot.catalog.sessions],
  );
  const activePaneId = snapshot.selection.activePaneId ?? snapshot.attachedSession?.focused_screen?.pane_id ?? null;
  const hasDiagnostics = snapshot.diagnostics.length > 0;
  const activeTitle = activeSession?.title ?? snapshot.attachedSession?.session.title ?? "Pick a session to inspect";
  const attachedSessionLabel = attachedSessionId ? compactTerminalId(attachedSessionId) : null;
  const activePaneLabel = activePaneId ? compactTerminalId(activePaneId) : null;
  const connectionSummary = describeConnectionState(snapshot.connection.state);
  const activeHealth = snapshot.attachedSession?.health ?? null;
  const healthSummary = describeSessionHealth(activeHealth?.phase ?? null);
  const activeScreen = snapshot.attachedSession?.focused_screen ?? null;
  const terminalDisplay = snapshot.terminalDisplay ?? DEFAULT_TERMINAL_DEMO_DISPLAY;
  const diagnosticsPreview = snapshot.diagnostics.slice(0, 3);
  const advancedNoticeCount = diagnosticsPreview.length + (actionError ? 1 : 0);
  const advancedSavedSessionsControl = useMemo(
    () => resolveTerminalSavedSessionsControlState(snapshot, {
      visibleSavedSessionCount: ADVANCED_SAVED_LAYOUT_VISIBLE_COUNT,
      pendingSavedSessionId: advancedSavedSessionAction?.sessionId ?? null,
      pendingSavedSessionAction: advancedSavedSessionAction?.action ?? null,
      pendingBulkAction: null,
      deleteConfirmationSessionId: advancedSavedSessionDeleteConfirmationId,
      pruneConfirmationArmed: false,
    }),
    [advancedSavedSessionAction, advancedSavedSessionDeleteConfirmationId, snapshot],
  );

  useEffect(() => {
    autoAttachAttemptRef.current = null;
    void props.kernel.bootstrap().catch(() => {
      // Transport failures are recorded in kernel diagnostics.
    });
  }, [props.kernel]);

  useEffect(() => {
    persistTerminalDemoThemeId(snapshot.theme.themeId);
  }, [snapshot.theme.themeId]);

  useEffect(() => {
    persistTerminalDemoDisplayPreferences(terminalDisplay);
  }, [terminalDisplay.fontScale, terminalDisplay.lineWrap]);

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

  const createNativeSession = useCallback(async (form: NativeSessionFormState): Promise<boolean> => {
    setActionError(null);
    setCreatePending(true);
    try {
      await props.kernel.commands.createSession("native", buildNativeSessionRequest(form));
      return true;
    } catch (error) {
      setActionError(getErrorMessage(error));
      return false;
    } finally {
      setCreatePending(false);
    }
  }, [props.kernel]);

  useEffect(() => {
    setCreateForm((current) => {
      if (current.program.trim()) {
        return current;
      }

      return createInitialNativeSessionFormState(props.config);
    });
  }, [props.config]);

  useEffect(() => {
    const debug = {
      controller: props.kernel,
      getState: () => props.kernel.getSnapshot(),
    };
    window.terminalDemoDebug = debug;

    return () => {
      if (window.terminalDemoDebug === debug) {
        delete window.terminalDemoDebug;
      }
    };
  }, [props.kernel]);

  async function handleCreateNativeSession() {
    await createNativeSession(createForm);
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

  async function handleRestoreSavedSession(sessionId: SessionId) {
    setActionError(null);
    if (advancedSavedSessionAction) {
      setActionError("Wait for the current saved layout action to finish.");
      return;
    }

    setAdvancedSavedSessionDeleteConfirmationId(null);
    const currentSnapshot = props.kernel.getSnapshot();
    const session = findRestorableSavedSession(currentSnapshot, {
      visibleSavedSessionCount: currentSnapshot.catalog.savedSessions.length,
      pendingSavedSessionId: null,
      pendingSavedSessionAction: null,
      pendingBulkAction: null,
      deleteConfirmationSessionId: null,
      pruneConfirmationArmed: false,
    }, sessionId);
    if (!session) {
      setActionError(resolveSavedLayoutRestoreBlockedMessage(currentSnapshot, sessionId));
      return;
    }

    setAdvancedSavedSessionAction({ sessionId, action: "restore" });
    try {
      await props.kernel.commands.restoreSavedSession(sessionId);
      await props.kernel.commands.refreshSessions();
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setAdvancedSavedSessionAction(null);
    }
  }

  async function handleDeleteSavedSession(sessionId: SessionId) {
    setActionError(null);
    if (advancedSavedSessionAction) {
      setActionError("Wait for the current saved layout action to finish.");
      return;
    }

    const currentSnapshot = props.kernel.getSnapshot();
    if (!hasSavedSession(currentSnapshot, sessionId)) {
      setActionError("Saved layout is no longer available.");
      setAdvancedSavedSessionDeleteConfirmationId(null);
      return;
    }

    if (advancedSavedSessionDeleteConfirmationId !== sessionId) {
      setAdvancedSavedSessionDeleteConfirmationId(sessionId);
      return;
    }

    setAdvancedSavedSessionDeleteConfirmationId(null);
    setAdvancedSavedSessionAction({ sessionId, action: "delete" });
    try {
      await props.kernel.commands.deleteSavedSession(sessionId);
      await props.kernel.commands.refreshSavedSessions();
    } catch (error) {
      setActionError(getErrorMessage(error));
    } finally {
      setAdvancedSavedSessionAction(null);
    }
  }

  return (
    <div
      className="shell"
      data-has-active-session={activeSessionId ? "true" : "false"}
      data-testid="terminal-demo-shell"
    >
      <aside className="shell__sidebar">
        <section className="panel hero hero--demo">
          <div className="hero__content">
            <div className="section__eyebrow">Terminal Platform</div>
            <h1 className="hero__title">NativeMux workspace</h1>
            <p className="hero__copy">{connectionSummary.copy}</p>

            <div className="hero__kpis" aria-label="Workspace summary">
              <div className="hero__stat">
                <span>Sessions</span>
                <strong>{snapshot.catalog.sessions.length}</strong>
              </div>
              <div className="hero__stat">
                <span>Saved</span>
                <strong>{snapshot.catalog.savedSessions.length}</strong>
              </div>
              <div className="hero__stat">
                <span>Health</span>
                <strong>{healthSummary.label}</strong>
              </div>
            </div>
          </div>

          <div className="meta-stack">
            <span className={`badge ${badgeToneForConnection(snapshot.connection.state)}`}>
              {connectionSummary.label}
            </span>
            <span className="badge badge--neutral">{snapshot.catalog.sessions.length} running shells</span>
            <span className="badge badge--neutral">
              {activeSessionId ? "Shell selected" : "No shell selected"}
            </span>
          </div>
        </section>

        <section className="panel panel--surface section">
          <div className="section__header">
            <div>
              <div className="section__eyebrow">Launch</div>
              <h2 className="section__title">Session launcher</h2>
            </div>
            <span className="section__meta">{createPending ? "starting" : "ready"}</span>
          </div>

          <div className="button-row">
            <button
              className="button button--primary"
              data-testid="start-default-shell"
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
                {activePaneId ? (
                  <span title={activePaneId}> - pane {activePaneLabel}</span>
                ) : null}
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
                      name="terminal-demo-session-title"
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
                      name="terminal-demo-session-program"
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
                      name="terminal-demo-session-args"
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
                      name="terminal-demo-session-cwd"
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
                    <dd title={attachedSessionId ?? undefined}>{attachedSessionLabel ?? "Not attached yet"}</dd>
                  </div>
                </dl>
              </div>

              {advancedSavedSessionsControl.savedSessionCount > 0 ? (
                <div className="advanced-block">
                  <div className="section__eyebrow">Saved layouts</div>
                  <div
                    className="list-stack list-stack--scroll"
                    data-testid="advanced-saved-layouts"
                    data-saved-count={advancedSavedSessionsControl.savedSessionCount}
                    data-visible-count={advancedSavedSessionsControl.visibleCount}
                    data-hidden-count={advancedSavedSessionsControl.hiddenCount}
                    data-pending={advancedSavedSessionsControl.anyPending ? "true" : "false"}
                  >
                    {advancedSavedSessionsControl.items.map((item) => (
                      <div
                        className="list-card list-card--saved list-card--advanced-saved"
                        data-testid="advanced-saved-layout"
                        data-can-restore={item.canRestore ? "true" : "false"}
                        data-restore-status={item.restoreStatus}
                        key={item.session.session_id}
                      >
                        <div className="saved-layout-card__body">
                          <strong>{item.title}</strong>
                          <small>{item.compatibilityLabel}</small>
                          <div className="saved-layout-card__badges" aria-label="Restore semantics">
                            {item.restoreSemanticsNotes.map((note) => (
                              <span
                                className={`badge ${badgeClassForRestoreSemantics(note.tone)}`}
                                data-semantics-code={note.code}
                                data-semantics-tone={note.tone}
                                data-testid="advanced-saved-layout-restore-semantics"
                                key={note.code}
                                title={note.detail}
                              >
                                {note.label}
                              </span>
                            ))}
                          </div>
                        </div>
                        <div className="button-row button-row--compact">
                          <button
                            className="button"
                            data-can-restore={item.canRestore ? "true" : "false"}
                            data-restore-status={item.restoreStatus}
                            data-testid="advanced-restore-saved-layout"
                            disabled={!item.canRestore}
                            onClick={() => void handleRestoreSavedSession(item.session.session_id)}
                            title={item.restoreTitle}
                          >
                            {item.isRestoring ? "Restoring..." : "Restore"}
                          </button>
                          <button
                            className="button button--danger"
                            data-confirming={item.isConfirmingDelete ? "true" : "false"}
                            data-testid="advanced-delete-saved-layout"
                            disabled={!item.canDelete}
                            onClick={() => void handleDeleteSavedSession(item.session.session_id)}
                            title={item.isConfirmingDelete ? "Confirm saved layout deletion." : "Delete saved layout."}
                          >
                            {item.isDeleting ? "Deleting..." : item.isConfirmingDelete ? "Confirm delete" : "Delete"}
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                  {advancedSavedSessionsControl.hiddenCount > 0 ? (
                    <small className="section__copy">
                      Showing {advancedSavedSessionsControl.visibleCount} of{" "}
                      {advancedSavedSessionsControl.savedSessionCount} saved layouts.
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
          <div className="panel__header panel__header--workspace">
            <div>
              <div className="section__eyebrow">Workspace</div>
              <h2 className="section__title workspace-summary__title" data-testid="workspace-active-title">
                {activeTitle}
              </h2>
              <p className="section__copy">{healthSummary.copy}</p>
            </div>

            <div className="meta-stack meta-stack--inline">
              <span className={`badge ${badgeToneForConnection(snapshot.connection.state)}`}>
                {connectionSummary.label}
              </span>
              <span className={`badge ${healthSummary.badgeClass}`}>{healthSummary.label}</span>
              {activePaneId ? (
                <span
                  className="badge badge--neutral"
                  data-testid="workspace-focused-pane-badge"
                  title={activePaneId}
                >
                  Focused pane {activePaneLabel}
                </span>
              ) : null}
              {activeScreen ? (
                <span className="badge badge--neutral">
                  {activeScreen.cols}x{activeScreen.rows}
                </span>
              ) : null}
            </div>
          </div>

          <div className="workspace-stack">
            <div className="terminal-workspace-host" data-testid="terminal-workspace-host">
              <TerminalWorkspace
                autoFocusCommandInput={true}
                kernel={props.kernel}
                quickCommands={terminalDemoQuickCommands}
              />
            </div>
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
      availableThemeIds: terminalDemoThemeIds,
      initialThemeId: readStoredTerminalDemoThemeId(),
      initialTerminalFontScale: readStoredValue(TERMINAL_DEMO_FONT_SCALE_STORAGE_KEY),
      initialTerminalLineWrap: readStoredBoolean(TERMINAL_DEMO_LINE_WRAP_STORAGE_KEY),
    });

    setKernel(nextKernel);

    return () => {
      setKernel((current) => (current === nextKernel ? null : current));
      void nextKernel.dispose();
    };
  }, [config.controlPlaneUrl, config.sessionStreamUrl]);

  return kernel;
}

function readStoredTerminalDemoThemeId(): string | null {
  return readStoredValue(TERMINAL_DEMO_THEME_STORAGE_KEY);
}

function readStoredValue(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function readStoredBoolean(key: string): boolean | null {
  const value = readStoredValue(key);
  if (value === "true") {
    return true;
  }

  if (value === "false") {
    return false;
  }

  return null;
}

function persistTerminalDemoThemeId(themeId: string): void {
  try {
    window.localStorage.setItem(TERMINAL_DEMO_THEME_STORAGE_KEY, themeId);
  } catch {
    // Theme persistence is a convenience and must not affect terminal control.
  }
}

function persistTerminalDemoDisplayPreferences(
  terminalDisplay: WorkspaceSnapshot["terminalDisplay"],
): void {
  try {
    window.localStorage.setItem(TERMINAL_DEMO_FONT_SCALE_STORAGE_KEY, terminalDisplay.fontScale);
    window.localStorage.setItem(TERMINAL_DEMO_LINE_WRAP_STORAGE_KEY, String(terminalDisplay.lineWrap));
  } catch {
    // Display persistence is a convenience and must not affect terminal control.
  }
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

function badgeClassForRestoreSemantics(tone: TerminalSavedSessionRestoreSemanticsTone): string {
  if (tone === "ok") {
    return "badge--success";
  }

  if (tone === "warning") {
    return "badge--warning";
  }

  return "badge--neutral";
}

function resolveSavedLayoutRestoreBlockedMessage(snapshot: WorkspaceSnapshot, sessionId: SessionId): string {
  const controls = resolveTerminalSavedSessionsControlState(snapshot, {
    visibleSavedSessionCount: snapshot.catalog.savedSessions.length,
    pendingSavedSessionId: null,
    pendingSavedSessionAction: null,
    pendingBulkAction: null,
    deleteConfirmationSessionId: null,
    pruneConfirmationArmed: false,
  });
  const item = controls.items.find((candidate) => candidate.session.session_id === sessionId);

  return item?.restoreTitle ?? "Saved layout is no longer available.";
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

function describeSessionHealth(phase: string | null): {
  label: string;
  copy: string;
  badgeClass: string;
} {
  if (phase === "ready") {
    return {
      label: "Healthy",
      copy: "The focused session is attachable and serving fresh topology and screen snapshots.",
      badgeClass: "badge--success",
    };
  }

  if (phase === "degraded") {
    return {
      label: "Degraded",
      copy: "The focused session is available with explicit degraded semantics from the runtime.",
      badgeClass: "badge--warning",
    };
  }

  if (phase === "stale") {
    return {
      label: "Stale",
      copy: "The focused session needs a refresh before its output should be trusted.",
      badgeClass: "badge--warning",
    };
  }

  if (phase === "terminated") {
    return {
      label: "Terminated",
      copy: "The selected session is no longer attachable.",
      badgeClass: "badge--danger",
    };
  }

  return {
    label: "Pending",
    copy: "No session health snapshot has been attached yet.",
    badgeClass: "badge--neutral",
  };
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return "Workspace command failed";
}

function createInitialNativeSessionFormState(config: TerminalRuntimeBootstrapConfig): NativeSessionFormState {
  return {
    title: "SDK Workspace",
    program: resolveDefaultShellProgram(config),
    args: "",
    cwd: "",
  };
}

function resolveDefaultShellProgram(config: TerminalRuntimeBootstrapConfig): string {
  const configuredProgram = config.demoDefaultShellProgram?.trim();
  if (configuredProgram) {
    return configuredProgram;
  }

  if (typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent)) {
    return "pwsh.exe";
  }

  if (typeof navigator !== "undefined" && /macintosh|mac os x/i.test(navigator.userAgent)) {
    return "zsh";
  }

  return "bash";
}
