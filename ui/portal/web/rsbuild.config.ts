import { execSync } from "node:child_process";
import crypto from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "fs-extra";

import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { pluginSvgr } from "@rsbuild/plugin-svgr";

import { paraglideRspackPlugin } from "@inlang/paraglide-js";
import { sentryWebpackPlugin } from "@sentry/webpack-plugin";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";
import { pluginNodePolyfill } from "@rsbuild/plugin-node-polyfill";
import { pluginSourceBuild } from "@rsbuild/plugin-source-build";

import devnet from "@left-curve/sdk/chains/devnet.json" with { type: "json" };
import local from "@left-curve/sdk/chains/local.json" with { type: "json" };
import mainnet from "@left-curve/sdk/chains/mainnet.json" with { type: "json" };
import testnet from "@left-curve/sdk/chains/testnet.json" with { type: "json" };

import type { Chain } from "@left-curve/types";
import type { Rspack } from "@rsbuild/core";

const isLocal = process.env.NODE_ENV === "development";

const PORT = 5080;

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const { publicVars } = loadEnv();

const environment = process.env.CONFIG_ENVIRONMENT || "test";

const enabledFeatures = process.env.ENABLED_FEATURES
  ? process.env.ENABLED_FEATURES.split(",").map((f) => f.trim())
  : [];

const gitCommit = (() => {
  if (process.env.GIT_COMMIT) return process.env.GIT_COMMIT;
  try {
    return execSync("git rev-parse HEAD", { stdio: ["ignore", "pipe", "ignore"] })
      .toString()
      .trim();
  } catch {
    return "unknown";
  }
})();

const r2AssetsPrefix = process.env.R2_ASSETS_PREFIX || "/";
const useR2Assets = r2AssetsPrefix !== "/";
const stableR2AssetsPrefix = (() => {
  if (!useR2Assets) return "/";
  try {
    return new URL("/", r2AssetsPrefix).toString();
  } catch {
    return "/";
  }
})();

const stableR2AssetTypes = {
  font: new Set([".eot", ".otf", ".ttf", ".woff", ".woff2"]),
  image: new Set([
    ".apng",
    ".avif",
    ".bmp",
    ".cur",
    ".gif",
    ".ico",
    ".jfif",
    ".jpg",
    ".jpeg",
    ".pjp",
    ".pjpeg",
    ".png",
    ".tif",
    ".tiff",
    ".webp",
  ]),
  svg: new Set([".svg"]),
} as const;

type StableR2AssetType = keyof typeof stableR2AssetTypes;

type AssetPathData = {
  filename?: string;
  module?: {
    matchResource?: string;
    nameForCondition?: () => string | null;
    resource?: string;
  };
};

type AssetInfo = unknown;
type AssetModuleFilename = string | ((pathData: AssetPathData, assetInfo?: AssetInfo) => string);
type AssetGenerator = {
  publicPath?: string | ((pathData: AssetPathData, assetInfo?: AssetInfo) => string);
};

type AssetRule = {
  generator?: AssetGenerator;
  oneOf?: AssetRule[];
  rules?: AssetRule[];
  type?: string;
};

const getAssetSource = (pathData: AssetPathData) =>
  pathData.filename ||
  pathData.module?.matchResource ||
  pathData.module?.resource ||
  pathData.module?.nameForCondition?.() ||
  "";

