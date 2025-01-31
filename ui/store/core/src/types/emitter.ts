export type EventMap = Record<string, object | never>;

export type EventKey<eventMap extends EventMap> = string & keyof eventMap;

export type EventFn<parameters extends unknown[] = any[]> = (...parameters: parameters) => void;

export type EventData<
  eventMap extends EventMap,
  eventName extends keyof eventMap,
> = (eventMap[eventName] extends [never] ? unknown : eventMap[eventName]) & {
  uid: string;
};

export type Emitter<eventMap extends EventMap> = {
  on<key extends EventKey<eventMap>>(
    eventName: key,
    fn: EventFn<
      eventMap[key] extends [never] ? [{ uid: string }] : [data: eventMap[key] & { uid: string }]
    >,
  ): void;

  once<key extends EventKey<eventMap>>(
    eventName: key,
    fn: EventFn<
      eventMap[key] extends [never] ? [{ uid: string }] : [data: eventMap[key] & { uid: string }]
    >,
  ): void;

  off<key extends EventKey<eventMap>>(
    eventName: key,
    fn: EventFn<
      eventMap[key] extends [never] ? [{ uid: string }] : [data: eventMap[key] & { uid: string }]
    >,
  ): void;

  emit<key extends EventKey<eventMap>>(
    eventName: key,
    ...params: eventMap[key] extends [never] ? [] : [data: eventMap[key]]
  ): void;

  listenerCount<key extends EventKey<eventMap>>(eventName: key): number;
};
