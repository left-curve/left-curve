import { defineConfig } from "vitest/config";
import path from "node:path";

const root = path.resolve(__dirname, "../../..");

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["tests/**/*.test.{ts,tsx}", "src/**/*.test.{ts,tsx}"],
    globals: true,
    setupFiles: ["tests/setup.ts"],
  },
  resolve: {
    alias: {
      "@left-curve/crypto": path.resolve(root, "sdk/typescript/crypto/src/index.ts"),
      "@left-curve/encoding": path.resolve(root, "sdk/typescript/encoding/src/index.ts"),
      "@left-curve/applets-kit": path.resolve(root, "ui/applets/kit/src/index.ts"),
      "@left-curve/foundation/paraglide/messages.js": path.resolve(
        __dirname,
        "tests/mocks/foundationMessages.ts",
      ),
      "@left-curve/store": path.resolve(root, "ui/store/src/index.ts"),
      "@left-curve/store/types": path.resolve(root, "ui/store/src/types/index.ts"),
      "@left-curve/types": path.resolve(root, "sdk/typescript/types/src/index.ts"),
      "@left-curve/utils": path.resolve(root, "sdk/typescript/utils/src/index.ts"),
      "@left-curve/sdk/utils": path.resolve(root, "sdk/typescript/dango/src/utils/index.ts"),
      "@left-curve/sdk/types": path.resolve(root, "sdk/typescript/dango/src/types/index.ts"),
      "@left-curve/sdk": path.resolve(root, "sdk/typescript/dango/src/index.ts"),
      "~/constants": path.resolve(__dirname, "constants.config.ts"),
      "~": path.resolve(__dirname, "src"),
    },
  },
});
