import { defineConfig } from "vitest/config";
import path from "node:path";

const root = path.resolve(__dirname, "../../..");

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["tests/**/*.test.{ts,tsx}"],
    globals: true,
    setupFiles: ["tests/setup.ts"],
  },
  resolve: {
    alias: {
      "@left-curve/dango/utils": path.resolve(root, "sdk/dango/build/utils/index.js"),
      "@left-curve/dango/types": path.resolve(root, "sdk/dango/build/types/index.js"),
      "@left-curve/dango": path.resolve(root, "sdk/dango/build/index.js"),
      "@left-curve/sdk/utils": path.resolve(root, "sdk/grug/build/utils/index.js"),
    },
  },
});
