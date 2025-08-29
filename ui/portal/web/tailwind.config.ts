import type { Config } from "tailwindcss";
import { tailwindConfig } from "../../foundation/tailwind/config";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}", "../../applets/kit/src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
