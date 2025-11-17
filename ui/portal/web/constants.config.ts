import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { AppletMetadata } from "@left-curve/store/types";

export const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

export const PRIVY_APP_ID = import.meta.env.PUBLIC_PRIVY_APP_ID;
export const PRIVY_CLIENT_ID = import.meta.env.PUBLIC_PRIVY_CLIENT_ID;

export const PRIVY_ERRORS_MAPPING = {
  "User already has one email account linked": m["auth.errors.userNotFound"](),
  authFailed: m["auth.errors.authFailed"](),
  "User does not exist": m["auth.errors.userNotFound"](),
};

export const WS_URI = import.meta.env.PUBLIC_WS_URI;

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
