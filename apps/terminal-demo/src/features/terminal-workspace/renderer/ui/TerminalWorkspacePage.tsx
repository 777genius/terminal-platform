import type { ReactElement } from "react";
import type { TerminalWorkspacePageCommands } from "../commands/TerminalWorkspacePageCommands.js";
import type {
  TerminalWorkspaceBadgeModel,
  TerminalWorkspaceBannerModel,
  TerminalWorkspaceDegradedReasonModel,
  TerminalWorkspaceDiscoveredSessionModel,
  TerminalWorkspacePageModel,
  TerminalWorkspacePaneTreeNodeModel,
} from "../view-models/TerminalWorkspacePageModel.js";

type TerminalWorkspacePageProps = TerminalWorkspacePageModel & {
  commands: TerminalWorkspacePageCommands;
};

export function TerminalWorkspacePage(
  props: TerminalWorkspacePageProps,
): ReactElement {
  return (
    <div className="shell">
      <aside className="shell__sidebar panel panel--sidebar">
        <section className="section">
          <div className="section__eyebrow">Gateway</div>
          <h1 className="section__title">Terminal Platform Demo</h1>
          <div className="meta-stack">
            <Badge {...props.statusBadge} />
            <Badge {...props.sessionStatusBadge} />
            <Badge {...props.sessionStreamBadge} />
          </div>
          <p className="section__copy">Renderer talks only to a local WebSocket gateway. Electron stays a leaf host.</p>
          <dl className="definition-list">
            {props.gatewayInfo.map((item) => (
              <div key={item.label}>
                <dt>{item.label}</dt>
                <dd>{item.value}</dd>
              </div>
            ))}
          </dl>
          <DegradedReasonList reasons={props.handshakeDegradedReasons} />
        </section>

        <section className="section">
          <div className="section__eyebrow">Create Native</div>
          <div className="form-grid">
            <label>
              <span>Title</span>
              <input value={props.createForm.title} onChange={(event) => props.commands.createSession.setTitle(event.target.value)} />
            </label>
            <label>
              <span>Program</span>
              <input placeholder="optional custom shell" value={props.createForm.program} onChange={(event) => props.commands.createSession.setProgram(event.target.value)} />
            </label>
            <label>
              <span>Args</span>
              <input placeholder='for example -l "-c echo demo"' value={props.createForm.args} onChange={(event) => props.commands.createSession.setArgs(event.target.value)} />
            </label>
            <label>
              <span>Cwd</span>
              <input placeholder="optional working directory" value={props.createForm.cwd} onChange={(event) => props.commands.createSession.setCwd(event.target.value)} />
            </label>
          </div>
          <button className="button button--primary" onClick={() => void props.commands.createSession.submit()}>
            Create Native Session
          </button>
        </section>

        <section className="section">
          <div className="section__eyebrow">Sessions</div>
          <div className="list-stack">
            {props.sessionItems.map((session) => (
              <button
                key={session.sessionId}
                className={session.isActive ? "list-item list-item--active" : "list-item"}
                onClick={() => void props.commands.sessions.select(session.sessionId)}
              >
                <span>{session.title}</span>
                <small>{session.meta}</small>
              </button>
            ))}
            {props.sessionItems.length === 0 ? <EmptyState text="No attached sessions yet." /> : null}
          </div>
        </section>

        <section className="section">
          <div className="section__eyebrow">Foreign Backends</div>
          {props.discoveredGroups.map((group) => (
            <DiscoveredGroup
              key={group.key}
              label={group.label}
              sessions={group.sessions}
              emptyText={group.emptyText}
              onImport={(importHandle) => props.commands.discoveredSessions.importSession(importHandle)}
            />
          ))}
        </section>

        <section className="section section--saved">
          <div className="section__header">
            <div className="section__eyebrow">Saved Sessions</div>
            {props.savedSessionItems.length > 0 ? (
              <span className="section__meta">{props.savedSessionItems.length + props.hiddenSavedSessionsCount}</span>
            ) : null}
          </div>
          <div className="list-stack list-stack--scroll">
            {props.savedSessionItems.map((saved) => (
              <div key={saved.sessionId} className="list-card list-card--saved">
                <div>
                  <strong>{saved.title}</strong>
                  <small>{saved.meta}</small>
                </div>
                <DegradedReasonList reasons={saved.degradedReasons} />
                <div className="button-row button-row--compact">
                  <button className="button" onClick={() => void props.commands.savedSessions.restore(saved.sessionId)} disabled={!saved.canRestore}>
                    Restore
                  </button>
                  <button className="button button--danger" onClick={() => void props.commands.savedSessions.delete(saved.sessionId)}>
                    Delete
                  </button>
                </div>
              </div>
            ))}
            {props.savedSessionItems.length === 0 ? <EmptyState text="No saved sessions yet." /> : null}
          </div>
          {props.hiddenSavedSessionsCount > 0 ? (
            <button
              className="button button--compact"
              onClick={() => props.commands.savedSessions.toggleVisibility()}
            >
              {props.showAllSavedSessions
                ? "Show recent only"
                : `Show ${props.hiddenSavedSessionsCount} more`}
            </button>
          ) : null}
        </section>
      </aside>

      <main className="shell__main">
        <section className="hero panel">
          <div>
            <div className="section__eyebrow">Session Surface</div>
            <h2 className="hero__title">{props.activeSessionTitle}</h2>
            <p className="hero__copy">
              Topology, focus, screen projection and mux commands come from the Rust daemon over the gateway.
            </p>
          </div>
          <div className="button-row">
            <button className="button" onClick={() => void props.commands.sessions.refreshCatalog()}>
              Refresh Catalog
            </button>
            <button className="button" onClick={() => void props.commands.topology.newTab()} disabled={!props.toolbar.canNewTab}>
              New Tab
            </button>
            <button className="button" onClick={() => void props.commands.topology.splitHorizontal()} disabled={!props.toolbar.canSplit}>
              Split Row
            </button>
            <button className="button" onClick={() => void props.commands.topology.splitVertical()} disabled={!props.toolbar.canSplit}>
              Split Column
            </button>
            <button className="button button--primary" onClick={() => void props.commands.topology.saveSession()} disabled={!props.toolbar.canSave}>
              Save Session
            </button>
          </div>
        </section>

        {props.errorBanner ? <Banner {...props.errorBanner} /> : null}
        {props.sessionStreamBanner ? <Banner {...props.sessionStreamBanner} /> : null}
        {props.actionDegradedBanner ? <Banner {...props.actionDegradedBanner} /> : null}
        {props.actionErrorBanner ? <Banner {...props.actionErrorBanner} /> : null}

        <section className="workspace-grid">
          <div className="panel panel--surface">
            <div className="panel__header">
              <div>
                <div className="section__eyebrow">Topology</div>
                <h3>Tabs and pane tree</h3>
              </div>
              {props.activeBackendBadge ? <Badge {...props.activeBackendBadge} /> : null}
            </div>
            {props.topologyTabs.length > 0 ? (
              <>
                <div className="tab-strip">
                  {props.topologyTabs.map((tab) => (
                    <button
                      key={tab.tabId}
                      className={tab.isFocused ? "tab-pill tab-pill--active" : "tab-pill"}
                      onClick={() => void props.commands.topology.focusTab(tab.tabId)}
                    >
                      {tab.title}
                    </button>
                  ))}
                </div>
                <div className="pane-tree-grid">
                  {props.topologyTabs.map((tab) => (
                    <div key={tab.tabId} className="pane-tree-card">
                      <div className="pane-tree-card__title">{tab.title}</div>
                      <PaneTreeNodeView node={tab.root} onFocus={(paneId) => void props.commands.topology.focusPane(paneId)} />
                    </div>
                  ))}
                </div>
              </>
            ) : (
              <EmptyState text="No session state loaded yet." />
            )}
          </div>

          <div className="panel panel--surface">
            <div className="panel__header">
              <div>
                <div className="section__eyebrow">Focused Screen</div>
                <h3>Rendered viewport snapshot</h3>
              </div>
              {props.screen ? (
                <div className="meta-stack meta-stack--inline">
                  <Badge {...props.screen.sizeBadge} />
                  <Badge {...props.screen.sequenceBadge} />
                </div>
              ) : null}
            </div>
            {props.screen ? (
              <>
                <div className="screen-meta">
                  {props.screen.meta.map((item) => (
                    <span key={item}>{item}</span>
                  ))}
                </div>
                <pre className="terminal-screen">
                  {props.screen.lines.map((line) => (
                    <div key={line.key} className="terminal-screen__line">
                      <span className="terminal-screen__gutter">{line.gutter}</span>
                      <span>{line.text}</span>
                    </div>
                  ))}
                </pre>
              </>
            ) : (
              <EmptyState text="Focused screen will appear here after the session is attached." />
            )}
          </div>

          <div className="panel panel--surface">
            <div className="panel__header">
              <div>
                <div className="section__eyebrow">Input</div>
                <h3>Command lane</h3>
              </div>
            </div>
            <div className="composer">
              <textarea
                placeholder="Write input for the focused pane"
                value={props.input.draft}
                onChange={(event) => props.commands.input.setDraft(event.target.value)}
              />
              <div className="button-row">
                <button className="button button--primary" onClick={() => void props.commands.input.submit()} disabled={!props.input.canWrite}>
                  Send + Enter
                </button>
                <button className="button" onClick={() => void props.commands.input.sendInterrupt()} disabled={!props.input.canWrite}>
                  Ctrl+C
                </button>
                <button className="button" onClick={() => void props.commands.input.recallHistory()} disabled={!props.input.canWrite}>
                  Arrow Up
                </button>
                <button className="button" onClick={() => void props.commands.input.sendEnter()} disabled={!props.input.canWrite}>
                  Enter
                </button>
              </div>
            </div>
          </div>

          <div className="panel panel--surface">
            <div className="panel__header">
              <div>
                <div className="section__eyebrow">Capabilities</div>
                <h3>Backend truth</h3>
              </div>
            </div>
            {props.capabilities ? (
              <div className="stack-block">
                <div className="capability-cloud">
                  {props.capabilities.badges.map((badge) => (
                    <Badge key={badge.label} {...badge} />
                  ))}
                </div>
                <DegradedReasonList reasons={props.capabilities.degradedReasons} />
              </div>
            ) : (
              <EmptyState text="Select a session to inspect backend capabilities." />
            )}
          </div>
        </section>
      </main>
    </div>
  );
}

