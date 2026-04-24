import type { ReactElement } from "react";
import type { TerminalWorkspaceCatalogCommands } from "../commands/TerminalWorkspaceCatalogCommands.js";
import type {
  TerminalWorkspaceCatalogBadgeModel,
  TerminalWorkspaceCatalogDegradedReasonModel,
  TerminalWorkspaceCatalogDiscoveredSessionModel,
  TerminalWorkspaceCatalogModel,
} from "../view-models/TerminalWorkspaceCatalogModel.js";

export function TerminalWorkspaceCatalogSidebarView(props: {
  model: TerminalWorkspaceCatalogModel;
  commands: TerminalWorkspaceCatalogCommands;
}): ReactElement {
  return (
    <>
      <section className="section">
        <div className="section__eyebrow">Gateway</div>
        <h1 className="section__title">Terminal Platform Demo</h1>
        <div className="meta-stack">
          <Badge {...props.model.statusBadge} />
          <Badge {...props.model.sessionStatusBadge} />
          <Badge {...props.model.sessionStreamBadge} />
        </div>
        <p className="section__copy">Renderer talks only to a local WebSocket gateway. Electron stays a leaf host.</p>
        <dl className="definition-list">
          {props.model.gatewayInfo.map((item) => (
            <div key={item.label}>
              <dt>{item.label}</dt>
              <dd>{item.value}</dd>
            </div>
          ))}
        </dl>
        <DegradedReasonList reasons={props.model.handshakeDegradedReasons} />
      </section>

      <section className="section">
        <div className="section__eyebrow">Create Native</div>
        <div className="form-grid">
          <label>
            <span>Title</span>
            <input name="terminal-catalog-title" value={props.model.createForm.title} onChange={(event) => props.commands.setTitle(event.target.value)} />
          </label>
          <label>
            <span>Program</span>
            <input name="terminal-catalog-program" placeholder="optional custom shell" value={props.model.createForm.program} onChange={(event) => props.commands.setProgram(event.target.value)} />
          </label>
          <label>
            <span>Args</span>
            <input name="terminal-catalog-args" placeholder='for example -l "-c echo demo"' value={props.model.createForm.args} onChange={(event) => props.commands.setArgs(event.target.value)} />
          </label>
          <label>
            <span>Cwd</span>
            <input name="terminal-catalog-cwd" placeholder="optional working directory" value={props.model.createForm.cwd} onChange={(event) => props.commands.setCwd(event.target.value)} />
          </label>
        </div>
        <button className="button button--primary" onClick={() => void props.commands.submitCreate()}>
          Create Native Session
        </button>
      </section>

      <section className="section">
        <div className="section__eyebrow">Sessions</div>
        <div className="list-stack">
          {props.model.sessionItems.map((session) => (
            <button
              key={session.sessionId}
              className={session.isActive ? "list-item list-item--active" : "list-item"}
              onClick={() => void props.commands.selectSession(session.sessionId)}
            >
              <span>{session.title}</span>
              <small>{session.meta}</small>
            </button>
          ))}
          {props.model.sessionItems.length === 0 ? <EmptyState text="No attached sessions yet." /> : null}
        </div>
      </section>

      <section className="section">
        <div className="section__eyebrow">Foreign Backends</div>
        {props.model.discoveredGroups.map((group) => (
          <DiscoveredGroup
            key={group.key}
            label={group.label}
            sessions={group.sessions}
            emptyText={group.emptyText}
            onImport={(importHandle) => props.commands.importSession(importHandle)}
          />
        ))}
      </section>
    </>
  );
}

function DiscoveredGroup(props: {
  label: string;
  sessions: TerminalWorkspaceCatalogDiscoveredSessionModel[];
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

function Badge(props: TerminalWorkspaceCatalogBadgeModel): ReactElement {
  return <span className={`badge badge--${props.tone}`}>{props.label}</span>;
}

function DegradedReasonList(props: {
  reasons: TerminalWorkspaceCatalogDegradedReasonModel[];
}): ReactElement | null {
  if (props.reasons.length === 0) {
    return null;
  }

  return (
    <div className="degraded-list">
      {props.reasons.map((reason) => (
        <div key={reason.id} className="degraded-list__item">
          <Badge {...reason.badge} />
          <span>{reason.detail}</span>
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
