import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "fs-extra";

import { defineConfig } from "vite";
import { rnw } from "vite-plugin-rnw";
import tailwindcss from "@tailwindcss/vite";
import { uniwind } from "uniwind/vite";
import { nodePolyfills } from "vite-plugin-node-polyfills";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { VitePWA } from "vite-plugin-pwa";

import { devnet, local, testnet, mainnet } from "@left-curve/dango";

import type { Chain } from "@left-curve/dango/types";
import type { Plugin } from "vite";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const PORT = 5090;

const environment = process.env.CONFIG_ENVIRONMENT || "test";

const enabledFeatures = process.env.ENABLED_FEATURES
  ? process.env.ENABLED_FEATURES.split(",").map((f) => f.trim())
  : [];

const workspaceRoot = path.resolve(__dirname, "../../../");

const tradingViewPath = path.resolve(
  workspaceRoot,
  "node_modules",
  "@left-curve/tradingview/charting_library",
);

fs.copySync(
  path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
  path.resolve(__dirname, "public/images"),
  { overwrite: true },
);

if (fs.existsSync(tradingViewPath)) {
  fs.copySync(tradingViewPath, path.resolve(__dirname, "public/static/charting_library"), {
    overwrite: true,
  });
}

const hyperlaneConfig = async () => {
  const hyperlaneDir = path.resolve(workspaceRoot, "dango/hyperlane-deployment");

  const mainFiles = {
    config: path.resolve(hyperlaneDir, "config.json"),
    deployment: path.resolve(hyperlaneDir, "deployments.json"),
  };

  const testFiles = {
    config: path.resolve(hyperlaneDir, "config.testnet.json"),
    deployment: path.resolve(hyperlaneDir, "deployments-testnet.json"),
  };

  const files = environment === "prod" ? mainFiles : testFiles;

  const config = JSON.parse(fs.readFileSync(files.config, "utf-8"));
  const deployments = JSON.parse(fs.readFileSync(files.deployment, "utf-8"));

  Object.entries(deployments.evm).forEach(([chainId, deployment]: [string, any]) => {
    config.evm[chainId].warp_routes = deployment.warp_routes.map(
      ([warp_route_type, route]: [string, object]) => ({
        warp_route_type,
        ...route,
      }),
    );
  });

  return config;
};

const chain = {
  local: local,
  dev: devnet,
  test: testnet,
  prod: mainnet,
}[environment] as Chain;

const urls = {
  local: {
    faucetUrl: "http://localhost:8082/mint",
    questUrl: "http://localhost:8081/check_username",
    upUrl: "http://localhost:8080/up",
    pointsUrl: "http://localhost:8083/points-api",
  },
  dev: {
    faucetUrl: "https://faucet-devnet-ovh2.dango.zone/mint",
    questUrl: "https://quest-bot-devnet.dango.zone/check_username",
    upUrl: `${chain.urls.indexer}/up`,
    pointsUrl: "https://points-devnet.dango.zone",
  },
  test: {
    faucetUrl: "https://faucet-testnet-hetzner4.dango.zone/mint",
    questUrl: "https://quest-bot-testnet.dango.zone/check_username",
    upUrl: `${chain.urls.indexer}/up`,
    pointsUrl: "https://points-testnet.dango.zone",
  },
  prod: {
    faucetUrl: "/faucet",
    questUrl: "/quest",
    upUrl: `${chain.urls.indexer}/up`,
    pointsUrl: "https://points-mainnet.dango.zone",
  },
}[environment]!;

const banner = {
  dev: "You are using devnet",
  test: "You are using testnet",
}[environment];

