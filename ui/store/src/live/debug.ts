import type { LiveResourceDebugState } from "./types.js";

type ResourceDebugGetter = () => LiveResourceDebugState["resources"][string];
type DebugWindow = Window & { __DANGO_LIVE_RESOURCES__?: LiveResourceDebugState };

const resourceDebugGetters = new Map<string, ResourceDebugGetter>();

function getBrowserWindow(): DebugWindow | undefined {
  if (typeof window === "undefined") return undefined;
  return window as DebugWindow;
}

export function registerLiveResourceDebug(name: string, getDebugState: ResourceDebugGetter) {
  resourceDebugGetters.set(name, getDebugState);
  syncLiveResourceDebug();
}

export function syncLiveResourceDebug() {
  const browserWindow = getBrowserWindow();
  if (!browserWindow) return;

  const resources: LiveResourceDebugState["resources"] = {};
  for (const [name, getDebugState] of resourceDebugGetters) {
    resources[name] = getDebugState();
  }

  browserWindow.__DANGO_LIVE_RESOURCES__ = { resources };
}
