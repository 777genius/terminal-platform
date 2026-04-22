import { app, BrowserWindow } from "electron";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import {
  DEFAULT_TERMINAL_RUNTIME_SLUG,
  startTerminalRuntimeHost,
  type TerminalRuntimeHostHandle,
} from "@features/terminal-runtime-host/main";
import { createMainWindow } from "./createMainWindow.js";

const runtimeSlug = process.env.TERMINAL_DEMO_RUNTIME_SLUG ?? DEFAULT_TERMINAL_RUNTIME_SLUG;
let hostHandle: TerminalRuntimeHostHandle | null = null;

async function bootstrap(): Promise<void> {
  await app.whenReady();

  hostHandle = await startTerminalRuntimeHost({ runtimeSlug });
  const config: TerminalRuntimeBootstrapConfig = {
    controlPlaneUrl: hostHandle.controlPlaneUrl,
    sessionStreamUrl: hostHandle.sessionStreamUrl,
    runtimeSlug: hostHandle.runtimeSlug,
  };

  await createMainWindow(config);

  app.on("activate", async () => {
    if (app.isReady() && BrowserWindow.getAllWindows().length === 0 && hostHandle) {
      await createMainWindow({
        controlPlaneUrl: hostHandle.controlPlaneUrl,
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
