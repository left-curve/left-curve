import { tailwindConfig } from "@leftcurve/config/tailwind/config";

const config = {
  content: ["./app/**/*.{js,ts,jsx,tsx,mdx}", "../shared/src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
