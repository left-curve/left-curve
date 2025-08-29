import { tailwindConfig } from "@left-curve/foundation/tailwind/config.js";
import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./.storybook/**/*.{js,ts,jsx,tsx,stories.tsx}",
    "./src/components/**/*.{js,ts,jsx,tsx,stories.tsx}",
  ],
  theme: {
    extend: {},
  },
  presets: [tailwindConfig],
};

export default config;
