import { LitElement, type PropertyValues } from "lit";

import { TERMINAL_PLATFORM_THEME_ATTRIBUTE } from "@terminal-platform/design-tokens";
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
    this.syncThemeAttribute();
  }

  override disconnectedCallback(): void {
    this.#unsubscribe?.();
    this.#unsubscribe = null;
    super.disconnectedCallback();
  }

  protected override willUpdate(changedProperties: PropertyValues): void {
    if (changedProperties.has("kernel")) {
      this.syncKernelSubscription();
    }

    if (changedProperties.has("snapshot")) {
      this.syncThemeAttribute();
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

  protected syncThemeAttribute(): void {
    this.setAttribute(TERMINAL_PLATFORM_THEME_ATTRIBUTE, this.snapshot.theme.themeId);
  }
}
