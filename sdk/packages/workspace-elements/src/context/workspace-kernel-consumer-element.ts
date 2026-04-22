import { LitElement, type PropertyValues } from "lit";

import {
  createInitialWorkspaceSnapshot,
  type WorkspaceKernel,
  type WorkspaceSnapshot,
} from "@terminal-platform/workspace-core";

export abstract class WorkspaceKernelConsumerElement extends LitElement {
  static properties = {
    kernel: { attribute: false },
    snapshot: { state: true },
  };

  declare kernel: WorkspaceKernel | null;
  protected declare snapshot: WorkspaceSnapshot;

  #unsubscribe: (() => void) | null = null;

  constructor() {
    super();
    this.kernel = null;
    this.snapshot = createInitialWorkspaceSnapshot();
  }

  override connectedCallback(): void {
    super.connectedCallback();
    this.syncKernelSubscription();
  }

  override disconnectedCallback(): void {
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    super.disconnectedCallback();
  }

  protected override willUpdate(changedProperties: PropertyValues<this>): void {
    if (changedProperties.has("kernel")) {
      this.syncKernelSubscription();
    }
  }

  protected syncKernelSubscription(): void {
    this.#unsubscribe?.();
    this.#unsubscribe = null;

    if (!this.kernel) {
      this.snapshot = createInitialWorkspaceSnapshot();
      return;
    }

    this.snapshot = this.kernel.getSnapshot();
    this.#unsubscribe = this.kernel.subscribe(() => {
      if (!this.kernel) {
        return;
      }

      this.snapshot = this.kernel.getSnapshot();
    });
  }
}
