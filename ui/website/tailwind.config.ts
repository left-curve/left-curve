import { tailwindConfig } from "@left-curve/config/tailwind/config";
import type { Config } from "tailwindcss";

export default {
  content: ["./components/**/*.{js,ts,jsx,tsx,mdx}", "./app/**/*.{js,ts,jsx,tsx,mdx}"],
  presets: [tailwindConfig],
} satisfies Config;
