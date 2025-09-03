import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { AppletMetadata } from "@left-curve/store/types";

export const WEBRTC_URI = "wss://webrtc.dango.exchange";

const translations = m as unknown as Record<string, () => string>;
export const APPLETS: Record<string, AppletMetadata> = Object.keys(translations)
  .filter((k) => /^applets\..*\.id$/.test(k))
  .reduce((acc, key) => {
    const [_, id] = key.split(".");
    acc[id] = {
      id,
      title: translations[`applets.${id}.title`](),
      description: translations[`applets.${id}.description`](),
      img: translations[`applets.${id}.img`](),
      keywords: translations[`applets.${id}.keywords`]().split(","),
      path: translations[`applets.${id}.path`](),
    };
    return acc;
  }, Object.create({}));

export const ASSETS = {
  trade: require("@left-curve/foundation/images/emojis/simple/protrading.svg"),
  "simple-swap": require("@left-curve/foundation/images/emojis/simple/swap.svg"),
  earn: require("@left-curve/foundation/images/emojis/simple/pig.svg"),
  transfer: require("@left-curve/foundation/images/emojis/simple/money.svg"),
  "create-account": require("@left-curve/foundation/images/emojis/simple/wizard.svg"),
  settings: require("@left-curve/foundation/images/emojis/simple/settings.svg"),
  notifications: require("@left-curve/foundation/images/emojis/simple/notifications.svg"),
  devtool: require("@left-curve/foundation/images/emojis/simple/factory-2.svg"),
};
