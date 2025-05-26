import { useStorage } from "@left-curve/store";

import { DEFAULT_FAV_APPLETS, APPLETS } from "~/constants";

import type { AppletMetadata } from "@left-curve/applets-kit";
import { useCallback } from "react";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<Record<string, AppletMetadata>>("app.applets", {
    initialValue: DEFAULT_FAV_APPLETS,
    version: 1.2,
    migrations: {
      1.1: (oldValue: Record<string, AppletMetadata>) => {
        return Object.keys(oldValue).reduce((acc, appletId) => {
          const applet = APPLETS.find((a) => a.id === appletId);
          if (applet) acc[appletId] = applet;
          return acc;
        }, Object.create({}));
      },
    },
  });

  const addFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => ({
      ...prev,
      [applet.id]: applet,
    }));
  }, []);

  const removeFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => {
      const { [applet.id]: _, ...rest } = prev;
      return rest;
    });
  }, []);

  return {
    favApplets,
    addFavApplet,
    removeFavApplet,
  };
}
