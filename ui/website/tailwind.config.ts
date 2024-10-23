import { tailwindConfig } from "@leftcurve/config/tailwind/config";

const config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx,astro}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
