import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import config from "@leftcurve/config/tsup/common.json" with { type: "json" };
import { glob } from "glob";
import { type Options, defineConfig } from "tsup";

async function onSuccess() {
  const files = glob.sync("build/**/*.{js,mjs,ts,mts}");

  for (const file of files) {
    const filePath = path.join(__dirname, file);

    const content = await fs.readFile(filePath, "utf8");
    const relativePath = path.relative(file, path.resolve(__dirname, "build")).replace("/..", "");

    await fs.writeFile(filePath, content.replace(/~\//g, relativePath + "/"), "utf8");
  }

  spawnSync("tsup", ["--dts-only", "--silent"]);
  spawnSync("pnpm", ["tw:build"]);
  spawnSync("cp", ["-r", "src/fonts", "build"]);
}

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig([
  {
    ...(config as Options),
    entry: glob.sync("src/**/*.{ts,tsx}", {
      ignore: ["**/*.stories.*", "**/*.spec.*"],
    }),
    bundle: false,
    splitting: false,
    treeshake: false,
    format: ["esm"],
    external: ["react", "react-dom", "@tanstack/react-query"],
    platform: "browser",
    onSuccess,
  },
]);
