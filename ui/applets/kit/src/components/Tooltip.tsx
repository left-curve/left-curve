import type React from "react";

import { motion, AnimatePresence } from "framer-motion";
import { tv, type VariantProps } from "tailwind-variants";
import { useControlledState } from "#hooks/useControlledState.js";
import { type ReactNode, useRef, useState, type PropsWithChildren, useEffect } from "react";
import { twMerge } from "#utils/twMerge.js";

type TooltipPlacement = "top" | "bottom" | "left" | "right" | "auto";

export interface TooltipProps extends Omit<VariantProps<typeof tooltipVariants>, "placement"> {
  content: ReactNode | string;
  placement?: TooltipPlacement;
  delay?: number;
  closeDelay?: number;
  isOpen?: boolean;
  onOpenChange?: (isOpen: boolean) => void;
  className?: string;
}

export const Tooltip: React.FC<PropsWithChildren<TooltipProps>> = ({
  children,
  content,
  placement = "top",
  delay = 0,
  closeDelay = 50,
  isOpen: controlledIsOpen,
  onOpenChange,
  className,
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
      className="relative w-fit"
    >
      {children}

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
            className={twMerge(tooltipVariants({ placement }), className)}
          >
            {content}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export const tooltipVariants = tv({
  base: "bg-surface-secondary-rice text-secondary-700 p-2 rounded-xl shadow-account-card max-w-lg text-center min-w-[8rem] diatype-sm-regular",
  variants: {
    placement: {
      top: "absolute bottom-full left-1/2 !-translate-x-1/2 mb-2 z-50",
      bottom: "absolute top-full left-1/2 !-translate-x-1/2 mt-2 z-50",
      left: "absolute right-full top-1/2 !-translate-y-1/2 mr-2 z-50",
      right: "absolute left-full top-1/2 !-translate-y-1/2 ml-2 z-50",
      auto: "fixed z-[90] pointer-events-none",
    } as Record<TooltipPlacement, string>,
  },
  defaultVariants: {
    placement: "top",
  },
});
