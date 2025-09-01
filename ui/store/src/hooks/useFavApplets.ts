import { useStorage } from "./useStorage.js";
import { useCallback } from "react";

import type { AppletMetadata } from "../types/applets.js";

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
    setFavApplets((prev) => prev.filter((id) => id !== applet.id));
  }, []);

  return {
    favApplets,
    addFavApplet,
    removeFavApplet,
  };
}
