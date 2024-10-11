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
        green: {
          DEFAULT: "#afb244",
          50: "#f9f8ec",
          100: "#f1f0d6",
          200: "#e5e4b1",
          300: "#d2d184",
          400: "#bdbf5c",
          500: "#afb244",
          600: "#7e822e",
          700: "#606427",
          800: "#4d5024",
          900: "#424522",
          950: "#22250e",
        },
        gradient: {
          start: "#7EE7A8",
          end: "#F53D6B",
        },
        "gradient-2": {
          start: "#FFF2E2",
          end: "#C4B7BA",
        },
        "typography-black": {
          100: "#867481",
          200: "#6B4862",
          300: "#402B3B",
        },
        "typography-green": {
          300: "#A9BCB2",
          400: "#71847A",
        },
        "typography-purple": {
          300: "#B9A2AA",
          400: "#926D7B",
        },
        "surface-pink": {
          200: "#D88F97",
          300: "#D07781",
        },
        "typography-pink": {
          200: "#D88F97",
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
          200: "#FEF1E1",
          300: "#FDE8CE",
          400: "#FCDFBA",
        },
        "surface-purple": {
          200: "#E0D6DA",
        },
        "surface-green": {
          100: "#EEF2F0",
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
        danger: {
          DEFAULT: "#ec6b6d",
          50: "#fdf3f3",
          100: "#fce4e4",
          200: "#facecf",
          300: "#f6abac",
          400: "#ec6b6d",
          500: "#e25153",
          600: "#cf3335",
          700: "#ad282a",
          800: "#902426",
          900: "#782425",
          950: "#410e0f",
        },
        brand: {
          pink: "#DD375B",
          green: "#AFB244",
          white: "#F2E2B8",
        },
        purple: {
          DEFAULT: "#C2C0E1",
          50: "#f7f7fb",
          100: "#f0f0f7",
          200: "#e3e3f1",
          300: "#c2c0e1",
          400: "#b2aed7",
          500: "#958cc6",
          600: "#8172b5",
          700: "#6f5fa2",
          800: "#5d5087",
          900: "#4e436f",
          950: "#312b4a",
        },
        primary: {
          DEFAULT: "#006FEE",
          foreground: "#e6f1fe",
          50: "#e6f1fe",
          100: "#cce3fd",
          200: "#99c7fb",
          300: "#66aaf9",
          400: "#338ef7",
          500: "#006FEE",
          600: "#005bc4",
          700: "#004493",
          800: "#002e62",
          900: "#001731",
        },
      },
      borderRadius: {
        small: "8px",
        medium: "12px",
        large: "14px",
      },
      fontFamily: {
        "diatype-rounded": "ABCDiatypeRounded",
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
