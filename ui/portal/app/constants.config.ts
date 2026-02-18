import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { AppletMetadata } from "@left-curve/store/types";

export const WS_URI = "wss://webrtc.dango.exchange";

export const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

export const PRIVY_ERRORS_MAPPING = {
  "User already has one email account linked": m["auth.errors.userNotFound"](),
  authFailed: m["auth.errors.authFailed"](),
  "User does not exist": m["auth.errors.userNotFound"](),
};

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
  convert: require("@left-curve/foundation/images/emojis/simple/swap.svg"),
  bridge: require("@left-curve/foundation/images/emojis/simple/moneybag.svg"),
  earn: require("@left-curve/foundation/images/emojis/simple/pig.svg"),
  transfer: require("@left-curve/foundation/images/emojis/simple/money.svg"),
  "create-account": require("@left-curve/foundation/images/emojis/simple/wizard.svg"),
  settings: require("@left-curve/foundation/images/emojis/simple/settings.svg"),
  devtool: require("@left-curve/foundation/images/emojis/simple/factory-2.svg"),
};

export const COINS = {
  SOL: require("@left-curve/foundation/images/coins/sol.svg"),
  USDC: require("@left-curve/foundation/images/coins/usdc.svg"),
  ETH: require("@left-curve/foundation/images/coins/eth.svg"),
  XRP: require("@left-curve/foundation/images/coins/xrp.svg"),
  BTC: require("@left-curve/foundation/images/coins/bitcoin.svg"),
};
