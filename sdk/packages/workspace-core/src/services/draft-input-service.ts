import type { PaneId } from "@terminal-platform/runtime-types";

import type { ServiceContext } from "./service-context.js";

export class DraftInputService {
  readonly #context: Pick<ServiceContext, "updateSnapshot">;

  constructor(context: Pick<ServiceContext, "updateSnapshot">) {
    this.#context = context;
  }

  updateDraft(paneId: PaneId, value: string): void {
    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      drafts: {
        ...snapshot.drafts,
        [paneId]: value,
      },
    }));
  }

  clearDraft(paneId: PaneId): void {
    this.#context.updateSnapshot((snapshot) => {
      const drafts = { ...snapshot.drafts };
      delete drafts[paneId];

      return {
        ...snapshot,
        drafts,
      };
    });
  }
}
