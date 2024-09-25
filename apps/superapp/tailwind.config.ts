import { tailwindConfig } from "@leftcurve/config/tailwind/config";
import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{js,ts,jsx,tsx,mdx}", "node_modules/@leftcurve/dango/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
