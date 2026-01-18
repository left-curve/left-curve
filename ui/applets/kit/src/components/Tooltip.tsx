import { useControlledState } from "@left-curve/foundation";

import { motion, AnimatePresence } from "framer-motion";
import { tv, type VariantProps } from "tailwind-variants";
import { type ReactNode, useRef, useState, type PropsWithChildren, useEffect } from "react";

import type React from "react";
import { IconInfo } from "./icons/IconInfo";

type TooltipPlacement = "top" | "bottom" | "left" | "right" | "auto";

export interface TooltipProps extends Omit<VariantProps<typeof tooltipVariants>, "placement"> {
  content?: ReactNode | string;
  title?: ReactNode | string;
  description?: ReactNode | string;
  placement?: TooltipPlacement;
  delay?: number;
  closeDelay?: number;
  isOpen?: boolean;
  onOpenChange?: (isOpen: boolean) => void;
  className?: string;
  showArrow?: boolean;
}

export const Tooltip: React.FC<PropsWithChildren<TooltipProps>> = ({
  children,
  content,
  title,
  description,
  placement = "top",
  delay = 0,
  closeDelay = 50,
  isOpen: controlledIsOpen,
  onOpenChange,
  className,
  showArrow = true,
}) => {
  const [isOpen, setIsOpen] = useControlledState(controlledIsOpen, onOpenChange, false);
  const [coords, setCoords] = useState({ x: 0, y: 0 });
  const openTimeout = useRef<ReturnType<typeof setTimeout>>();
  const closeTimeout = useRef<ReturnType<typeof setTimeout>>();
  const triggerRef = useRef<HTMLDivElement | null>(null);

  const handleOpen = () => {
    clearTimeout(closeTimeout.current);
    openTimeout.current = setTimeout(() => setIsOpen(true), delay);
  };

  const handleClose = () => {
    clearTimeout(openTimeout.current);
    closeTimeout.current = setTimeout(() => setIsOpen(false), closeDelay);
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (placement === "auto") {
      setCoords({ x: e.clientX, y: e.clientY });
    }
  };

  const { panel, arrow } = tooltipVariants({ placement });

  useEffect(() => {
    return () => {
      clearTimeout(openTimeout.current);
      clearTimeout(closeTimeout.current);
    };
  }, []);

  return (
    <div
      ref={triggerRef}
      onMouseEnter={handleOpen}
      onMouseLeave={handleClose}
      onMouseMove={handleMouseMove}
      className="relative w-fit cursor-pointer"
    >
      {children ? children : <IconInfo className="text-ink-tertiary-500" />}

      <AnimatePresence>
        {isOpen && (
          <motion.div
            role="tooltip"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.15 }}
            style={
              placement === "auto"
                ? {
                    top: coords.y + 12,
                    left: coords.x + 12,
                    position: "fixed",
                    pointerEvents: "none",
                  }
                : undefined
            }
            className={panel({ class: className })}
          >
            {title || description ? (
              <div className="flex flex-col">
                {title ? (
                  <p className="diatype-sm-bold text-primitives-white-light-100">{title}</p>
                ) : null}
                {description ? (
                  <p className="diatype-sm-regular text-primitives-gray-dark-200">{description}</p>
                ) : null}
              </div>
            ) : (
              content
            )}
            {showArrow && placement !== "auto" ? <span className={arrow()} /> : null}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export const tooltipVariants = tv({
  slots: {
    panel:
      "relative bg-primitives-gray-dark-950 text-primitives-gray-dark-200 p-3 rounded-2xl shadow-account-card max-w-[18rem] w-max text-left min-w-[8rem] diatype-sm-regular",
    arrow: "absolute w-3 h-3 rotate-45 bg-primitives-gray-dark-950",
  },
  variants: {
    placement: {
      top: {
        panel: "absolute bottom-full left-1/2 !-translate-x-1/2 mb-3 z-50",
        arrow: "left-1/2 -translate-x-1/2 bottom-[-6px]",
      },
      bottom: {
        panel: "absolute top-full left-1/2 !-translate-x-1/2 mt-3 z-50",
        arrow: "left-1/2 -translate-x-1/2 top-[-6px]",
      },
      left: {
        panel: "absolute right-full top-1/2 !-translate-y-1/2 mr-3 z-50",
        arrow: "top-1/2 -translate-y-1/2 right-[-6px]",
      },
      right: {
        panel: "absolute left-full top-1/2 !-translate-y-1/2 ml-3 z-50",
        arrow: "top-1/2 -translate-y-1/2 left-[-6px]",
      },
      auto: {
        panel: "fixed z-[90] pointer-events-none",
        arrow: "hidden",
      },
    },
  },
  defaultVariants: {
    placement: "top",
  },
});
