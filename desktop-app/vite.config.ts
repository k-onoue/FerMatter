import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: { target: "safari17", chunkSizeWarningLimit: 1500 },
  server: { host: "127.0.0.1", port: 1420, strictPort: true },
});
