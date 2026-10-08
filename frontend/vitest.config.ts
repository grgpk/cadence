import { defineConfig } from "vitest/config";

// Kept apart from vite.config.ts so unit tests do not load the Start and nitro plugins.
export default defineConfig({
  test: { environment: "node", include: ["src/**/*.test.ts"] },
});
