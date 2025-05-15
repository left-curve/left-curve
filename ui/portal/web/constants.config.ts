import { m } from "~/paraglide/messages";

import type { AppletMetadata } from "@left-curve/applets-kit";

export const DEFAULT_SESSION_EXPIRATION = 24 * 60 * 60 * 1000; // 24 hours

export const APPLETS = Array.from(
  { length: Object.keys(m).filter((k) => k.includes("applet")).length / 5 },
  (_, i) => {
    return {
      title: m[`applets.${i as 0}.title`](),
      description: m[`applets.${i as 0}.description`](),
      img: m[`applets.${i as 0}.img`](),
      keywords: m[`applets.${i as 0}.keywords`]().split(","),
      path: m[`applets.${i as 0}.path`](),
    } as AppletMetadata;
  },
);

export const DEFAULT_FAV_APPLETS = APPLETS.filter(({ path }) =>
  ["/transfer", "/swap", "/settings", "/notifications"].includes(path),
).reduce((acc, applet) => {
  acc[applet.path] = applet;
  return acc;
}, Object.create({}));