const getStableR2AssetType = (pathData: AssetPathData): StableR2AssetType | null => {
  const assetPath = getAssetSource(pathData).split(/[?#]/)[0];
  const extension = path.extname(assetPath).toLowerCase();

  return (
    (Object.entries(stableR2AssetTypes) as [StableR2AssetType, Set<string>][]).find(
      ([, extensions]) => extensions.has(extension),
    )?.[0] || null
  );
};

const getStableR2AssetFilename = (pathData: AssetPathData) => {
  const assetType = getStableR2AssetType(pathData);
  return assetType ? `static/${assetType}/[name].[contenthash:8][ext][query]` : null;
};

const getStableR2AssetPublicPath = (pathData: AssetPathData) =>
  getStableR2AssetType(pathData) ? stableR2AssetsPrefix : r2AssetsPrefix;

const setStableR2AssetRulePublicPath = (rules: AssetRule[] | undefined) => {
  if (!rules) return;

  for (const rule of rules) {
    if (rule.type === "asset" || rule.type === "asset/resource") {
      rule.generator = {
        ...rule.generator,
        publicPath: getStableR2AssetPublicPath,
      };
    }

    setStableR2AssetRulePublicPath(rule.rules);
    setStableR2AssetRulePublicPath(rule.oneOf);
  }
};

const workspaceRoot = path.resolve(__dirname, "../../../");

const tradingViewPath = path.resolve(
  workspaceRoot,
  "node_modules",
  "@left-curve/tradingview/charting_library",
);

const tvVersion = (
  fs.existsSync(tradingViewPath)
    ? (fs.readJsonSync(
        path.resolve(workspaceRoot, "node_modules", "@left-curve/tradingview/package.json"),
      ).version as string)
    : "unknown"
).replace(/\./g, "_");

fs.copySync(
  path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
  path.resolve(__dirname, "public/images"),
  { overwrite: true },
);

const hyperlaneConfig = async () => {
  const mainFiles = {
    config: "./config/hyperlane/config.json",
    deployment: "./config/hyperlane/deployments.json",
  };

  const testFiles = {
    config: "./config/hyperlane/config.testnet.json",
    deployment: "./config/hyperlane/deployments-testnet.json",
  };

  const files = environment === "prod" ? mainFiles : testFiles;

  const config = await import(files.config);
  const deployments = await import(files.deployment);

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
    upUrl: `${chain.url}/up`,
    pointsUrl: "https://points-devnet.dango.zone",
  },
  test: {
    faucetUrl: "https://faucet-testnet-hetzner4.dango.zone/mint",
    questUrl: "https://quest-bot-testnet.dango.zone/check_username",
    upUrl: `${chain.url}/up`,
    pointsUrl: "https://points-testnet.dango.zone",
  },
  prod: {
    faucetUrl: "/faucet",
    questUrl: "/quest",
    upUrl: `${chain.url}/up`,
    pointsUrl: "https://points-mainnet.dango.zone",
  },
}[environment]!;

const banner = {
  dev: "You are using devnet",
  test: "You are using testnet",
}[environment];

const defaultDangoConfig = {
  chain: isLocal
    ? {
        ...chain,
        url: `http://localhost:${PORT}`,
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
};

const envConfig = `window.dango = ${
  process.env.DANGO_CONFIG_JSON || JSON.stringify(defaultDangoConfig, null, 2)
};`;

const configHash = crypto.createHash("md5").update(envConfig).digest("hex").slice(0, 8);

const swContent = `const COMMIT = ${JSON.stringify(gitCommit)};
self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    const oldSw = self.registration.active;
    if (!oldSw) return;
    const isOurSw = await new Promise((resolve) => {
      const channel = new MessageChannel();
      const timer = setTimeout(() => resolve(false), 1500);
      channel.port1.onmessage = () => { clearTimeout(timer); resolve(true); };
      try {
        oldSw.postMessage({ type: "GET_COMMIT" }, [channel.port2]);
      } catch (_) {
        clearTimeout(timer);
        resolve(false);
      }
    });
    if (!isOurSw) await self.skipWaiting();
  })());
});
self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});
self.addEventListener("message", (event) => {
  if (event.data?.type === "SKIP_WAITING") self.skipWaiting();
  if (event.data?.type === "GET_COMMIT") {
    event.ports[0]?.postMessage({ commit: COMMIT });
  }
});
`;

const copyPattern: { from: string; to: string }[] = [];

if (!useR2Assets && fs.existsSync(tradingViewPath)) {
  copyPattern.push({
    from: path.resolve(workspaceRoot, "node_modules", "@left-curve/tradingview/charting_library"),
    to: `./charting_library/${tvVersion}`,
  });
}

export default defineConfig({
  resolve: {
    aliasStrategy: "prefer-alias",
    alias: {
      "~/constants": path.resolve(__dirname, "./constants.config.ts"),
      "~/mock": path.resolve(__dirname, "./mockData.ts"),
      "~/store": path.resolve(__dirname, "./store.config.ts"),
      "~/images": path.resolve(__dirname, "node_modules", "@left-curve/foundation/images"),
      "~/datafeed": path.resolve(__dirname, "./datafeed.config.ts"),
      "~": path.resolve(__dirname, "./src"),
    },
  },
  source: {
    include: [/[\\/]@left-curve[\\/]/],
    entry: {
      index: "./src/index.tsx",
    },
    define: {
      ...publicVars,
      "import.meta.env.CONFIG_ENVIRONMENT": `"${process.env.CONFIG_ENVIRONMENT || "local"}"`,
      "import.meta.env.HYPERLANE_CONFIG": JSON.stringify(await hyperlaneConfig()),
      "import.meta.env.GIT_COMMIT": `"${gitCommit}"`,
      "import.meta.env.TV_VERSION": `"${tvVersion}"`,
      "import.meta.env.R2_ASSETS_PREFIX": JSON.stringify(r2AssetsPrefix),
      "process.env": {},
      "import.meta.env": {},
    },
  },
  server: {
    port: PORT,
    proxy: {
      "/graphql": {
        target: `${chain.url}/graphql`,
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
        target: `${chain.url}/up`,
        changeOrigin: true,
        pathRewrite: { "^/up": "" },
      },
      "/points-api": {
        target: urls.pointsUrl,
        changeOrigin: true,
        pathRewrite: { "^/points-api": "" },
      },
    },
  },
  html: {
    template: "public/index.html",
    title: "",
    tags: [
      {
        tag: "script",
        attrs: { src: `/config.js?v=${configHash}` },
        append: false,
        publicPath: false,
      },
      ...(environment === "test" || environment === "dev"
        ? [
            {
              tag: "script",
              children: `if (new URLSearchParams(window.location.search).has("debug")) {
                            var s = document.createElement("script");
                            s.src = "https://cdn.jsdelivr.net/npm/eruda";
                            s.onload = function () { eruda.init(); };
                            document.head.appendChild(s);
                  }`,
            },
          ]
        : []),
    ],
  },
  performance: {
    prefetch: useR2Assets
      ? undefined
      : {
          type: "all-assets",
          include: [/.*\.woff2$/],
        },
  },
  output: {
    assetPrefix: r2AssetsPrefix,
    distPath: {
      root: "build",
    },
    copy: copyPattern,
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
    pluginSourceBuild(),
    pluginNodePolyfill({
      include: ["buffer"],
    }),
  ],
  tools: {
    rspack: (config, { rspack }) => {
      config.output ??= {};

      const fallbackAssetModuleFilename = config.output.assetModuleFilename as
        | AssetModuleFilename
        | undefined;

      config.output.assetModuleFilename = ((pathData: AssetPathData, assetInfo?: AssetInfo) => {
        const stableFilename = getStableR2AssetFilename(pathData);
        if (stableFilename) return stableFilename;
        if (typeof fallbackAssetModuleFilename === "function") {
          return fallbackAssetModuleFilename(pathData, assetInfo);
        }
        return fallbackAssetModuleFilename || "static/assets/[name].[contenthash:8][ext][query]";
      }) as NonNullable<NonNullable<Rspack.Configuration["output"]>["assetModuleFilename"]>;

      if (useR2Assets) {
        config.module ??= {};
        config.module.generator ??= {};

        const assetGenerator = (config.module.generator.asset || {}) as AssetGenerator;
        config.module.generator.asset = {
          ...assetGenerator,
          publicPath: getStableR2AssetPublicPath,
        };

        const assetResourceGenerator = (config.module.generator["asset/resource"] ||
          {}) as AssetGenerator;
        config.module.generator["asset/resource"] = {
          ...assetResourceGenerator,
          publicPath: getStableR2AssetPublicPath,
        };

        setStableR2AssetRulePublicPath(config.module.rules as AssetRule[] | undefined);
      }

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
        paraglideRspackPlugin({
          outdir: "../../foundation/paraglide",
          project: "../../foundation/project.inlang",
          emitGitIgnore: false,
          emitPrettierIgnore: false,
          includeEslintDisableComment: false,
          strategy: ["localStorage", "preferredLanguage", "baseLocale"],
          localStorageKey: "dango.locale",
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
                  assets["config.js"] = new rspack.sources.RawSource(envConfig);
                },
              );
            });
          },
        },
        {
          apply(compiler: Rspack.Compiler) {
            compiler.hooks.thisCompilation.tap("GenerateServiceWorkerPlugin", (compilation) => {
              compilation.hooks.processAssets.tap(
                {
                  name: "GenerateServiceWorkerPlugin",
                  stage: rspack.Compilation.PROCESS_ASSETS_STAGE_ADDITIONAL,
                },
                (assets) => {
                  assets["service-worker.js"] = new rspack.sources.RawSource(swContent);
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
