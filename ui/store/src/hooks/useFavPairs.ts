import { useCallback } from "react";
import { useStorage } from "./useStorage.js";

const FAVORITE_PAIR_VERSION = 1;

function migrateFavPairsFromV0ToV1(value: unknown): string[] {
  if (!Array.isArray(value)) return [];

  const pairs = new Set<string>();
  for (const pair of value) {
    if (typeof pair !== "string") continue;

    pairs.add(pair.replace("-", ""));
  }

  return [...pairs];
}

export function useFavPairs() {
  const [favPairs, setFavPairs] = useStorage<string[]>("favorites.pairs", {
    initialValue: [],
    version: FAVORITE_PAIR_VERSION,
    migrations: {
      0: migrateFavPairsFromV0ToV1,
    },
    sync: true,
  });

  const hasFavPair = useCallback((pair: string) => favPairs.includes(pair), [favPairs]);

  const addFavPair = useCallback(
    (pair: string) => setFavPairs((pairs) => (pairs.includes(pair) ? pairs : [...pairs, pair])),
    [setFavPairs],
  );

  const removeFavPair = useCallback(
    (pair: string) => setFavPairs((prev = []) => prev.filter((p) => p !== pair)),
    [setFavPairs],
  );

  const toggleFavPair = useCallback(
    (pair: string) => (hasFavPair(pair) ? removeFavPair(pair) : addFavPair(pair)),
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
