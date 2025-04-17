export function createEventBus<eventMap extends Record<string, any>>() {
  type EventKey = keyof eventMap;

  const listeners: {
    [K in EventKey]?: Array<(payload: eventMap[K]) => void>;
  } = {};

  return {
    publish<K extends EventKey>(event: K, payload: eventMap[K]) {
      listeners[event]?.forEach((fn) => fn(payload));
    },
    subscribe<K extends EventKey>(event: K, callback: (payload: eventMap[K]) => void) {
      listeners[event] ??= [];
      listeners[event]!.push(callback);
      return () => {
        listeners[event] = listeners[event]!.filter((fn) => fn !== callback);
      };
    },
  };
}
