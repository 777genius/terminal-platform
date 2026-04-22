import path from "node:path";
import { fileURLToPath } from "node:url";
import { BrowserWindow } from "electron";
import type { TerminalRuntimeBootstrapConfig } from "@features/terminal-runtime-host/contracts";
import { resolveTerminalRuntimePreloadPath } from "@features/terminal-runtime-host/preload";

const moduleDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(moduleDir, "../../../");
const rendererDistPath = path.resolve(appRoot, "dist/renderer/index.html");

export async function createMainWindow(
  config: TerminalRuntimeBootstrapConfig,
): Promise<BrowserWindow> {
  const window = new BrowserWindow({
    width: 1580,
    height: 980,
    minWidth: 1180,
    minHeight: 760,
    backgroundColor: "#0c1118",
    show: false,
    title: "Terminal Platform Demo",
    webPreferences: {
      preload: resolveTerminalRuntimePreloadPath(),
      contextIsolation: true,
      nodeIntegration: false,
      additionalArguments: [
        `--terminal-demo-config=${encodeURIComponent(JSON.stringify(config))}`,
      ],
    },
  });

  window.once("ready-to-show", () => {
    window.show();
  });

  const rendererUrl = process.env.TERMINAL_DEMO_RENDERER_URL;
  if (rendererUrl) {
    await window.loadURL(rendererUrl);
    window.webContents.openDevTools({ mode: "detach" });
  } else {
    await window.loadFile(rendererDistPath);
  }

  return window;
}