const isLocal = process.env.NODE_ENV === "development";

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
          pointsUrl: `http://localhost:${PORT}/points-api`,
        }
      : urls,
    banner,
    enabledFeatures,
  },
  null,
  2,
)};`;

const configHash = crypto.createHash("md5").update(envConfig).digest("hex").slice(0, 8);

function generateConfigPlugin(): Plugin {
  return {
    name: "generate-config",
    transformIndexHtml() {
      return [
        {
          tag: "script",
          attrs: { src: `/static/js/config.js?v=${configHash}` },
          injectTo: "head-prepend",
        },
        ...(environment === "test" || environment === "dev"
          ? [
              {
                tag: "script" as const,
                children: `if (new URLSearchParams(window.location.search).has("debug")) {
                  var s = document.createElement("script");
                  s.src = "https://cdn.jsdelivr.net/npm/eruda";
                  s.onload = function () { eruda.init(); };
                  document.head.appendChild(s);
                }`,
                injectTo: "head" as const,
              },
            ]
          : []),
      ];
    },
    configureServer(server) {
      server.middlewares.use("/static/js/config.js", (_req, res) => {
        res.setHeader("Content-Type", "application/javascript");
        res.end(envConfig);
      });
    },
    generateBundle() {
      this.emitFile({
        type: "asset",
        fileName: "static/js/config.js",
        source: envConfig,
      });
    },
  };
}

export default defineConfig(async () => {
  const hyperlane = await hyperlaneConfig();

  return {
    envPrefix: "PUBLIC_",
    define: {
      "import.meta.env.CONFIG_ENVIRONMENT": JSON.stringify(process.env.CONFIG_ENVIRONMENT || "local"),
      "import.meta.env.HYPERLANE_CONFIG": JSON.stringify(hyperlane),
      "process.env": "{}",
    },
    resolve: {
      alias: {
        "~/constants": path.resolve(__dirname, "./constants.config.ts"),
        "~/store": path.resolve(__dirname, "./store.config.ts"),
        "~/datafeed": path.resolve(__dirname, "./datafeed.config.ts"),
        "~/images": path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
        "~": path.resolve(__dirname, "./src"),
      },
      extensions: [".web.tsx", ".web.ts", ".web.js", ".tsx", ".ts", ".js"],
    },
    server: {
      port: PORT,
      proxy: {
        "/graphql": {
          target: `${chain.urls.indexer}/graphql`,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/graphql/, ""),
          ws: true,
        },
        "/faucet": {
          target: urls.faucetUrl,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/faucet/, ""),
        },
        "/quest": {
          target: urls.questUrl,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/quest/, ""),
        },
        "/up": {
          target: `${chain.urls.indexer}/up`,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/up/, ""),
        },
        "/points-api": {
          target: urls.pointsUrl,
          changeOrigin: true,
          rewrite: (p) => p.replace(/^\/points-api/, ""),
        },
      },
    },
    build: {
      outDir: "build",
      sourcemap: true,
    },
    plugins: [
      rnw(),
      tailwindcss(),
      uniwind({
        cssEntryFile: "./src/styles/global.css",
      }),
      nodePolyfills({
        include: ["buffer"],
      }),
      generateConfigPlugin(),
      VitePWA({
        registerType: "autoUpdate",
        workbox: {
          cacheId: "leftcurve-nova",
          clientsClaim: true,
          skipWaiting: true,
          cleanupOutdatedCaches: true,
          navigationPreload: true,
          maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
          globIgnores: ["**/charting_library/**"],
          runtimeCaching: [
            {
              urlPattern: ({ request }) => request.mode === "navigate",
              handler: "NetworkFirst",
              options: {
                cacheName: "html-cache",
                networkTimeoutSeconds: 3,
              },
            },
          ],
        },
        injectRegister: false,
      }),
      sentryVitePlugin({
        org: process.env.SENTRY_ORG,
        project: process.env.SENTRY_PROJECT,
        authToken: process.env.SENTRY_AUTH_TOKEN,
        telemetry: false,
        sourcemaps: {
          filesToDeleteAfterUpload: ["build/**/*.map"],
        },
      }),
    ],
  };
});
