import { noopTelemetrySink } from "@terminal-platform/foundation";
import type { PaneHistory } from "@terminal-platform/runtime-types";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";
import { describe, expect, it } from "vitest";

import {
  createInitialWorkspaceSnapshot,
  type WorkspaceDiagnosticRecord,
  type WorkspaceHistoricalPaneSnapshot,
  type WorkspaceSnapshot,
} from "../read-models/workspace-snapshot.js";
import { CatalogService } from "./catalog-service.js";
import type { ServiceContext } from "./service-context.js";
import { SessionCommandService } from "./session-command-service.js";

describe("SessionCommandService pane history paging", () => {
  it("projects raw VT redraws before appending a durable history page", async () => {
    const paneId = "pane-redraw";
    let snapshot = createWorkspaceSnapshot(createHistoricalPane({
      paneId,
      lines: ["existing"],
      nextEventSeq: 2n,
    }));
    const context = {
      ensureTransport: async () => ({
        ...createUnusedTransport(),
        getPaneHistory: async () => createPaneHistory(
          "session-1",
          paneId,
          "p\bprint 'fdfd'\r\nfdfd\r\n",
          {
            fromEventSeq: 2n,
            eventSeqLow: 2n,
            eventSeqHigh: 2n,
          },
        ),
      } as WorkspaceTransportClient),
      getSnapshot: () => snapshot,
      updateSnapshot: (updater) => {
        snapshot = updater(snapshot);
      },
      recordDiagnostic: (input) => ({ ...input, timestampMs: 10_000 }),
      clearDiagnostics: () => {},
      telemetry: noopTelemetrySink,
      now: () => 10_000,
    } satisfies ServiceContext;
    const service = new SessionCommandService(context, new CatalogService(context));

    await expect(service.loadMorePaneHistory(paneId)).resolves.toBe(true);

    expect(snapshot.historicalPanes?.[paneId]?.lines).toEqual([
      "existing",
      "print 'fdfd'",
      "fdfd",
    ]);
  });

  it("reports stale page loads as unapplied", async () => {
    const paneId = "pane-stale";
    const existingHistory = createHistoricalPane({
      paneId,
      lines: ["first page"],
      nextEventSeq: 2n,
    });
    let snapshot = createWorkspaceSnapshot(existingHistory);
    const diagnostics: WorkspaceDiagnosticRecord[] = [];
    const context = {
      ensureTransport: async () => ({
        ...createUnusedTransport(),
        getPaneHistory: async () => createPaneHistory("session-1", paneId, "second page\r\n", {
          fromEventSeq: 2n,
          eventSeqLow: 2n,
          eventSeqHigh: 2n,
        }),
      } as WorkspaceTransportClient),
      getSnapshot: () => snapshot,
      updateSnapshot: (updater) => {
        const staleHistory = createHistoricalPane({
          paneId,
          lines: ["first page"],
          nextEventSeq: 3n,
        });
        snapshot = updater(createWorkspaceSnapshot(staleHistory));
      },
      recordDiagnostic: (input) => {
        const diagnostic = {
          ...input,
          timestampMs: 10_000,
        };
        diagnostics.push(diagnostic);
        return diagnostic;
      },
      clearDiagnostics: () => {
        diagnostics.length = 0;
      },
      telemetry: noopTelemetrySink,
      now: () => 10_000,
    } satisfies ServiceContext;
    const service = new SessionCommandService(context, new CatalogService(context));

    await expect(service.loadMorePaneHistory(paneId)).resolves.toBe(false);

    expect(snapshot.historicalPanes?.[paneId]).toMatchObject({
      lines: ["first page"],
      nextEventSeq: 3n,
    });
    expect(diagnostics).toEqual([]);
  });
});

function createUnusedTransport(): Partial<WorkspaceTransportClient> {
  return {
    close: async () => {},
    discoverSessions: async () => [],
  };
}

function createWorkspaceSnapshot(history: WorkspaceHistoricalPaneSnapshot): WorkspaceSnapshot {
  const base = createInitialWorkspaceSnapshot();
  return {
    ...base,
    selection: {
      activeSessionId: history.sessionId,
      activePaneId: history.paneId,
    },
    historicalPanes: {
      [history.paneId]: history,
    },
  };
}

function createHistoricalPane(
  options: {
    paneId: string;
    lines: string[];
    nextEventSeq: bigint;
  },
): WorkspaceHistoricalPaneSnapshot {
  return {
    sessionId: "session-1",
    paneId: options.paneId,
    sourceSessionId: "session-1",
    sourcePaneId: options.paneId,
    source: "v2_pane_history",
    replayStrategy: "raw_vt_stream",
    restoreGuaranteeLevel: "basic_history",
    lines: options.lines,
    capturedAtMs: 9_000n,
    hasGaps: false,
    hasMoreSegments: true,
    fromEventSeq: 1n,
    nextEventSeq: options.nextEventSeq,
    segmentCount: 1,
    loadedPayloadBytes: 128n,
    streamStartsWithLineBreak: true,
    streamEndsWithLineBreak: true,
  };
}

function createPaneHistory(
  sessionId: string,
  paneId: string,
  text: string,
  options: {
    fromEventSeq: bigint;
    eventSeqLow: bigint;
    eventSeqHigh: bigint;
  },
): PaneHistory {
  const encoded = new TextEncoder().encode(text);
  return {
    session_id: sessionId,
    pane_id: paneId,
    from_event_seq: options.fromEventSeq,
    max_segments: 256n,
    max_bytes: 1_048_576n,
    restore_plan: {
      session_id: sessionId,
      restore_guarantee_level: "basic_history",
      latest_screen_snapshot_id: null,
      latest_topology_snapshot_id: null,
      high_water_commit_seq: options.eventSeqHigh,
      evidence: [{ kind: "stream_segment_count", value: "1" }],
    },
    latest_screen_snapshot: null,
    segments: [
      {
        id: "segment-page",
        event_seq_low: options.eventSeqLow,
        event_seq_high: options.eventSeqHigh,
        byte_low: 0n,
        byte_high: BigInt(encoded.byteLength),
        payload: Array.from(encoded),
        checksum: "checksum",
        capture_semantics: "raw_vt_stream",
        created_at_ms: 10_000n,
      },
    ],
    gaps: [],
    replay_strategy: "raw_vt_stream",
    has_more_segments: false,
    next_event_seq: null,
    total_payload_bytes: BigInt(encoded.byteLength),
  };
}
