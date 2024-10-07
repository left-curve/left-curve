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
  html: { template: "public/index.html" },
  plugins: [pluginReact()],
});
