import { m } from "~/paraglide/messages";

import type { AppletMetadata } from "@left-curve/applets-kit";

export const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

export const WEBRTC_URI = import.meta.env.PUBLIC_WEBRTC_URI;

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
