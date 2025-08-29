import { dirname, join } from "node:path";
import type { StorybookConfig } from "storybook/internal/types";

/**
 * This function is used to resolve the absolute path of a package.
 * It is needed in projects that use Yarn PnP or are set up within a monorepo.
 */
function getAbsolutePath(value: string) {
  return dirname(require.resolve(join(value, "package.json")));
}

const config: StorybookConfig = {
  stories: ["./pages/**/*.mdx", "../src/**/*.stories.tsx"],
  addons: [
    getAbsolutePath("@storybook/addon-a11y"),
    getAbsolutePath("@storybook/addon-links"),
    getAbsolutePath("@storybook/addon-essentials"),
  ],
  framework: {
    name: getAbsolutePath("storybook-react-rsbuild"),
    options: {},
  },
  core: {
    disableTelemetry: true,
  },
};
export default config;
