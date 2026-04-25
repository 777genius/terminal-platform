import { describe, expect, it } from "vitest";

import { compactTerminalId, resolveTerminalEntityIdLabel } from "./terminal-identity.js";

describe("terminal identity presentation", () => {
  it("keeps short ids readable without adding ellipsis", () => {
    expect(compactTerminalId("pane-1")).toBe("pane-1");
    expect(resolveTerminalEntityIdLabel("pane-1", { prefix: "Pane" })).toEqual({
      label: "Pane pane-1",
      title: "pane-1",
      isCompact: false,
    });
  });

  it("compacts long ids while preserving the full title", () => {
    const fullId = "d5bcf588-f6ba-46f9-a9b2-d77e6f7258cd";

    expect(compactTerminalId(fullId)).toBe("d5bcf588...7258cd");
    expect(resolveTerminalEntityIdLabel(fullId, { prefix: "Pane" })).toEqual({
      label: "Pane d5bcf588...7258cd",
      title: fullId,
      isCompact: true,
    });
  });
});
