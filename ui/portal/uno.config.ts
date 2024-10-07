import { leftCurvePreset } from "@leftcurve/config/unocss/config";
import { defineConfig, presetUno } from "unocss";

export default defineConfig({
  content: {
    filesystem: [
      "./src/**/*.{html,js,ts,jsx,tsx}",
      "./node_modules/@dango/shared/**/*.{js,ts,jsx,tsx}",
    ],
  },
  presets: [presetUno(), leftCurvePreset()],
});
