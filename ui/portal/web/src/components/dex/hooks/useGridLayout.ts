import { useCallback, useMemo, useState } from "react";
import type { Layout } from "react-grid-layout";

export type PanelId = "chart" | "orderbook" | "history" | "trademenu";

export const PANEL_LABELS: Record<PanelId, string> = {
  chart: "Chart",
  orderbook: "Order Book",
  history: "History",
  trademenu: "Trade Menu",
};

export const GRID_COLS = 12;
export const GRID_MARGIN: [number, number] = [12, 12];
export const GRID_CONTAINER_PADDING: [number, number] = [0, 0];
export const GRID_MAX_ROWS = 32;

const STORAGE_KEY = "pro-trade-grid-layout";
const VISIBILITY_KEY = "pro-trade-panel-visibility";

const DEFAULT_LAYOUT: Layout[] = [
  { i: "chart", x: 0, y: 0, w: 8, h: 19, minW: 6, minH: 8 },
  { i: "orderbook", x: 8, y: 0, w: 2, h: 19, minW: 1, minH: 8, maxW: 4 },
  { i: "history", x: 0, y: 19, w: 10, h: 13, minW: 6, minH: 5 },
  { i: "trademenu", x: 10, y: 0, w: 2, h: 32, minW: 1, minH: 10, maxW: 4 },
];

const SIDE_PANELS: ReadonlySet<PanelId> = new Set(["orderbook", "trademenu"]);

export const DRAGGABLE_PANELS: ReadonlySet<PanelId> = new Set([
  "chart",
  "orderbook",
  "history",
  "trademenu",
]);

const DEFAULT_VISIBILITY: Record<PanelId, boolean> = {
  chart: true,
  orderbook: true,
  history: true,
  trademenu: true,
};

export const PANEL_BORDER_RADIUS: Record<PanelId, string> = {
  chart: "rounded-r-[12px]",
  orderbook: "rounded-[12px]",
  history: "rounded-tr-[12px]",
  trademenu: "rounded-l-[12px] rounded-br-none",
};

const PANEL_ORDER: PanelId[] = ["chart", "orderbook", "history", "trademenu"];
export { PANEL_ORDER };

function loadLayout(): Layout[] {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) return DEFAULT_LAYOUT;
    const parsed = JSON.parse(stored) as Layout[];
    if (!Array.isArray(parsed) || parsed.length === 0) return DEFAULT_LAYOUT;
    return parsed.map((item) => {
      const defaults = DEFAULT_LAYOUT.find((d) => d.i === item.i);
      return { ...defaults, ...item };
    });
  } catch {
    return DEFAULT_LAYOUT;
  }
}

function saveLayout(layout: Layout[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {}
}

function loadVisibility(): Record<PanelId, boolean> {
  try {
    const stored = localStorage.getItem(VISIBILITY_KEY);
    if (!stored) return DEFAULT_VISIBILITY;
    return { ...DEFAULT_VISIBILITY, ...JSON.parse(stored) };
  } catch {
    return DEFAULT_VISIBILITY;
  }
}

function saveVisibility(visibility: Record<PanelId, boolean>) {
  try {
    localStorage.setItem(VISIBILITY_KEY, JSON.stringify(visibility));
  } catch {}
}

const FULL_WIDTH_BREAKPOINT = 1440;
const SHRINK_FACTOR = 0.75;

function computeResponsiveLayout(base: Layout[], containerWidth: number): Layout[] {
  if (containerWidth >= FULL_WIDTH_BREAKPOINT || containerWidth === 0) return base;

  const ratio = containerWidth / FULL_WIDTH_BREAKPOINT;
  const sideShrink = Math.max(SHRINK_FACTOR, ratio);

  let colsFreed = 0;

  const adjusted = base.map((item) => {
    if (!SIDE_PANELS.has(item.i as PanelId)) return item;
    const newW = Math.max(item.minW ?? 1, Math.round(item.w * sideShrink));
    colsFreed += item.w - newW;
    return { ...item, w: newW };
  });

  if (colsFreed <= 0) return adjusted;

  const mainPanels = adjusted.filter((item) => !SIDE_PANELS.has(item.i as PanelId));
  const colsPerMain = Math.floor(colsFreed / mainPanels.length);
  let remainder = colsFreed - colsPerMain * mainPanels.length;

  return adjusted.map((item) => {
    if (SIDE_PANELS.has(item.i as PanelId)) return item;
    const extra = remainder > 0 ? 1 : 0;
    if (extra) remainder--;
    return { ...item, w: item.w + colsPerMain + extra };
  });
}

export function useGridLayout(containerWidth = 0) {
  const [layout, setLayout] = useState<Layout[]>(loadLayout);
  const [visibility, setVisibility] = useState<Record<PanelId, boolean>>(loadVisibility);
  const [isLocked, setIsLocked] = useState(true);

  const onLayoutChange = useCallback(
    (newLayout: Layout[]) => {
      if (isLocked) return;
      const merged = newLayout.map((item) => {
        const defaults = DEFAULT_LAYOUT.find((d) => d.i === item.i);
        return {
          ...item,
          minW: defaults?.minW,
          minH: defaults?.minH,
          maxW: defaults?.maxW,
          maxH: defaults?.maxH,
        };
      });
      setLayout(merged);
      saveLayout(merged);
    },
    [isLocked],
  );

  const togglePanel = useCallback((panelId: PanelId) => {
    setVisibility((prev) => {
      const next = { ...prev, [panelId]: !prev[panelId] };
      saveVisibility(next);
      return next;
    });
  }, []);

  const resetLayout = useCallback(() => {
    setLayout(DEFAULT_LAYOUT);
    setVisibility(DEFAULT_VISIBILITY);
    saveLayout(DEFAULT_LAYOUT);
    saveVisibility(DEFAULT_VISIBILITY);
  }, []);

  const toggleLock = useCallback(() => {
    setIsLocked((prev) => !prev);
  }, []);

  const responsiveLayout = useMemo(
    () => computeResponsiveLayout(layout, containerWidth),
    [layout, containerWidth],
  );

  return {
    layout: responsiveLayout.filter((item) => visibility[item.i as PanelId]),
    fullLayout: responsiveLayout,
    visibility,
    isLocked,
    onLayoutChange,
    togglePanel,
    resetLayout,
    toggleLock,
  };
}
