import { useCallback } from "react";
import { useStorage } from "./useStorage.js";

import type { PairId } from "@left-curve/dango/types";

export const pairEquals = (a: PairId, b: PairId) =>
  a.baseDenom === b.baseDenom && a.quoteDenom === b.quoteDenom;

export function useFavPairs() {
  const [favPairs, setFavPairs] = useStorage<PairId[]>("app.pairs", {
    initialValue: [],
    version: 0,
    sync: true,
  });

  const hasFavPair = useCallback(
    (pair: PairId) => (favPairs ?? []).some((p) => pairEquals(p, pair)),
    [favPairs],
  );

  const addFavPair = useCallback(
    (pair: PairId) =>
      setFavPairs((prev = []) => (prev.some((p) => pairEquals(p, pair)) ? prev : [...prev, pair])),
    [setFavPairs],
  );

  const removeFavPair = useCallback(
    (pair: PairId) => setFavPairs((prev = []) => prev.filter((p) => !pairEquals(p, pair))),
    [setFavPairs],
  );

  const toggleFavPair = useCallback(
    (pair: PairId) => (hasFavPair(pair) ? removeFavPair(pair) : addFavPair(pair)),
    [hasFavPair, removeFavPair, addFavPair],
  );

  return {
    favPairs,
    hasFavPair,
    addFavPair,
    setFavPairs,
    removeFavPair,
    toggleFavPair,
  };
}
