import { tailwindConfig } from "@left-curve/foundation/tailwind/config.js";
import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{js,ts,jsx,tsx,stories.tsx}", "../kit/src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  presets: [tailwindConfig],
};

export default config;
