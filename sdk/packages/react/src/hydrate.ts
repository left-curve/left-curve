"use client";

import { hydrate } from "@leftcurve/connect-kit";
import type { Config, State } from "@leftcurve/types";
import { type ReactElement, useEffect, useRef } from "react";

export type HydrateProps = {
  config: Config;
  initialState?: State;
  reconnectOnMount?: boolean;
};

export function Hydrate(parameters: React.PropsWithChildren<HydrateProps>) {
  const { children, config, initialState, reconnectOnMount = true } = parameters;

  const { onMount } = hydrate(config, {
    initialState,
    reconnectOnMount,
  });

  // Hydrate for non-SSR
  if (!config._internal.ssr) onMount();

  // Hydrate for SSR
  const active = useRef(true);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `queryKey` not required
  useEffect(() => {
    if (!active.current) return;
    if (!config._internal.ssr) return;
    onMount();
    return () => {
      active.current = false;
    };
  }, []);

  return children as ReactElement;
}
