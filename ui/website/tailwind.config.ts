import type { Config } from "tailwindcss";
import { tailwindConfig } from "../config/tailwind/config";

export default {
  content: ["./components/**/*.{js,ts,jsx,tsx,mdx}", "./app/**/*.{js,ts,jsx,tsx,mdx}"],
  presets: [tailwindConfig],
} satisfies Config;
