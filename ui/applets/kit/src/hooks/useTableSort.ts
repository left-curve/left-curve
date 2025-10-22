import { useCallback, useMemo, useState } from "react";

export type Dir = "asc" | "desc";
export type SortKeys<T, K extends string> = Record<
  K,
  (row: T) => string | number | null | undefined
>;

type UseSortOpts<T, K extends string> = {
  data: T[];
  sortKeys: SortKeys<T, K>;
  initialKey: K;
  initialDir?: Dir;
  groupFirst?: (row: T) => boolean;
  onChange?: (state: { sortKey: K; sortDir: Dir }) => void;
};

export function useTableSort<T, K extends string>({
  data,
  sortKeys,
  initialKey,
  initialDir = "asc",
  groupFirst,
  onChange,
}: UseSortOpts<T, K>) {
  const [sortKey, setSortKey] = useState<K>(initialKey);
  const [sortDir, setSortDir] = useState<Dir>(initialDir);

  const toggleSort = useCallback(
    (col: K) => {
      setSortKey((prev) => {
        if (prev !== col) {
          setSortDir("asc");
          onChange?.({ sortKey: col, sortDir: "asc" });
          return col;
        }
        const nextDir: Dir = sortDir === "asc" ? "desc" : "asc";
        setSortDir(nextDir);
        onChange?.({ sortKey: col, sortDir: nextDir });
        return prev;
      });
    },
    [onChange, sortDir],
  );

  const getVal = useCallback((row: T, col: K) => sortKeys[col](row), [sortKeys]);

  const sortedData = useMemo(() => {
    return data
      .map((row, idx) => ({
        row,
        idx,
        grp: groupFirst ? !!groupFirst(row) : false,
        val: getVal(row, sortKey),
      }))
      .sort((a, b) => {
        if (a.grp !== b.grp) return a.grp ? -1 : 1;

        const va = a.val;
        const vb = b.val;
        let cmp = 0;

        if (typeof va === "number" && typeof vb === "number") {
          cmp = va === vb ? 0 : va < vb ? -1 : 1;
        } else {
          const sa = (va ?? "").toString().toUpperCase();
          const sb = (vb ?? "").toString().toUpperCase();
          cmp = sa === sb ? 0 : sa < sb ? -1 : 1;
        }

        if (cmp !== 0) return sortDir === "asc" ? cmp : -cmp;
        return a.idx - b.idx;
      })
      .map((x) => x.row);
  }, [data, groupFirst, getVal, sortKey, sortDir]);

  return { sortedData, sortKey, sortDir, toggleSort };
}
