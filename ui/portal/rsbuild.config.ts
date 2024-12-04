import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

const { publicVars } = loadEnv();

export default defineConfig({
  source: {
    entry: {
      index: "./src/App.tsx",
    },
    alias: {
      "~": "./src",
    },
    define: publicVars,
  },
  server: { port: 5080 },
  performance: {
    prefetch: {
      type: "all-assets",
      include: [/.*\.woff2$/],
    },
  },
  output: { distPath: { root: "build" } },
  html: { template: "public/index.html" },
  plugins: [pluginReact()],
});
