import { useStorage } from "./useStorage.js";
import { useCallback } from "react";

import type { AppletMetadata } from "../types/applets.js";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<string[]>("app.applets", {
    initialValue: ["transfer", "settings", "convert", "trade", "earn"],
    version: 1.7,
    sync: true,
    migrations: {
      "*": () => ["transfer", "settings", "convert", "trade", "earn"],
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
