import config from "@leftcurve/config/tsup/common.json" with { type: "json" };
import { type Options, defineConfig } from "tsup";

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig([
  {
    ...(config as Options),
    outExtension: ({ format }) => (format === "cjs" ? { js: ".cjs" } : { js: ".js" }),
    banner: {
      js: '"use client";',
    },
    entry: ["src/**"],
    format: ["esm"],
    external: ["react", "@tanstack/react-query"],
    platform: "browser",
    splitting: false,
    treeshake: false,
  },
]);
