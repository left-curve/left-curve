import config from "@left-curve/config/tsup/common.json" with { type: "json" };
import { type Options, defineConfig } from "tsup";

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig([
  {
    ...(config as Options),
    outExtension: ({ format }) => (format === "cjs" ? { js: ".cjs" } : { js: ".js" }),
    entry: ["src/**", "!src/**/*.spec.ts"],
    format: ["esm", "cjs"],
    platform: "browser",
  },
]);
