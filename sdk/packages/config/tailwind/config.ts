import tailwindScrollbar from "tailwind-scrollbar";
import type { Config } from "tailwindcss";
import plugin from "tailwindcss/plugin";

export const tailwindConfig: Partial<Config> = {
  theme: {
    extend: {
      backgroundImage: {
        "gradient-container": "linear-gradient(156.47deg, #FFF2E299 23.72%, #C4B7BA99 128.44%)",
      },
      colors: {
        purple: {
          600: "#A38590",
        },
        green: {
          600: "#71847A",
        },
        gradient: {
          start: "#7EE7A8",
          end: "#F53D6B",
        },
        "gradient-2": {
          start: "#FFF2E2",
          end: "#C4B7BA",
        },
        "borders-purple": {
          600: "#A38590",
        },
        "typography-black": {
          100: "#867481",
          200: "#6B4862",
          300: "#402B3B",
        },
        "typography-green": {
          300: "#A9BCB2",
          400: "#71847A",
          500: "#596861",
        },
        "typography-purple": {
          300: "#B9A2AA",
          400: "#926D7B",
          500: "#755762",
        },
        "surface-pink": {
          200: "#D88F97",
          300: "#D07781",
        },
        "typography-pink": {
          200: "#D88F97",
          300: "#C93646",
        },
        "typography-rose": {
          500: "#E0B989",
          600: "#C9A274",
        },
        "typography-yellow": {
          300: "#CFBA4F",
          400: "#C8B137",
        },
        "surface-rose": {
          100: "#FFFAF5",
          200: "#FEF1E1",
          300: "#FDE8CE",
          400: "#FCDFBA",
          600: "#C9A274",
        },
        "surface-purple": {
          100: "#E5DCE0",
          200: "#E0D6DA",
          300: "#CFBFC5",
        },
        "surface-green": {
          100: "#EEF2F0",
          200: "#EEF2F0",
          300: "#DCE4E0",
          400: "#C2D0C9",
        },
        "surface-yellow": {
          100: "#F9F7EB",
          200: "#F4EFD7",
          300: "#ECE4BA",
        },
        "surface-off-white": {
          200: "#FFFBF0",
          500: "#DCD5BC",
        },
        sand: {
          DEFAULT: "#F5DDB8",
          50: "#fdf8ef",
          100: "#faeeda",
          200: "#f5ddb8",
          300: "#edc184",
          400: "#e59e52",
          500: "#df8430",
          600: "#d16c25",
          700: "#ad5421",
          800: "#8a4322",
          900: "#70391e",
          950: "#3c1b0e",
        },
        brand: {
          pink: "#DD375B",
          green: "#AFB244",
          white: "#F2E2B8",
        },
      },
      borderRadius: {
        small: "8px",
        medium: "12px",
        large: "14px",
      },
      fontFamily: {
        "diatype-rounded": "ABCDiatypeRounded",
        exposure: "Exposure",
      },
      animation: {
        "rotate-2": "rotate 4s linear infinite",
        "rotate-4": "rotate 4s linear infinite",
        "dash-4": "dash 2s ease-in-out infinite",
        "spinner-ease-spin": "spinner-spin 0.8s ease infinite",
        "spinner-linear-spin": "spinner-spin 0.8s linear infinite",
      },
      keyframes: {
        "spinner-spin": {
          "0%": {
            transform: "rotate(0deg)",
          },
          "100%": {
            transform: "rotate(360deg)",
          },
        },
        rotate: {
          "100%": {
            transform: "rotate(360deg)",
          },
        },
        dash: {
          "0%": { "stroke-dasharray": "1, 200", "stroke-dashoffset": "0" },
          "50%": { "stroke-dasharray": "90, 200", "stroke-dashoffset": "-35px" },
          "100%": { "stroke-dashoffset": "-125px" },
        },
      },
    },
  },
  plugins: [
    require("tailwindcss-animate"),
    tailwindScrollbar({ nocompatible: true }),
    plugin(({ addUtilities }) => {
      addUtilities({
        ".tap-highlight-transparent": {
          "-webkit-tap-highlight-color": "transparent",
        },
        ".drag-none": {
          "-webkit-user-drag": "none",
          "-khtml-user-drag": "none",
          "-moz-user-drag": "none",
          "-o-user-drag": "none",
          "user-drag": "none",
        },
      });
    }),
  ],
};
