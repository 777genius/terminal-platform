import test from "node:test";
import assert from "node:assert/strict";

import {
  disposeTerminalRuntimeHostResources,
  startTerminalRuntimeHostWithDependencies,
} from "../dist/features/terminal-runtime-host/main/composition/startTerminalRuntimeHost.js";

test("runtime host startup disposes daemon when initial native session creation fails", async () => {
  const calls = [];
  const createError = new Error("create native failed");
  const daemonSupervisor = {
    async ensureRunning() {
      calls.push("daemon.ensure");
    },
    async dispose() {
      calls.push("daemon.dispose");
    },
  };
  const client = {
    async listSessions() {
      calls.push("client.listSessions");
      return [];
    },
    async createNativeSession() {
      calls.push("client.createNativeSession");
      throw createError;
    },
  };

  await assert.rejects(
    () => startTerminalRuntimeHostWithDependencies({
      initialNativeSession: {
        program: "cmd.exe",
        cwd: "C:\\work",
      },
      runtimeSlug: "terminal-demo-test",
    }, {
      createClientProvider: () => ({
        async getClient() {
          return client;
        },
      }),
      daemonSupervisor,
      async startGateway() {
        calls.push("gateway.start");
        throw new Error("gateway should not start");
      },
    }),
    createError,
  );

  assert.deepEqual(calls, [
    "daemon.ensure",
    "client.listSessions",
    "client.createNativeSession",
    "daemon.dispose",
  ]);
});

test("runtime host dispose attempts daemon cleanup even when gateway dispose fails", async () => {
  const calls = [];
  const gatewayError = new Error("gateway dispose failed");

  await assert.rejects(
    () => disposeTerminalRuntimeHostResources({
      gatewayServer: {
        controlPlaneUrl: "ws://127.0.0.1:1/control",
        runtimeSlug: "terminal-demo-test",
        sessionStreamUrl: "ws://127.0.0.1:1/stream",
        async dispose() {
          calls.push("gateway.dispose");
          throw gatewayError;
        },
      },
      daemonSupervisor: {
        async ensureRunning() {},
        async dispose() {
          calls.push("daemon.dispose");
        },
      },
    }),
    gatewayError,
  );

  assert.deepEqual(calls, [
    "gateway.dispose",
    "daemon.dispose",
  ]);
});
