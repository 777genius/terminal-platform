const { contextBridge } = require("electron") as typeof import("electron");

type TerminalDemoBootstrapConfig =
  import("../contracts/bootstrap-config.js").TerminalDemoBootstrapConfig;

const config = readBootstrapConfig();

contextBridge.exposeInMainWorld("terminalDemo", {
  config,
});

function readBootstrapConfig(): TerminalDemoBootstrapConfig {
  const rawArgument = process.argv.find((argument) =>
    argument.startsWith("--terminal-demo-config="),
  );

  if (!rawArgument) {
    throw new Error("Missing terminal demo bootstrap config");
  }

  return JSON.parse(
    decodeURIComponent(rawArgument.slice("--terminal-demo-config=".length)),
  ) as TerminalDemoBootstrapConfig;
}
