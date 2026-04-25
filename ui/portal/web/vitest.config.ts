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
      "@left-curve/dango/utils": path.resolve(root, "sdk/typescript/dango/src/utils/index.ts"),
      "@left-curve/dango/types": path.resolve(root, "sdk/typescript/dango/src/types/index.ts"),
      "@left-curve/dango": path.resolve(root, "sdk/typescript/dango/src/index.ts"),
      "@left-curve/sdk/utils": path.resolve(root, "sdk/typescript/grug/src/utils/index.ts"),
    },
  },
});
