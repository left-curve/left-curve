import { useStorage } from "@left-curve/store";

import type { AppletMetadata } from "@left-curve/applets-kit";
import { useCallback } from "react";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<string[]>("app.applets", {
    initialValue: ["transfer", "settings", "notifications", "simple-swap", "trade", "earn"],
    version: 1.6,
    sync: true,
    migrations: {
      1.5: (_: Record<string, AppletMetadata>) => {
        return ["transfer", "settings", "notifications", "simple-swap", "trade", "earn"];
      },
    },
  });

  const addFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => [...prev, applet.id]);
  }, []);

  const removeFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => {
      const index = prev.indexOf(applet.id);
      if (index > -1) prev.splice(index, 1);
      return prev;
    });
  }, []);

  return {
    favApplets,
    addFavApplet,
    removeFavApplet,
  };
}
