import { Button } from "@left-curve/applets-kit";
import { useApp } from "@left-curve/foundation";
import { useEffect, useSyncExternalStore } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type UpdateState = { registration: ServiceWorkerRegistration; version: number };

let current: UpdateState | null = null;
const listeners = new Set<() => void>();

export function notifyUpdate(registration: ServiceWorkerRegistration): void {
  current = { registration, version: (current?.version ?? 0) + 1 };
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): UpdateState | null {
  return current;
}

const TOAST_ID = "app-update";

export const AppUpdater: React.FC = () => {
  const update = useSyncExternalStore(subscribe, getSnapshot, () => null);
  const { toast } = useApp();

  useEffect(() => {
    if (!update) return;
    const sw = update.registration.waiting;
    if (!sw) return;
    toast.dismiss(TOAST_ID);
    toast.warning(
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
              }}
            >
              {m["appUpdate.updateButton"]()}
            </Button>
          </div>
        ),
      },
      { id: TOAST_ID, duration: Number.POSITIVE_INFINITY },
    );
  }, [update, toast]);

  return null;
};
