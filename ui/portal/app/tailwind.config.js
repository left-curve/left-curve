import { tailwindConfig } from "../../config/tailwind/config";

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  presets: [require("nativewind/preset"), tailwindConfig],
  future: {
    hoverOnlyWhenSupported: true,
  },
  plugins: [],
};
