import { describe, expect, it } from "vitest";

import type { CreateWorkspaceSessionInput } from "./commands.js";
import { WorkspaceError, toWorkspaceError, type WorkspaceErrorShape } from "./errors.js";
import type { WorkspaceObservation } from "./observations.js";
import type { WorkspaceTransportClient } from "./ports.js";

describe("workspace contracts public subpath entrypoints", () => {
  it("exposes errors as the only runtime contract helper subpath", () => {
    const fallback: WorkspaceErrorShape = {
      code: "transport_failed",
      message: "transport failed",
      recoverable: true,
    };

    expect(toWorkspaceError(new Error("socket closed"), fallback)).toMatchObject({
      code: "transport_failed",
      message: "socket closed",
      recoverable: true,
    });
    expect(new WorkspaceError(fallback)).toBeInstanceOf(Error);
  });

  it("keeps command, observation, and port subpaths type-only", () => {
    assertContractTypesAreImportable(null as never);
  });
});

function assertContractTypesAreImportable(
  _value:
    | CreateWorkspaceSessionInput
    | WorkspaceObservation
    | WorkspaceTransportClient,
): void {}
