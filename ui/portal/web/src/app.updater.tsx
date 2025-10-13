import { Button, useApp } from "@left-curve/applets-kit";
import { useEffect } from "react";
import { Workbox } from "workbox-window";

import { m } from "@left-curve/foundation/paraglide/messages.js";

export const AppUpdater: React.FC = () => {
  const { toast } = useApp();

  useEffect(() => {
    if (window.location.origin.includes("localhost")) return;
    if ("serviceWorker" in navigator) {
      const wb = new Workbox("/service-worker.js");

      wb.addEventListener("waiting", (event) => {
        if (!event.sw) return;
        const { sw } = event;
        if (navigator.serviceWorker.controller) {
          toast.maintenance(
            {
              title: m["appUpdate.title"](),
              description: ({ id }) => (
                <div className="text-ink-tertiary-500 diatype-xs-medium">
                  <span>{m["appUpdate.description"]()}</span>
                  <Button
                    as="span"
                    variant="link"
                    size="xs"
                    className="min-w-20"
                    onClick={() => {
                      sw.postMessage({ type: "SKIP_WAITING" });
                      toast.dismiss(id);
                      setTimeout(() => {
                        window.location.reload();
                      }, 1000);
                    }}
                  >
                    {m["appUpdate.updateButton"]()}
                  </Button>
                </div>
              ),
            },
            { duration: Number.POSITIVE_INFINITY },
          );
        } else {
          sw.postMessage({ type: "SKIP_WAITING" });
          setTimeout(() => {
            window.location.reload();
          }, 500);
        }
      });

      wb.register();

      const intervalId = setInterval(
        () => {
          wb.update();
        },
        60 * 60 * 1000,
      );

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

  return null;
};
