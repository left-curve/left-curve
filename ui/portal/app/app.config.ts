import type { ConfigContext, ExpoConfig } from "expo/config";

const bundleId =
  process.env.NODE_ENV === "production" ? "io.leftcurve.dangoapp" : "io.leftcurve.dangoapp.preview";

export default ({ config }: ConfigContext): ExpoConfig => ({
  ...config,
  name: "Dango",
  slug: "dango-app",
  owner: "leftcurve",
  version: "1.0.0",
  orientation: "portrait",
  icon: "./assets/icon.png",
  userInterfaceStyle: "dark",
  newArchEnabled: true,
  splash: {
    image: "./assets/splash-icon.png",
    resizeMode: "contain",
    backgroundColor: "#ffffff",
  },
  ios: {
    supportsTablet: true,
    bundleIdentifier: bundleId,
    associatedDomains: ["webcredentials:dango.exchange"],
    infoPlist: {
      ITSAppUsesNonExemptEncryption: false,
    },
  },
  android: {
    adaptiveIcon: {
      foregroundImage: "./assets/adaptive-icon.png",
      backgroundColor: "#ffffff",
    },
    package: bundleId,
  },
  web: {
    bundler: "metro",
    output: "static",
    favicon: "./assets/favicon.png",
  },
  plugins: [
    ["expo-router", { root: "./src/screens" }],
    [
      "expo-camera",
      {
        cameraPermission: "Allow $(PRODUCT_NAME) to access your camera",
      },
    ],
  ],
  extra: {
    eas: {
      projectId: "ce83fbb5-a392-413c-be8f-daa0a7527785",
    },
  },
});
