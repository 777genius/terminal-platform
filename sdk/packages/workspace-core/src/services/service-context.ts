import type { TelemetrySink } from "@terminal-platform/foundation";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

import type {
  WorkspaceDiagnosticRecord,
  WorkspaceSnapshot,
} from "../read-models/workspace-snapshot.js";

export interface ServiceContext {
  ensureTransport(): Promise<WorkspaceTransportClient>;
  getSnapshot(): WorkspaceSnapshot;
  updateSnapshot(updater: (snapshot: WorkspaceSnapshot) => WorkspaceSnapshot): void;
  recordDiagnostic(
    input: Omit<WorkspaceDiagnosticRecord, "timestampMs">,
  ): WorkspaceDiagnosticRecord;
  clearDiagnostics(): void;
  telemetry: TelemetrySink;
  now: () => number;
}
