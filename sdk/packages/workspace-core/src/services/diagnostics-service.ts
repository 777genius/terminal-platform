import type { ServiceContext } from "./service-context.js";

import type { WorkspaceDiagnosticRecord } from "../read-models/workspace-snapshot.js";

export class DiagnosticsService {
  readonly #context: Pick<
    ServiceContext,
    "clearDiagnostics" | "getSnapshot" | "now" | "telemetry" | "updateSnapshot"
  >;

  constructor(context: Pick<ServiceContext, "clearDiagnostics" | "getSnapshot" | "now" | "telemetry" | "updateSnapshot">) {
    this.#context = context;
  }

  list(): WorkspaceDiagnosticRecord[] {
    return this.#context.getSnapshot().diagnostics;
  }

  record(input: Omit<WorkspaceDiagnosticRecord, "timestampMs">): WorkspaceDiagnosticRecord {
    const record: WorkspaceDiagnosticRecord = {
      ...input,
      timestampMs: this.#context.now(),
    };

    this.#context.updateSnapshot((snapshot) => ({
      ...snapshot,
      diagnostics: [...snapshot.diagnostics, record],
    }));

    this.#context.telemetry.emit({
      name: "workspace.diagnostic.recorded",
      attributes: {
        code: record.code,
        severity: record.severity,
        recoverable: record.recoverable,
      },
    });

    return record;
  }

  clear(): void {
    this.#context.clearDiagnostics();
  }
}
