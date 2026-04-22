import { useEffect, useRef, useState } from "react";

import type { PredictPointsEvent } from "./pointsApi.js";

export type UsePredictPointsParameters = {
  pointsUrl: string;
  userIndex: number | undefined;
  enabled?: boolean;
};

export function usePredictPoints(parameters: UsePredictPointsParameters) {
  const { pointsUrl, userIndex, enabled = true } = parameters;
  const [data, setData] = useState<PredictPointsEvent | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!enabled || userIndex === undefined) return;

    const url = `${pointsUrl}/predict/points/${userIndex}`;
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.onmessage = (event) => {
      try {
        const parsed: PredictPointsEvent = JSON.parse(event.data);
        setData(parsed);
      } catch {
        // ignore malformed messages
      }
    };

    es.onerror = () => {
      es.close();
      eventSourceRef.current = null;
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, [pointsUrl, userIndex, enabled]);

  return { predictedPoints: data };
}
