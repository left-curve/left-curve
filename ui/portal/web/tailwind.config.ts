import { tailwindConfig } from "@left-curve/ui-config/tailwind/config.js";
import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}", "../../applets/kit/src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
