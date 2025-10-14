import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "fs-extra";

import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSvgr } from "@rsbuild/plugin-svgr";

import { sentryWebpackPlugin } from "@sentry/webpack-plugin";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";
import { GenerateSW } from "workbox-webpack-plugin";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";

import { devnet, local, testnet } from "@left-curve/dango";

import type { Chain } from "@left-curve/dango/types";
import type { Rspack } from "@rsbuild/core";

const isLocal = process.env.NODE_ENV === "development";

const PORT = 5080;

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

const urls = {
  local: {
    faucetUrl: "http://localhost:8082",
    questUrl: "http://localhost:8081",
    upUrl: "http://localhost:8080/up",
  },
  dev: {
    faucetUrl: `${chain.urls.indexer.replace("api", "faucet")}`,
    questUrl: `http://api.devnet.ovh2.dango.zone:8091`,
    upUrl: `${chain.urls.indexer}/up`,
  },
  test: {
    faucetUrl: `${chain.urls.indexer.replace("api", "faucet")}`,
    questUrl: `http://api.testnet.ovh2.dango.zone:8091`,
    upUrl: `${chain.urls.indexer}/up`,
  },
}[environment]!;

const banner = {
  dev: "You are using devnet",
}[environment];

const envConfig = `window.dango = ${JSON.stringify(
  {
    chain: isLocal
      ? {
          ...chain,
          urls: { indexer: `http://localhost:${PORT}` },
        }
      : chain,
    urls: isLocal
      ? {
          faucetUrl: `http://localhost:${PORT}/faucet`,
          questUrl: `http://localhost:${PORT}/quest`,
          upUrl: `http://localhost:${PORT}/up`,
        }
      : urls,
    banner,
  },
  null,
  2,
)};`;

export default defineConfig({
  resolve: {
    aliasStrategy: "prefer-alias",
    alias: {
      // Order matters
      "~/constants": path.resolve(__dirname, "./constants.config.ts"),
      "~/mock": path.resolve(__dirname, "./mockData.ts"),
      "~/store": path.resolve(__dirname, "./store.config.ts"),
      "~/images": path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
      "~/chartiq": path.resolve(__dirname, "./chartiq.config.ts"),
      "~/datafeed": path.resolve(__dirname, "./datafeed.config.ts"),
      "~": path.resolve(__dirname, "./src"),
    },
  },
  source: {
    entry: {
      index: "./src/index.tsx",
    },
    define: {
      ...publicVars,
      "import.meta.env.CONFIG_ENVIRONMENT": `"${process.env.CONFIG_ENVIRONMENT || "local"}"`,
      "process.env": {},
      "import.meta.env": {},
    },
  },
  server: {
    port: PORT,
    proxy: {
      "/graphql": {
        target: `${chain.urls.indexer}/graphql`,
        changeOrigin: true,
        pathRewrite: { "^/graphql": "" },
        ws: true,
      },
      "/faucet": {
        target: urls.faucetUrl,
        changeOrigin: true,
        pathRewrite: { "^/faucet": "" },
      },
      "/quest": {
        target: urls.questUrl,
        changeOrigin: true,
        pathRewrite: { "^/quest": "" },
      },
      "/up": {
        target: `${chain.urls.indexer}/up`,
        changeOrigin: true,
        pathRewrite: { "^/up": "" },
      },
    },
  },
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
  plugins: [
    pluginReact(),
    pluginSvgr(),
    pluginNodePolyfill({
      include: ["buffer"],
    }),
  ],
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

      if (process.env.NODE_ENV === "production") {
        config.plugins.push(
          new GenerateSW({
            cacheId: "leftcurve-portal",
            clientsClaim: true,
            skipWaiting: false,
            cleanupOutdatedCaches: true,
            runtimeCaching: [
              {
                urlPattern: ({ request }) => request.mode === "navigate",
                handler: "NetworkFirst",
                options: {
                  cacheName: "html-cache",
                },
              },
            ],
          }),
        );
      }

      config.devtool = "source-map";
      return config;
    },
  },
});
