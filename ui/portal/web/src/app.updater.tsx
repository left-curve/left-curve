import { Button, IconTools } from "@left-curve/applets-kit";
import { useState, useSyncExternalStore } from "react";

import { m } from "@left-curve/foundation/paraglide/messages.js";

type Listener = () => void;

let registration: ServiceWorkerRegistration | null = null;
const listeners = new Set<Listener>();

export function notifyUpdate(next: ServiceWorkerRegistration): void {
  registration = next;
  for (const listener of listeners) listener();
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): ServiceWorkerRegistration | null {
  return registration;
}

export const AppUpdater: React.FC = () => {
  const reg = useSyncExternalStore(subscribe, getSnapshot, () => null);
  const [isLoading, setIsLoading] = useState(false);

  if (!reg) return null;

  const updateApp = () => {
    setIsLoading(true);
    reg.waiting?.postMessage({ type: "SKIP_WAITING" });
  };

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
