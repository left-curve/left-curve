import path from "node:path";
import { fileURLToPath } from "node:url";
import { paraglideRspackPlugin } from "@inlang/paraglide-js";
import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { sentryWebpackPlugin } from "@sentry/webpack-plugin";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const { publicVars } = loadEnv();

const storePath = {
  local: path.resolve(__dirname, "./store.config.local.ts"),
  dev: path.resolve(__dirname, "./store.config.dev.ts"),
  test: path.resolve(__dirname, "./store.config.testnet.ts"),
  prod: path.resolve(__dirname, "./store.config.prod.ts"),
};

export default defineConfig({
  resolve: {
    aliasStrategy: "prefer-alias",
    alias: {
      // Order matters
      "~/paraglide": path.resolve(__dirname, "./.paraglide"),
      "~/constants": path.resolve(__dirname, "./constants.config.ts"),
      "~/mock": path.resolve(__dirname, "./mockData.ts"),
      "~/store": storePath[(process.env.CONFIG_ENVIRONMENT || "local") as keyof typeof storePath],
      "~": path.resolve(__dirname, "./src"),
    },
  },
  source: {
    entry: {
      index: "./src/index.tsx",
    },
    define: {
      ...publicVars,
      "process.env": {},
      "import.meta.env": {},
    },
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
      devtool: "source-map",
      plugins: [
        sentryWebpackPlugin({
          org: process.env.SENTRY_ORG,
          project: process.env.SENTRY_PROJECT,
          authToken: process.env.SENTRY_AUTH_TOKEN,
          telemetry: false,
          sourcemaps: {
            filesToDeleteAfterUpload: ["build/**/*.map"],
          },
        }),
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
