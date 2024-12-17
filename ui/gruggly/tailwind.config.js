import { nextui } from "@nextui-org/react";

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
    "../../node_modules/@nextui-org/theme/dist/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: "#E18780",
        },
        "cw-grey-950": "#141414",
        "cw-grey-900": "#151416",
        "cw-grey-850": "#1A191B",
        "cw-grey-800": "#222222",
        "cw-grey-750": "#292929",
        "cw-grey-700": "#383838",
        "cw-grey-600": "#424242",
        "cw-grey-500": "#505050",
        "cw-grey-400": "#737373",
        "cw-grey-300": "#8E8E8E",
        "cw-grey-200": "#C5C5C5",
        "cw-grey-100": "#D2D2D2",
      },
    },
  },
  plugins: [nextui()],
};
