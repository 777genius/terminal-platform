# Vanilla WebSocket Terminal

Production integration sample for host apps that want Terminal Platform custom elements with a WebSocket gateway.

This sample is intentionally downstream-only:

- it imports published SDK entrypoints only
- it owns gateway URLs and lifecycle in host code
- it does not import `apps/terminal-demo` or package internals

## Run

Use any browser bundler that can serve TypeScript ESM, for example Vite in a consumer app.

```sh
npm install \
  @terminal-platform/design-tokens \
  @terminal-platform/workspace-adapter-websocket \
  @terminal-platform/workspace-core \
  @terminal-platform/workspace-elements
```

Then serve this directory and point it at a running Terminal Platform gateway:

```text
/index.html?controlUrl=ws://127.0.0.1:34115/workspace/control
```

Optional stream override:

```text
/index.html?controlUrl=ws://127.0.0.1:34115/workspace/control&streamUrl=ws://127.0.0.1:34115/workspace/stream
```

The default `streamUrl` is derived by the WebSocket adapter from `controlUrl`.
