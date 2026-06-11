import { describe, expect, it } from "vitest";

import {
  createWorkspaceTestHarness,
  type WorkspaceTestHarness,
} from "./index.js";
import type { WorkspaceKernel } from "@terminal-platform/workspace-core";
import type { WorkspaceHost } from "@terminal-platform/workspace-core/bootstrap";
import type { WorkspaceTransportClient } from "@terminal-platform/workspace-contracts";

type Assert<T extends true> = T;
type Equal<Actual, Expected> = (<T>() => T extends Actual ? 1 : 2) extends
  <T>() => T extends Expected ? 1 : 2
  ? true
  : false;

type _HarnessKernelIsPublicKernel = Assert<Equal<WorkspaceTestHarness["kernel"], WorkspaceKernel>>;
type _HarnessHostIsPublicHost = Assert<Equal<WorkspaceTestHarness["host"], WorkspaceHost>>;
type _HarnessTransportIsPublicTransport = Assert<
  Equal<WorkspaceTestHarness["transport"], WorkspaceTransportClient>
>;

describe("createWorkspaceTestHarness", () => {
  it("bootstraps kernel state from the memory transport", async () => {
    const harness = createWorkspaceTestHarness();

    await expect(harness.bootstrap()).resolves.toBe(harness.kernel);

    const snapshot = harness.kernel.getSnapshot();
    expect(snapshot.connection.state).toBe("ready");
    expect(snapshot.catalog.sessions).toHaveLength(1);
    expect(snapshot.catalog.savedSessions).toHaveLength(1);

    await harness.dispose();
  });

  it("attaches the active session and exposes its focused screen", async () => {
    const harness = createWorkspaceTestHarness();

    await harness.bootstrap();
    const sessionId = harness.kernel.getSnapshot().catalog.sessions[0]?.session_id;
    expect(sessionId).toBeTruthy();

    await harness.kernel.commands.attachSession(sessionId!);

    const snapshot = harness.kernel.getSnapshot();
    expect(snapshot.attachedSession?.session.session_id).toBe(sessionId);
    expect(snapshot.attachedSession?.focused_screen?.surface.lines[0]?.text).toBe("ready");

    await harness.dispose();
  });

  it("exposes the underlying host and fake transport for conformance-style tests", async () => {
    const harness = createWorkspaceTestHarness({
      autoBootstrap: true,
    });

    const [firstKernel, secondKernel] = await Promise.all([
      harness.bootstrap(),
      harness.host.bootstrap(),
    ]);

    expect(firstKernel).toBe(harness.kernel);
    expect(secondKernel).toBe(harness.kernel);
    expect(await harness.transport.listSessions()).toHaveLength(1);
    expect(harness.kernel.getSnapshot().connection.state).toBe("ready");

    await harness.dispose();
  });
});
