import { defineConfig } from "tsup";

/**
 * @see https://tsup.egoist.dev/#usage
 */

export default defineConfig({
  dts: true,
  clean: true,
  // https://tsup.egoist.dev/#inject-cjs-and-esm-shims
  shims: true,
  bundle: true,
  outDir: "dist",
  platform: "node",
  target: "esnext",
  format: ["esm", "cjs"],
  treeshake: "recommended",
  entry: ["./src/index.ts"],
  external: ["node:fs", "node:os", "node:crypto"],
});
