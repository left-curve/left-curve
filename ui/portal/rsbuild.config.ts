import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

export default defineConfig({
  source: {
    entry: {
      index: "./src/App.tsx",
    },
    alias: {
      "~": "./src",
    },
  },
  server: { port: 5080 },
  output: { distPath: { root: "build" } },
  performance: { profile: true },
  html: { template: "public/index.html" },
  plugins: [pluginReact()],
});
