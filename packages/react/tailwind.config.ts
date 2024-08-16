import { tailwindConfig } from "@leftcurve/config/tailwind/config";
import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./.storybook/**/*.{js,ts,jsx,tsx,stories.tsx}",
    "./src/**/*.{js,ts,jsx,tsx,stories.tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
