import { Button, IconTools } from "@left-curve/applets-kit";
import { useEffect, useState } from "react";
import { Workbox } from "workbox-window";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const AppUpdater: React.FC = () => {
  const [isLoading, setIsLoading] = useState(false);
  const [sw, setSw] = useState<ServiceWorker>();

  useEffect(() => {
    if (window.location.hostname === "localhost") return;
    if ("serviceWorker" in navigator) {
      const wb = new Workbox("/service-worker.js");

      wb.addEventListener("waiting", (event) => {
        if (!event.sw) return;
        setSw(event.sw);
      });

      wb.register();

      const intervalId = setInterval(wb.update, 60 * 60 * 1000);

      const handleVisibilityChange = () => {
        if (document.visibilityState === "visible") {
          wb.update();
        }
      };

      document.addEventListener("visibilitychange", handleVisibilityChange);

      return () => {
        clearInterval(intervalId);
        document.removeEventListener("visibilitychange", handleVisibilityChange);
      };
    }
  }, []);

  const updateApp = () => {
    if (!sw) return;
    setIsLoading(true);
    sw.postMessage({ type: "SKIP_WAITING" });
    setTimeout(() => {
      setIsLoading(false);
      window.location.reload();
    }, 1000);
  };

  if (!sw) return null;

  return (
    <div className="fixed bottom-5 right-5 flex flex-col bg-surface-primary-rice rounded-xl z-50">
      <div className="p-4 flex flex-col gap-4">
        <div className="w-12 h-12 rounded-full bg-red-bean-100 flex items-center justify-center text-red-bean-600">
          <IconTools />
        </div>
        <div className="flex flex-col gap-2 max-w-md">
          <h3 className="h4-bold text-primary-900">{m["appUpdate.title"]()}</h3>
          <p className="text-tertiary-500 diatype-m-regular">{m["appUpdate.description"]()}</p>
        </div>
      </div>
      <Button className="min-w-32" onClick={updateApp} isLoading={isLoading}>
        {m["appUpdate.updateButton"]()}
      </Button>
    </div>
  );
};
