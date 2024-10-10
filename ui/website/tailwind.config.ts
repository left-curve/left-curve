import { tailwindConfig } from "@leftcurve/config/tailwind/config";

const config = {
  content: [
    "./src/**/*.{js,ts,jsx,tsx,mdx,astro}",
    "node_modules/@dango/shared/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
