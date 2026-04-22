import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const appRoot = path.dirname(fileURLToPath(import.meta.url));
const rendererPort = Number(process.env.TERMINAL_DEMO_RENDERER_PORT ?? "5173");

export default defineConfig({
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      "@features": path.resolve(appRoot, "src/features"),
    },
  },
  build: {
    outDir: path.resolve(appRoot, "dist/renderer"),
    emptyOutDir: false,
  },
  server: {
    host: "127.0.0.1",
    port: rendererPort,
    strictPort: true,
  },
});
