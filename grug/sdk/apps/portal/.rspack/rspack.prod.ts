import { mergeRsbuildConfig } from "@rsbuild/core";
import { type Configuration, rspack } from "@rspack/core";
import { common } from "./rspack.config";

const tsRule = {
  test: /\.(ts|tsx)$/,
  use: {
    loader: "builtin:swc-loader",
    options: {
      jsc: {
        parser: {
          syntax: "typescript",
          tsx: true,
        },
        transform: {
          react: {
            runtime: "automatic",
            development: false,
            refresh: false,
          },
        },
      },
    },
  },
};

const config: Configuration = {
  mode: "production",
  devtool: false,
  module: {
    rules: [tsRule],
  },
  output: {
    filename: "js/[name].[contenthash].js",
    chunkFilename: "js/[name].[contenthash].js",
    cssFilename: "css/[name].[contenthash].css",
    cssChunkFilename: "css/[name].[contenthash].css",
  },
  optimization: {
    runtimeChunk: "single",
    splitChunks: {
      chunks: "all",
      cacheGroups: {
        vendors: {
          test: /[\\/]node_modules[\\/]/,
          name: "vendors",
          priority: -10,
        },
        default: {
          minChunks: 2,
          priority: -20,
          reuseExistingChunk: true,
        },
      },
    },
    minimize: true,
    chunkIds: "deterministic",
    removeEmptyChunks: true,
    mergeDuplicateChunks: true,
  },
};

const compiler = rspack(mergeRsbuildConfig(common, config));

compiler.run((err) => {
  if (err) {
    console.error(err);
    process.exit(1);
  }
});
