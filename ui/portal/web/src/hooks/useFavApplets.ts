import { useStorage } from "@left-curve/store";

import { DEFAULT_FAV_APPLETS } from "~/constants";

import type { AppletMetadata } from "@left-curve/applets-kit";
import { useCallback } from "react";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<Record<string, AppletMetadata>>("app.applets", {
    initialValue: DEFAULT_FAV_APPLETS,
    version: 1.1,
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
