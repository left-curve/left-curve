import type { Preview } from "@storybook/react";

import "../styles.css";
import "@dango/assets/fonts/ABCDiatypeRounded/index.css";

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
