import { defineConfig } from "tsup";

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig({
  dts: false,
  shims: false,
  bundle: true,
  clean: true,
  silent: true,
  outDir: "build",
  platform: "node",
  target: "esnext",
  format: ["esm", "cjs"],
  treeshake: false,
  entry: ["src/index.ts"],
});
