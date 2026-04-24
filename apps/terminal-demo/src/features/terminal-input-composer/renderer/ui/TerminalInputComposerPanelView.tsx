import type { ReactElement } from "react";
import type { TerminalInputComposerCommands } from "../commands/TerminalInputComposerCommands.js";
import type { TerminalInputComposerModel } from "../view-models/TerminalInputComposerModel.js";

export function TerminalInputComposerPanelView(props: {
  model: TerminalInputComposerModel;
  commands: TerminalInputComposerCommands;
}): ReactElement {
  return (
    <div className="panel panel--surface">
      <div className="panel__header">
        <div>
          <div className="section__eyebrow">Input</div>
          <h3>Command lane</h3>
        </div>
      </div>
      <div className="composer">
        <textarea
          name="terminal-input-composer-draft"
          placeholder="Write input for the focused pane"
          value={props.model.draft}
          onChange={(event) => props.commands.setDraft(event.target.value)}
        />
        <div className="button-row">
          <button className="button button--primary" onClick={() => void props.commands.submit()} disabled={!props.model.canWrite}>
            Send + Enter
          </button>
          <button className="button" onClick={() => void props.commands.sendInterrupt()} disabled={!props.model.canWrite}>
            Ctrl+C
          </button>
          <button className="button" onClick={() => void props.commands.recallHistory()} disabled={!props.model.canWrite}>
            Arrow Up
          </button>
          <button className="button" onClick={() => void props.commands.sendEnter()} disabled={!props.model.canWrite}>
            Enter
          </button>
        </div>
      </div>
    </div>
  );
}
