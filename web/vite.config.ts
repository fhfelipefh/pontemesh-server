import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": new URL("./src", import.meta.url).pathname
    }
  },
  build: {
    outDir: "dist"
  },
  server: {
    port: 5173,
    proxy: {
      "/api": "http://localhost:8080"
    }
  }
});
