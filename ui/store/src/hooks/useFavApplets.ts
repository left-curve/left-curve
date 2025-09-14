import { useStorage } from "./useStorage.js";
import { useCallback } from "react";

import type { AppletMetadata } from "../types/applets.js";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<string[]>("app.applets", {
    initialValue: ["trade", "convert", "transfer", "create-account", "settings"],
    version: 1.8,
    sync: true,
    migrations: {
      "*": () => ["trade", "convert", "transfer", "create-account", "settings"],
    },
  });

  const addFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => [...prev, applet.id]);
  }, []);

  const removeFavApplet = useCallback((applet: AppletMetadata) => {
    setFavApplets((prev) => prev.filter((id) => id !== applet.id));
  }, []);

  return {
    favApplets,
    addFavApplet,
    removeFavApplet,
  };
}
