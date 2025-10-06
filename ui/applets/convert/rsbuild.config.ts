import { defineConfig } from "@rsbuild/core";
import { loadEnv } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

const { publicVars } = loadEnv();

export default defineConfig({
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
  server: { port: 5180 },
  html: {},
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
  },
  plugins: [pluginReact()],
});
