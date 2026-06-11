import type { WorkspaceKernel } from "./kernel/types.js";
import type { WorkspaceSnapshot } from "./read-models/workspace-snapshot.js";

export interface WorkspaceTestClock {
  now(): number;
  set(valueMs: number): number;
  advance(deltaMs: number): number;
}

export interface WorkspaceSnapshotRecorder {
  readonly snapshots: readonly WorkspaceSnapshot[];
  unsubscribe(): void;
}

export function createWorkspaceTestClock(initialMs = 0): WorkspaceTestClock {
  let currentMs = normalizeFiniteTimestamp(initialMs, "initialMs");

  return {
    now: () => currentMs,
    set(valueMs) {
      currentMs = normalizeFiniteTimestamp(valueMs, "valueMs");
      return currentMs;
    },
    advance(deltaMs) {
      const delta = normalizeFiniteTimestamp(deltaMs, "deltaMs");
      currentMs += delta;
      return currentMs;
    },
  };
}

export function recordWorkspaceSnapshots(
  kernel: Pick<WorkspaceKernel, "getSnapshot" | "subscribe">,
): WorkspaceSnapshotRecorder {
  const snapshots: WorkspaceSnapshot[] = [kernel.getSnapshot()];
  const unsubscribe = kernel.subscribe(() => {
    snapshots.push(kernel.getSnapshot());
  });

  return {
    snapshots,
    unsubscribe,
  };
}

export async function flushWorkspaceMicrotasks(turns = 1): Promise<void> {
  const safeTurns = Math.max(1, Math.trunc(normalizeFiniteTimestamp(turns, "turns")));

  for (let index = 0; index < safeTurns; index += 1) {
    await Promise.resolve();
  }
}

function normalizeFiniteTimestamp(value: number, label: string): number {
  if (!Number.isFinite(value)) {
    throw new TypeError(`${label} must be a finite number`);
  }

  return value;
}
