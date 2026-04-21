import { useStorage } from "./useStorage.js";
import { useCallback } from "react";

import type { AppletMetadata } from "../types/applets.js";

export function useFavApplets() {
  const [favApplets, setFavApplets] = useStorage<string[]>("app.applets", {
    initialValue: [
      "earn",
      "trade",
      // "convert",
      "bridge",
      "transfer",
      "create-account",
      "settings",
      "referral",
    ],
    version: 3.2,
    sync: true,
    migrations: {
      "*": () => [
        "earn",
        "trade",
        // "convert",
        "bridge",
        "transfer",
        "create-account",
        "settings",
        "referral",
      ],
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
    setFavApplets,
    removeFavApplet,
  };
}
