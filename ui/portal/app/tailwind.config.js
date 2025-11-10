import { tailwindConfig } from "../../foundation/tailwind/config";

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  presets: [require("nativewind/preset"), tailwindConfig],
  future: {
    hoverOnlyWhenSupported: true,
  },
  plugins: [],
};
