import { twMerge } from "@left-curve/applets-kit";
import type { PropsWithChildren } from "react";
import { DRAGGABLE_PANELS, PANEL_BORDER_RADIUS, type PanelId } from "../hooks/useGridLayout";

type GridPanelProps = PropsWithChildren<{
  panelId: PanelId;
  isLocked: boolean;
  onClose: (panelId: PanelId) => void;
  className?: string;
}>;

export function GridPanel({ panelId, isLocked, onClose, className, children }: GridPanelProps) {
  const isDraggable = DRAGGABLE_PANELS.has(panelId);
  const borderRadius = PANEL_BORDER_RADIUS[panelId];

  return (
    <div
      className={twMerge(
        "relative flex flex-col h-full w-full bg-surface-primary-rice shadow-account-card overflow-clip",
        borderRadius,
        className,
      )}
    >
      {!isLocked && (
        <>
          {isDraggable && (
            <div className="grid-drag-handle absolute top-0 left-0 right-8 h-6 cursor-grab z-20 flex items-center justify-center">
              <svg
                width="16"
                height="4"
                viewBox="0 0 16 4"
                className="text-ink-tertiary-500 opacity-60"
              >
                <circle cx="2" cy="2" r="1.5" fill="currentColor" />
                <circle cx="8" cy="2" r="1.5" fill="currentColor" />
                <circle cx="14" cy="2" r="1.5" fill="currentColor" />
              </svg>
            </div>
          )}
          <button
            type="button"
            className="absolute top-1 right-1 z-30 flex items-center justify-center w-5 h-5 rounded-full bg-surface-tertiary-rice text-ink-tertiary-500 hover:text-ink-primary transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              onClose(panelId);
            }}
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path
                d="M1 1L9 9M9 1L1 9"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </>
      )}
      <div className="flex-1 min-h-0 flex flex-col overflow-y-auto overflow-x-hidden">
        {children}
      </div>
    </div>
  );
}
