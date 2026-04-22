import type { Disposable, DisposeCallback } from "./disposable.js";
import { toDisposable } from "./disposable.js";

export class ResourceScope implements Disposable {
  #resources = new Set<Disposable>();
  #disposed = false;

  add(resource: Disposable | DisposeCallback): Disposable {
    const disposable =
      typeof resource === "function" ? toDisposable(resource) : resource;

    if (this.#disposed) {
      void disposable.dispose();
      return disposable;
    }

    this.#resources.add(disposable);
    return disposable;
  }

  async dispose(): Promise<void> {
    if (this.#disposed) {
      return;
    }

    this.#disposed = true;
    const resources = Array.from(this.#resources).reverse();
    this.#resources.clear();

    for (const resource of resources) {
      await resource.dispose();
    }
  }
}
