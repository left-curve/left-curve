import config from "@left-curve/config/tsup/common.json" with { type: "json" };
import { type Options, defineConfig } from "tsup";

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig([
  {
    ...(config as Options),
    entry: ["./react/**"],
    bundle: false,
    splitting: false,
    treeshake: false,
    format: ["esm", "cjs"],
    external: ["react", "react-dom", "@tanstack/react-query"],
    platform: "browser",
    publicDir: "./public",
    outExtension: ({ format }) => (format === "cjs" ? { js: ".cjs" } : { js: ".js" }),
    esbuildOptions(options) {
      options.banner = {
        js: "'use client'",
      };
    },
  },
]);
