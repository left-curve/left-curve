import { useCallback } from "react";
import { useStorage } from "./useStorage.js";

export function useFavPairs() {
  const [favPairs, setFavPairs] = useStorage<string[]>("favorites.pairs", {
    initialValue: [],
    version: 0,
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
