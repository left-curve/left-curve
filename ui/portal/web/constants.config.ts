import { m } from "~/paraglide/messages";

import type { AppletMetadata } from "@left-curve/applets-kit";

export const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

export const WEBRTC_URI = import.meta.env.PUBLIC_WEBRTC_URI;

export const APPLETS = Array.from(
  { length: Object.keys(m).filter((k) => k.includes("applet")).length },
  (_, i) => {
    if (m[`applets.${i as 0}.id`]) {
      return {
        id: m[`applets.${i as 0}.id`](),
        title: m[`applets.${i as 0}.title`](),
        description: m[`applets.${i as 0}.description`](),
        img: m[`applets.${i as 0}.img`](),
        keywords: m[`applets.${i as 0}.keywords`]().split(","),
        path: m[`applets.${i as 0}.path`](),
      } as AppletMetadata;
    }
  },
).filter(Boolean) as AppletMetadata[];

export const DEFAULT_FAV_APPLETS = APPLETS.filter(({ id }) =>
  ["transfer", "settings", "notifications", "simple-swap", "trade", "earn"].includes(id),
).reduce((acc, applet) => {
  acc[applet.id] = applet;
  return acc;
}, Object.create({}));
