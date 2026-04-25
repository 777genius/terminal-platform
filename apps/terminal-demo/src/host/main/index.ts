import { app, BrowserWindow } from "electron";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  resolveDemoDefaultShellProgram,
  startTerminalRuntimeHost,
  type TerminalRuntimeHostHandle,
} from "@features/terminal-runtime-host/main";
import { createMainWindow } from "./createMainWindow.js";

const runtimeSlug = process.env.TERMINAL_DEMO_RUNTIME_SLUG ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
const sessionStorePath = process.env.TERMINAL_DEMO_SESSION_STORE_PATH ?? null;
const demoAutoStartSession = process.env.TERMINAL_DEMO_AUTO_START_SESSION === "1";
const demoDefaultShellProgram = resolveDemoDefaultShellProgram();
let hostHandle: TerminalRuntimeHostHandle | null = null;

async function bootstrap(): Promise<void> {
  await app.whenReady();

  hostHandle = await startTerminalRuntimeHost({
    runtimeSlug,
    forceRestartReadyDaemon: true,
    initialNativeSession: demoAutoStartSession
      ? {
          title: "SDK Workspace",
          program: demoDefaultShellProgram,
        }
      : null,
    sessionStorePath,
  });
  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl: hostHandle.controlPlaneUrl,
    demoDefaultShellProgram,
    sessionStreamUrl: hostHandle.sessionStreamUrl,
    runtimeSlug: hostHandle.runtimeSlug,
  };

  await createMainWindow(config);

  app.on("activate", async () => {
    if (app.isReady() && BrowserWindow.getAllWindows().length === 0 && hostHandle) {
      await createMainWindow({
        controlPlaneUrl: hostHandle.controlPlaneUrl,
        demoDefaultShellProgram,
        sessionStreamUrl: hostHandle.sessionStreamUrl,
        runtimeSlug: hostHandle.runtimeSlug,
      });
    }
  });
}

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

app.on("before-quit", () => {
  void hostHandle?.dispose();
});

void bootstrap().catch((error) => {
  console.error(error);
  app.exit(1);
});
