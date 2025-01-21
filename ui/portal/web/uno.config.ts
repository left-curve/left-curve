import { leftCurvePreset } from "@left-curve/config/unocss/config";
import { defineConfig, presetUno } from "unocss";

export default defineConfig({
  content: {
    filesystem: [
      "node_modules/@left-curve/applets-kit/build/**/*.mjs",
      "./src/**/*.{html,js,ts,jsx,tsx}",
    ],
  },
  presets: [presetUno(), leftCurvePreset()],
});
