import { describe, expect, it } from "vitest";

import { createWorkspaceTestHarness } from "./index.js";

describe("createWorkspaceTestHarness", () => {
  it("bootstraps kernel state from the memory transport", async () => {
    const harness = createWorkspaceTestHarness();

    await harness.kernel.bootstrap();

    const snapshot = harness.kernel.getSnapshot();
    expect(snapshot.connection.state).toBe("ready");
    expect(snapshot.catalog.sessions).toHaveLength(1);
    expect(snapshot.catalog.savedSessions).toHaveLength(1);

    await harness.dispose();
  });

  it("attaches the active session and exposes its focused screen", async () => {
    const harness = createWorkspaceTestHarness();

    await harness.kernel.bootstrap();
    const sessionId = harness.kernel.getSnapshot().catalog.sessions[0]?.session_id;
    expect(sessionId).toBeTruthy();

    await harness.kernel.commands.attachSession(sessionId!);

    const snapshot = harness.kernel.getSnapshot();
    expect(snapshot.attachedSession?.session.session_id).toBe(sessionId);
    expect(snapshot.attachedSession?.focused_screen?.surface.lines[0]?.text).toBe("ready");

    await harness.dispose();
  });
});
