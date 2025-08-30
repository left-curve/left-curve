import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "fs-extra";

import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

import { paraglideRspackPlugin } from "@inlang/paraglide-js";
import { sentryWebpackPlugin } from "@sentry/webpack-plugin";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";

import { devnet, local, testnet } from "@left-curve/dango";

import type { Chain } from "@left-curve/dango/types";
import type { Rspack } from "@rsbuild/core";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const { publicVars } = loadEnv();

const environment = process.env.CONFIG_ENVIRONMENT || "local";

const workspaceRoot = path.resolve(__dirname, "../../../");

fs.copySync(
  path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
  path.resolve(__dirname, "public/images"),
  { overwrite: true },
);

const chain = {
  local: local,
  dev: devnet,
  test: testnet,
}[environment] as Chain;

const urls =
  environment === "local"
    ? {
        faucetUrl: "http://localhost:8082/mint",
        questUrl: "http://localhost:8081/check_username",
        upUrl: "http://localhost:8080/up",
      }
    : {
        faucetUrl: `${chain.urls.indexer.replace(/\/graphql$/, "/faucet")}/mint`,
        questUrl: `${chain.urls.indexer.replace(/\/graphql$/, "/quests")}/check_username`,
        upUrl: `${chain.urls.indexer.replace(/\/graphql$/, "/up")}`,
      };

const envConfig = `window.dango = ${JSON.stringify(
  {
    chain,
    urls,
  },
  null,
  2,
)};`;

export default defineConfig({
  resolve: {
    aliasStrategy: "prefer-alias",
    alias: {
      // Order matters
      "~/paraglide": path.resolve(__dirname, "./.paraglide"),
      "~/constants": path.resolve(__dirname, "./constants.config.ts"),
      "~/mock": path.resolve(__dirname, "./mockData.ts"),
      "~/store": path.resolve(__dirname, "./store.config.ts"),
      "~/chartiq": path.resolve(__dirname, "./chartiq.config.ts"),
      "~/datafeed": path.resolve(__dirname, "./datafeed.config.ts"),
      "~": path.resolve(__dirname, "./src"),
    },
  },
  source: {
    entry: {
      index: "./src/index.tsx",
      "tv-overrides": "./public/styles/tv-overrides.css",
    },
    define: {
      ...publicVars,
      "import.meta.env.CONFIG_ENVIRONMENT": `"${process.env.CONFIG_ENVIRONMENT || "local"}"`,
      "process.env": {},
      "import.meta.env": {},
    },
  },
  server: { port: 5080 },
  html: { template: "public/index.html", title: "" },
  performance: {
    prefetch: {
      type: "all-assets",
      include: [/.*\.woff2$/],
    },
  },
  output: {
    distPath: {
      root: "build",
    },
    copy: [
      {
        from: path.resolve(
          workspaceRoot,
          "node_modules",
          "@left-curve/tradingview/charting_library",
        ),
        to: "./static/charting_library",
      },
    ],
    minify: {
      jsOptions: {
        exclude: [],
        minimizerOptions: {
          compress: false,
        },
      },
    },
  },
  plugins: [pluginReact()],
  tools: {
    rspack: (config, { rspack }) => {
      config.plugins ??= [];

      config.plugins.push(
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
        {
          apply(compiler: Rspack.Compiler) {
            compiler.hooks.thisCompilation.tap("GenerateConfigPlugin", (compilation) => {
              compilation.hooks.processAssets.tap(
                {
                  name: "GenerateConfigPlugin",
                  stage: rspack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL,
                },
                (assets) => {
                  assets["static/js/config.js"] = new rspack.sources.RawSource(envConfig);
                },
              );
            });
          },
        },
      );

      config.devtool = "source-map";
      return config;
    },
  },
});
