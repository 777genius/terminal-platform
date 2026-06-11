import { describe, expect, it } from "vitest";

import { createInitialWorkspaceSnapshot, type WorkspaceSnapshot } from "./index.js";
import {
  createWorkspaceTestClock,
  flushWorkspaceMicrotasks,
  recordWorkspaceSnapshots,
} from "./testing.js";

describe("workspace core testing public subpath", () => {
  it("provides a deterministic clock for kernel tests", () => {
    const clock = createWorkspaceTestClock(100);

    expect(clock.now()).toBe(100);
    expect(clock.advance(25)).toBe(125);
    expect(clock.set(5)).toBe(5);
    expect(() => createWorkspaceTestClock(Number.NaN)).toThrow(TypeError);
  });

  it("records workspace snapshots without owning the kernel lifecycle", () => {
    const kernel = new MinimalSnapshotKernel();
    const recorder = recordWorkspaceSnapshots(kernel);

    kernel.publish(createInitialWorkspaceSnapshot({ commandHistoryEntries: ["pwd"] }));

    expect(recorder.snapshots).toHaveLength(2);
    expect(recorder.snapshots[1]?.commandHistory.entries).toEqual(["pwd"]);

    recorder.unsubscribe();
    kernel.publish(createInitialWorkspaceSnapshot({ commandHistoryEntries: ["git status"] }));

    expect(recorder.snapshots).toHaveLength(2);
  });

  it("flushes a bounded number of microtask turns", async () => {
    let flushed = false;
    Promise.resolve().then(() => {
      flushed = true;
    });

    await flushWorkspaceMicrotasks();

    expect(flushed).toBe(true);
  });
});

class MinimalSnapshotKernel {
  #snapshot = createInitialWorkspaceSnapshot();
  #listeners = new Set<() => void>();

  getSnapshot(): WorkspaceSnapshot {
    return this.#snapshot;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => {
      this.#listeners.delete(listener);
    };
  }

  publish(snapshot: WorkspaceSnapshot): void {
    this.#snapshot = snapshot;
    for (const listener of this.#listeners) {
      listener();
    }
  }
}
