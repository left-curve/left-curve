import { useState } from "react";
import { IconGear, twMerge } from "@left-curve/applets-kit";
import { AnimatePresence, motion } from "framer-motion";
import { PANEL_LABELS, PANEL_ORDER, type PanelId } from "../hooks/useGridLayout";

type SettingsDrawerProps = {
  visibility: Record<PanelId, boolean>;
  isLocked: boolean;
  onTogglePanel: (panelId: PanelId) => void;
  onToggleLock: () => void;
  onReset: () => void;
};

export function SettingsDrawer({
  visibility,
  isLocked,
  onTogglePanel,
  onToggleLock,
  onReset,
}: SettingsDrawerProps) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setIsOpen((prev) => !prev)}
        className="flex items-center justify-center w-8 h-8 rounded-md text-ink-tertiary-500 hover:text-ink-primary hover:bg-surface-tertiary-rice transition-colors"
        aria-label="Layout settings"
      >
        <IconGear className="w-4 h-4" />
      </button>

      <AnimatePresence>
        {isOpen && (
          <>
            <div className="fixed inset-0 z-40" onClick={() => setIsOpen(false)} />
            <motion.div
              initial={{ opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.15 }}
              className="absolute right-0 top-full mt-2 z-50 w-[220px] rounded-md bg-surface-primary-rice shadow-account-card border border-outline-secondary-gray p-3 flex flex-col gap-3"
            >
              <div className="flex items-center justify-between">
                <span className="diatype-xs-medium text-ink-primary">Layout</span>
                <button
                  type="button"
                  onClick={onToggleLock}
                  className={twMerge(
                    "flex items-center gap-1.5 px-2 py-1 rounded-xs text-[11px] transition-colors",
                    isLocked
                      ? "bg-surface-tertiary-rice text-ink-secondary-700"
                      : "bg-primitives-red-light-100 text-primitives-red-light-600",
                  )}
                >
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
                    {isLocked ? (
                      <path
                        d="M9.5 5.5H2.5C1.95 5.5 1.5 5.95 1.5 6.5V10C1.5 10.55 1.95 11 2.5 11H9.5C10.05 11 10.5 10.55 10.5 10V6.5C10.5 5.95 10.05 5.5 9.5 5.5ZM4 5.5V3.5C4 2.4 4.9 1.5 6 1.5C7.1 1.5 8 2.4 8 3.5V5.5"
                        stroke="currentColor"
                        strokeWidth="1.2"
                        strokeLinecap="round"
                      />
                    ) : (
                      <path
                        d="M9.5 5.5H2.5C1.95 5.5 1.5 5.95 1.5 6.5V10C1.5 10.55 1.95 11 2.5 11H9.5C10.05 11 10.5 10.55 10.5 10V6.5C10.5 5.95 10.05 5.5 9.5 5.5ZM8 5.5V3.5C8 2.4 8.9 1.5 10 1.5"
                        stroke="currentColor"
                        strokeWidth="1.2"
                        strokeLinecap="round"
                      />
                    )}
                  </svg>
                  {isLocked ? "Locked" : "Unlocked"}
                </button>
              </div>

              <div className="h-px bg-outline-secondary-gray" />

              <div className="flex flex-col gap-1">
                <span className="diatype-xxs-medium text-ink-tertiary-500 mb-1">Panels</span>
                {PANEL_ORDER.map((panelId) => (
                  <label
                    key={panelId}
                    className="flex items-center justify-between py-1.5 px-1 rounded-xs hover:bg-surface-tertiary-rice cursor-pointer transition-colors"
                  >
                    <span className="diatype-xs-regular text-ink-secondary-700">
                      {PANEL_LABELS[panelId]}
                    </span>
                    <button
                      type="button"
                      onClick={() => onTogglePanel(panelId)}
                      className={twMerge(
                        "relative w-7 h-4 rounded-full transition-colors",
                        visibility[panelId]
                          ? "bg-primitives-red-light-400"
                          : "bg-outline-secondary-gray",
                      )}
                    >
                      <span
                        className={twMerge(
                          "absolute top-0.5 w-3 h-3 rounded-full bg-white transition-transform",
                          visibility[panelId] ? "left-3.5" : "left-0.5",
                        )}
                      />
                    </button>
                  </label>
                ))}
              </div>

              <div className="h-px bg-outline-secondary-gray" />

              <button
                type="button"
                onClick={() => {
                  onReset();
                  setIsOpen(false);
                }}
                className="w-full py-1.5 rounded-xs diatype-xs-medium text-ink-tertiary-500 hover:text-ink-primary hover:bg-surface-tertiary-rice transition-colors"
              >
                Reset to Default
              </button>
            </motion.div>
          </>
        )}
      </AnimatePresence>
    </div>
  );
}
