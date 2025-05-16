import { useStorage } from "@left-curve/store";

import { DEFAULT_FAV_APPLETS } from "~/constants";

import type { AppletMetadata } from "@left-curve/applets-kit";
import { useCallback } from "react";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<Record<string, AppletMetadata>>(
    "app.favorite_applets",
    { initialValue: DEFAULT_FAV_APPLETS },
  );

  const addFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => ({
      ...prev,
      [applet.path]: applet,
    }));
  }, []);

  const removeFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => {
      const { [applet.path]: _, ...rest } = prev;
      return rest;
    });
  }, []);

  return {
    favApplets,
    addFavApplet,
    removeFavApplet,
  };
}
