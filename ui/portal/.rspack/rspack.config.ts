import path from "node:path";
import type { Configuration } from "@rspack/core";
import { CopyRspackPlugin, EnvironmentPlugin, HtmlRspackPlugin } from "@rspack/core";
import { version } from "../package.json" with { type: "json" };

const _dirname = path.resolve(__dirname, "..");

const cssRule = {
  test: /\.css$/,
  use: [
    {
      loader: "postcss-loader",
      options: {
        postcssOptions: {
          plugins: {
            tailwindcss: {},
            autoprefixer: {},
          },
        },
      },
    },
  ],
  type: "css",
};

const fontRule = {
  test: /\.(woff2|woff)$/,
  type: "asset/resource",
  generator: {
    filename: "fonts/[name][ext]",
  },
};

const imagesRule = {
  test: /\.(png)$/,
  type: "asset/resource",
  generator: {
    filename: "assets/[name][ext]",
  },
};

export const common: Configuration = {
  context: _dirname,
  resolve: {
    alias: {
      "~": path.resolve(_dirname, "./src"),
      react: path.resolve(_dirname, "./node_modules/react"),
    },
    extensions: [".tsx", ".ts", ".js", ".mjs"],
  },
  output: {
    filename: "js/[name].js",
    publicPath: "/",
    path: path.resolve(_dirname, "build"),
    clean: true,
  },
  entry: {
    main: "./src/App.tsx",
  },
  plugins: [
    new EnvironmentPlugin({ VERSION: version }),
    new HtmlRspackPlugin({
      template: "./public/index.html",
      filename: "index.html",
      chunks: ["main"],
    }),
    new CopyRspackPlugin({
      patterns: [{ from: "./public/images", to: "images", force: true }],
    }),
  ],
  module: {
    rules: [cssRule, fontRule, imagesRule],
  },
  experiments: {
    css: true,
  },
};
