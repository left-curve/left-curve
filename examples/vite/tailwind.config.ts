import { tailwindConfig } from "@leftcurve/config/tailwind/config";
import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/**/*.{js,ts,jsx,tsx}",
    "node_modules/@leftcurve/config/tailwind/**",
    "node_modules/@leftcurve/dango/src/components/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
