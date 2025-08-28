import type { Config } from "tailwindcss";
import { tailwindConfig } from "../../foundation/shared/tailwind/config";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,mdx}", "../../foundation/web/src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [tailwindConfig],
};

export default config;
