import config from "@leftcurve/config/tsup/common.json" with { type: "json" };

import { type Options, defineConfig } from "tsup";

const isDev = process.env.NODE_ENV === "development";

const extraOptions: Partial<Options> = isDev
  ? {
      treeshake: false,
      splitting: false,
      bundle: false,
    }
  : {
      treeshake: "recommended",
      splitting: true,
      bundle: true,
    };

/**
 * @see https://tsup.egoist.dev/#usage
 */
export default defineConfig({
  ...(config as Options),
  ...extraOptions,
  platform: "node",
  entry: ["src/**"],
});
