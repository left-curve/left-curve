import { mergeRsbuildConfig } from "@rsbuild/core";
import { type Configuration, rspack } from "@rspack/core";
import { RspackDevServer } from "@rspack/dev-server";
import ReactRefreshPlugin from "@rspack/plugin-react-refresh";
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
            development: true,
            refresh: true,
          },
        },
      },
    },
  },
};

const dev: Configuration = {
  devtool: "eval-cheap-module-source-map",
  mode: "development",
  module: {
    rules: [tsRule],
  },
  plugins: [new ReactRefreshPlugin()],
};

const compiler = rspack(mergeRsbuildConfig(common, dev));

const server = new RspackDevServer(
  {
    port: 8080,
    hot: true,
    historyApiFallback: true,
    client: {
      overlay: {
        errors: true,
        warnings: false,
      },
    },
  },
  compiler,
);

server.start();
