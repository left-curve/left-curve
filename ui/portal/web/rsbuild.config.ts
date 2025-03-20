import path from "node:path";
import { fileURLToPath } from "node:url";
import { paraglideRspackPlugin } from "@inlang/paraglide-js";
import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const { publicVars } = loadEnv();

export default defineConfig({
  source: {
    entry: {
      index: "./src/app.tsx",
    },
    alias: {
      "~": "./src",
      "~/paraglide": path.resolve(__dirname, "./.paraglide"),
    },
    define: publicVars,
  },
  server: { port: 5080 },

  html: { template: "public/index.html" },
  performance: {
    prefetch: {
      type: "all-assets",
      include: [/.*\.woff2$/],
    },
  },
  output: { distPath: { root: "build" } },
  plugins: [pluginReact()],
  tools: {
    rspack: {
      plugins: [
        paraglideRspackPlugin({
          outdir: "./.paraglide",
          emitGitIgnore: false,
          emitPrettierIgnore: false,
          includeEslintDisableComment: false,
          project: "../../config/project.inlang",
          strategy: ["localStorage", "preferredLanguage", "baseLocale"],
          localStorageKey: "dango.locale",
        }),
        TanStackRouterRspack({
          routesDirectory: "./src/pages",
          generatedRouteTree: "./src/app.pages.ts",
        }),
      ],
    },
  },
});
