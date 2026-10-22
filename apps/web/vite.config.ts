/// <reference types="vitest/config" />
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vite";

// REWIRE_E2E=1 adds labcheck.html, a bare page that runs one program through
// webRun in the production build (see e2e/prod-check.mjs). It never ships.
const e2e = process.env.REWIRE_E2E === "1";

export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  build: {
    target: "es2022",
    outDir: e2e ? "dist-e2e" : "dist",
    rollupOptions: {
      input: e2e ? { main: "index.html", labcheck: "labcheck.html" } : { main: "index.html" },
    },
  },
  worker: { format: "es" },
  test: {
    // Node by default (lab + core tests). Component tests opt in with
    // `// @vitest-environment jsdom` at the top of the file.
    environment: "node",
    include: ["src/**/*.test.ts", "scripts/**/*.test.ts"],
  },
});
