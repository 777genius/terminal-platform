import type { ReactElement } from "react";
import type { TerminalSavedSessionsCommands } from "../commands/TerminalSavedSessionsCommands.js";
import type {
  TerminalSavedSessionsBadgeModel,
  TerminalSavedSessionsDegradedReasonModel,
  TerminalSavedSessionsModel,
} from "../view-models/TerminalSavedSessionsModel.js";

export function TerminalSavedSessionsSectionView(props: {
  model: TerminalSavedSessionsModel;
  commands: TerminalSavedSessionsCommands;
}): ReactElement {
  return (
    <section className="section section--saved">
      <div className="section__header">
        <div className="section__eyebrow">Saved Sessions</div>
        {props.model.savedSessionItems.length > 0 ? (
          <span className="section__meta">{props.model.savedSessionItems.length + props.model.hiddenSavedSessionsCount}</span>
        ) : null}
      </div>
      <div className="list-stack list-stack--scroll">
        {props.model.savedSessionItems.map((saved) => (
          <div key={saved.sessionId} className="list-card list-card--saved">
            <div>
              <strong>{saved.title}</strong>
              <small>{saved.meta}</small>
            </div>
            <DegradedReasonList reasons={saved.degradedReasons} />
            <div className="button-row button-row--compact">
              <button className="button" onClick={() => void props.commands.restore(saved.sessionId)} disabled={!saved.canRestore}>
                Restore
              </button>
              <button className="button button--danger" onClick={() => void props.commands.delete(saved.sessionId)}>
                Delete
              </button>
            </div>
          </div>
        ))}
        {props.model.savedSessionItems.length === 0 ? <EmptyState text="No saved sessions yet." /> : null}
      </div>
      {props.model.hiddenSavedSessionsCount > 0 ? (
        <button className="button button--compact" onClick={() => props.commands.toggleVisibility()}>
          {props.model.showAllSavedSessions
            ? "Show recent only"
            : `Show ${props.model.hiddenSavedSessionsCount} more`}
        </button>
      ) : null}
    </section>
  );
}

function Badge(props: TerminalSavedSessionsBadgeModel): ReactElement {
  return <span className={`badge badge--${props.tone}`}>{props.label}</span>;
}

function DegradedReasonList(props: {
  reasons: TerminalSavedSessionsDegradedReasonModel[];
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
