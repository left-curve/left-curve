import React from "react";

export function useInfiniteScroll(fetchData: () => void, hasMore: boolean) {
  const loadMoreRef = React.useRef<HTMLDivElement | null>(null);

  const handleIntersection = React.useCallback(
    (entries: IntersectionObserverEntry[]) => {
      const isIntersecting = entries[0]?.isIntersecting;
      if (isIntersecting && hasMore) {
        fetchData();
      }
    },
    [fetchData, hasMore],
  );

  React.useEffect(() => {
    const observer = new IntersectionObserver(handleIntersection);

    if (loadMoreRef.current) {
      observer.observe(loadMoreRef.current);
    }

    return () => observer.disconnect();
  }, [handleIntersection]);

  return { loadMoreRef };
}
