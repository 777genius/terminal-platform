import type { TerminalRuntimeWorkspaceFacade } from "@features/terminal-workspace-kernel/contracts";
import type { TerminalInputComposerModel } from "../view-models/TerminalInputComposerModel.js";

export function createTerminalInputComposerModel(input: {
  runtime: TerminalRuntimeWorkspaceFacade;
  draft: string;
}): TerminalInputComposerModel {
  const state = input.runtime.state;
  const activeSession = state.sessions.find((session) => session.session_id === state.activeSessionId) ?? null;
  const activeCapabilities = activeSession
    ? state.capabilities[activeSession.origin.backend] ?? null
    : null;

  return {
    draft: input.draft,
    canWrite: Boolean(state.activeSessionId && activeCapabilities?.capabilities.pane_input_write),
  };
}
