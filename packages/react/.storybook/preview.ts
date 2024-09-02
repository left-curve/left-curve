import type { Preview } from "@storybook/react";

import "../styles.css";
// @ts-expect-error
// TODO: This is a workaround to import the styles until figure out why it fails
import * as _ from "../styles.css";
console.log(_);

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
};

export default preview;
