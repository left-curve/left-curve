import { useEffect, useRef } from "react";
import type { RefObject } from "react";

const defaultEvents = ["mousedown", "touchstart"];

export function useClickAway<E extends Event = Event>(
  ref: RefObject<HTMLElement | null>,
  onClickAway: (event: E) => void,
  events: string[] = defaultEvents,
  ignoreRefs: RefObject<HTMLElement | null>[] = [],
) {
  const savedCallback = useRef(onClickAway);
  const savedIgnoreRefs = useRef(ignoreRefs);

  useEffect(() => {
    savedCallback.current = onClickAway;
  }, [onClickAway]);

  useEffect(() => {
    savedIgnoreRefs.current = ignoreRefs;
  }, [ignoreRefs]);

  useEffect(() => {
    const handler = (event: Event) => {
      const { current: el } = ref;
      const target = event?.target as Node;

      const isInsideIgnored = savedIgnoreRefs.current.some(
        (ignoreRef) => ignoreRef.current?.contains(target),
      );

      if (isInsideIgnored) return;

      el && !el.contains(target) && savedCallback.current(event as E);
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