function DiscoveredGroup(props: {
  label: string;
  sessions: TerminalWorkspaceDiscoveredSessionModel[];
  emptyText: string;
  onImport(importHandle: string): Promise<void>;
}): ReactElement {
  return (
    <div className="list-stack list-stack--spaced">
      <div className="list-subtitle">{props.label}</div>
      {props.sessions.map((session) => (
        <div key={session.importHandle} className="list-card">
          <div>
            <strong>{session.title}</strong>
            <small>{session.sourceLabel}</small>
          </div>
          <DegradedReasonList reasons={session.degradedReasons} />
          <button className="button" onClick={() => void props.onImport(session.importHandle)}>
            Import
          </button>
        </div>
      ))}
      {props.sessions.length === 0 ? <EmptyState text={props.emptyText} /> : null}
    </div>
  );
}

function PaneTreeNodeView(props: {
  node: TerminalWorkspacePaneTreeNodeModel;
  onFocus(paneId: string): void;
}): ReactElement {
  if (props.node.kind === "leaf") {
    const leaf = props.node;

    return (
      <button
        className={leaf.isFocused ? "pane-leaf pane-leaf--focused" : "pane-leaf"}
        onClick={() => props.onFocus(leaf.paneId)}
      >
        {leaf.label}
      </button>
    );
  }

  return (
    <div className={props.node.direction === "horizontal" ? "pane-split pane-split--horizontal" : "pane-split pane-split--vertical"}>
      <PaneTreeNodeView node={props.node.first} onFocus={props.onFocus} />
      <PaneTreeNodeView node={props.node.second} onFocus={props.onFocus} />
    </div>
  );
}

function Badge(props: TerminalWorkspaceBadgeModel): ReactElement {
  return <span className={`badge badge--${props.tone}`}>{props.label}</span>;
}

function Banner(props: TerminalWorkspaceBannerModel): ReactElement {
  const className = props.tone === "warning"
    ? "banner banner--warning"
    : props.tone === "subtle"
      ? "banner banner--subtle"
      : "banner";

  return (
    <div className={className}>
      {props.title ? <strong>{props.title}</strong> : null}
      <div>{props.detail}</div>
    </div>
  );
}

function DegradedReasonList(props: {
  reasons: TerminalWorkspaceDegradedReasonModel[];
}): ReactElement | null {
  if (props.reasons.length === 0) {
    return null;
  }

  return (
    <div className="degraded-list">
      {props.reasons.map((reason) => (
        <div key={reason.id} className="degraded-list__item">
          <Badge {...reason.badge} />
          <small>{reason.detail}</small>
        </div>
      ))}
    </div>
  );
}

function EmptyState(props: {
  text: string;
}): ReactElement {
  return <div className="empty-state">{props.text}</div>;
}
