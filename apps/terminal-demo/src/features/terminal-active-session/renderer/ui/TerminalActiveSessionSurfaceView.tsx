import type { ReactElement, ReactNode } from "react";
import type { TerminalActiveSessionCommands } from "../commands/TerminalActiveSessionCommands.js";
import type {
  TerminalActiveSessionBadgeModel,
  TerminalActiveSessionBannerModel,
  TerminalActiveSessionDegradedReasonModel,
  TerminalActiveSessionModel,
  TerminalActiveSessionPaneTreeNodeModel,
} from "../view-models/TerminalActiveSessionModel.js";

export function TerminalActiveSessionSurfaceView(props: {
  model: TerminalActiveSessionModel;
  commands: TerminalActiveSessionCommands;
  inputPanel?: ReactNode;
}): ReactElement {
  return (
    <>
      <section className="hero panel">
        <div>
          <div className="section__eyebrow">Session Surface</div>
          <h2 className="hero__title">{props.model.activeSessionTitle}</h2>
          <p className="hero__copy">
            Topology, focus, screen projection and mux commands come from the Rust daemon over the gateway.
          </p>
        </div>
        <div className="button-row">
          <button className="button" onClick={() => void props.commands.refreshCatalog()}>
            Refresh Catalog
          </button>
          <button className="button" onClick={() => void props.commands.newTab()} disabled={!props.model.toolbar.canNewTab}>
            New Tab
          </button>
          <button className="button" onClick={() => void props.commands.splitHorizontal()} disabled={!props.model.toolbar.canSplit}>
            Split Row
          </button>
          <button className="button" onClick={() => void props.commands.splitVertical()} disabled={!props.model.toolbar.canSplit}>
            Split Column
          </button>
          <button className="button button--primary" onClick={() => void props.commands.saveSession()} disabled={!props.model.toolbar.canSave}>
            Save Session
          </button>
        </div>
      </section>

      {props.model.errorBanner ? <Banner {...props.model.errorBanner} /> : null}
      {props.model.sessionStreamBanner ? <Banner {...props.model.sessionStreamBanner} /> : null}
      {props.model.actionDegradedBanner ? <Banner {...props.model.actionDegradedBanner} /> : null}
      {props.model.actionErrorBanner ? <Banner {...props.model.actionErrorBanner} /> : null}

      <section className="workspace-grid">
        <div className="panel panel--surface">
          <div className="panel__header">
            <div>
              <div className="section__eyebrow">Topology</div>
              <h3>Tabs and pane tree</h3>
            </div>
            {props.model.activeBackendBadge ? <Badge {...props.model.activeBackendBadge} /> : null}
          </div>
          {props.model.topologyTabs.length > 0 ? (
            <>
              <div className="tab-strip">
                {props.model.topologyTabs.map((tab) => (
                  <button
                    key={tab.tabId}
                    className={tab.isFocused ? "tab-pill tab-pill--active" : "tab-pill"}
                    onClick={() => void props.commands.focusTab(tab.tabId)}
                  >
                    {tab.title}
                  </button>
                ))}
              </div>
              <div className="pane-tree-grid">
                {props.model.topologyTabs.map((tab) => (
                  <div key={tab.tabId} className="pane-tree-card">
                    <div className="pane-tree-card__title">{tab.title}</div>
                    <PaneTreeNodeView node={tab.root} onFocus={(paneId) => void props.commands.focusPane(paneId)} />
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
            {props.model.screen ? (
              <div className="meta-stack meta-stack--inline">
                <Badge {...props.model.screen.sizeBadge} />
                <Badge {...props.model.screen.sequenceBadge} />
              </div>
            ) : null}
          </div>
          {props.model.screen ? (
            <>
              <div className="screen-meta">
                {props.model.screen.meta.map((item) => (
                  <span key={item}>{item}</span>
                ))}
              </div>
              <pre className="terminal-screen">
                {props.model.screen.lines.map((line) => (
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

        {props.inputPanel}

        <div className="panel panel--surface">
          <div className="panel__header">
            <div>
              <div className="section__eyebrow">Capabilities</div>
              <h3>Backend truth markers</h3>
            </div>
          </div>
          {props.model.capabilities ? (
            <>
              <div className="meta-stack meta-stack--wrap">
                {props.model.capabilities.badges.map((badge) => (
                  <Badge key={badge.label} {...badge} />
                ))}
              </div>
              <DegradedReasonList reasons={props.model.capabilities.degradedReasons} />
            </>
          ) : (
            <EmptyState text="Select a session to inspect its backend capability truth." />
          )}
        </div>
      </section>
    </>
  );
}

function PaneTreeNodeView(props: {
  node: TerminalActiveSessionPaneTreeNodeModel;
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

function Badge(props: TerminalActiveSessionBadgeModel): ReactElement {
  return <span className={`badge badge--${props.tone}`}>{props.label}</span>;
}

function Banner(props: TerminalActiveSessionBannerModel): ReactElement {
  const className = props.tone === "warning"
    ? "banner banner--warning"
    : props.tone === "subtle"
      ? "banner banner--subtle"
      : "banner";

  return (
    <section className={className}>
      {props.title ? <strong>{props.title}</strong> : null}
      <span>{props.detail}</span>
    </section>
  );
}

function DegradedReasonList(props: {
  reasons: TerminalActiveSessionDegradedReasonModel[];
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
