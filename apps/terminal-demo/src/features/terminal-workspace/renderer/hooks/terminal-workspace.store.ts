import { create } from "zustand";
import {
  initialTerminalWorkspaceViewState,
  type TerminalWorkspaceViewState,
} from "../../core/application/index.js";

export interface TerminalWorkspaceStoreState extends TerminalWorkspaceViewState {
  setCreateField(field: CreateFieldName, value: string): void;
  setInputDraft(value: string): void;
  reset(): void;
}

type CreateFieldName =
  | "createTitleDraft"
  | "createProgramDraft"
  | "createArgsDraft"
  | "createCwdDraft";

export const useTerminalWorkspaceStore = create<TerminalWorkspaceStoreState>((set) => ({
  ...initialTerminalWorkspaceViewState,
  setCreateField: (field, value) => {
    set({ [field]: value } as Pick<TerminalWorkspaceViewState, CreateFieldName>);
  },
  setInputDraft: (value) => {
    set({ inputDraft: value });
  },
  reset: () => {
    set(initialTerminalWorkspaceViewState);
  },
}));
