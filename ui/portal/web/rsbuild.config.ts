import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";
import { TanStackRouterRspack } from "@tanstack/router-plugin/rspack";

const { publicVars } = loadEnv();

export default defineConfig({
  source: {
    entry: {
      index: "./src/app.tsx",
    },
    alias: {
      "~": "./src",
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
        TanStackRouterRspack({
          routesDirectory: "./src/pages",
          generatedRouteTree: "./src/app.pages.ts",
        }),
      ],
    },
  },
});
