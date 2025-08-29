import { useEffect, useRef } from "react";
import type { RefObject } from "react";

const defaultEvents = ["mousedown", "touchstart"];

export function useClickAway<E extends Event = Event>(
  ref: RefObject<HTMLElement | null>,
  onClickAway: (event: E) => void,
  events: string[] = defaultEvents,
) {
  const savedCallback = useRef(onClickAway);

  useEffect(() => {
    savedCallback.current = onClickAway;
  }, [onClickAway]);

  useEffect(() => {
    const handler = (event: Event) => {
      const { current: el } = ref;
      el && !el.contains(event?.target as Node) && savedCallback.current(event as E);
    };
    for (const eventName of events) {
      window?.document.addEventListener(eventName, handler);
    }
    return () => {
      for (const eventName of events) {
        window?.document.removeEventListener(eventName, handler);
      }
    };
  }, [events, ref]);
}
