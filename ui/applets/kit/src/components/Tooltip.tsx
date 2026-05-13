import { useControlledState } from "@left-curve/foundation";

import { motion, AnimatePresence } from "framer-motion";
import { tv, type VariantProps } from "tailwind-variants";
import {
  type ReactNode,
  useRef,
  useState,
  type PropsWithChildren,
  useEffect,
  useLayoutEffect,
  useCallback,
} from "react";
import { createPortal } from "react-dom";

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
  /** When true, the tooltip opens/closes on click instead of hover. */
  trigger?: "hover" | "click";
}

interface TooltipPosition {
  top: number;
  left: number;
  arrowOffset?: number;
  resolvedPlacement?: TooltipPlacement;
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
  trigger = "hover",
}) => {
  const [isOpen, setIsOpen] = useControlledState(controlledIsOpen, onOpenChange, false);
  const [coords, setCoords] = useState({ x: 0, y: 0 });
  const [position, setPosition] = useState<TooltipPosition | null>(null);
  const openTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);
  const closeTimeout = useRef<ReturnType<typeof setTimeout>>(undefined);
  const triggerRef = useRef<HTMLDivElement | null>(null);
  const panelRef = useRef<HTMLDivElement | null>(null);

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

  useEffect(() => {
    if (trigger !== "click" || !isOpen) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node) &&
        panelRef.current &&
        !panelRef.current.contains(e.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [trigger, isOpen]);

  const calculatePosition = useCallback(() => {
    if (!isOpen || placement === "auto" || !triggerRef.current || !panelRef.current) {
      return;
    }

    const trigger = triggerRef.current;
    const panelEl = panelRef.current;
    const triggerRect = trigger.getBoundingClientRect();
    const panelRect = panelEl.getBoundingClientRect();
    const gap = 10;
    const padding = 16;
    const viewportWidth = Math.min(window.innerWidth, document.documentElement.clientWidth);
    const viewportHeight = Math.min(window.innerHeight, document.documentElement.clientHeight);

    let top = 0;
    let left = 0;

    const triggerCenterX = triggerRect.left + triggerRect.width / 2;
    const triggerCenterY = triggerRect.top + triggerRect.height / 2;

    let resolvedPlacement = placement;

    // Auto-flip when there's not enough space
    if (placement === "top" && triggerRect.top - panelRect.height - gap < padding) {
      resolvedPlacement = "bottom";
    } else if (
      placement === "bottom" &&
      triggerRect.bottom + panelRect.height + gap > viewportHeight - padding
    ) {
      resolvedPlacement = "top";
    }

    switch (resolvedPlacement) {
      case "top":
        top = triggerRect.top - panelRect.height - gap;
        left = triggerCenterX - panelRect.width / 2;
        break;
      case "bottom":
        top = triggerRect.bottom + gap;
        left = triggerCenterX - panelRect.width / 2;
        break;
      case "left":
        top = triggerCenterY - panelRect.height / 2;
        left = triggerRect.left - panelRect.width - gap;
        break;
      case "right":
        top = triggerCenterY - panelRect.height / 2;
        left = triggerRect.right + gap;
        break;
    }

    if (left < padding) {
      left = padding;
    } else if (left + panelRect.width > viewportWidth - padding) {
      left = viewportWidth - panelRect.width - padding;
    }

    if (top < padding) {
      top = padding;
    } else if (top + panelRect.height > viewportHeight - padding) {
      top = viewportHeight - panelRect.height - padding;
    }

    const panelCenterX = left + panelRect.width / 2;
    const arrowOffset = triggerCenterX - panelCenterX - 6;
    const maxOffset = panelRect.width / 2 - 20;
    const clampedOffset = Math.max(-maxOffset, Math.min(maxOffset, arrowOffset));

    setPosition({
      top,
      left,
      arrowOffset: clampedOffset !== 0 ? clampedOffset : undefined,
      resolvedPlacement,
    });
  }, [isOpen, placement]);

  useLayoutEffect(() => {
    calculatePosition();
    requestAnimationFrame(() => {
      calculatePosition();
    });
  }, [calculatePosition]);

  const tooltipContent = (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          ref={panelRef}
          role="tooltip"
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.95 }}
          transition={{ duration: 0.15 }}
          style={
            placement === "auto"
              ? { top: coords.y + 12, left: coords.x + 12 }
              : position
                ? { top: position.top, left: position.left }
                : { visibility: "hidden", top: 0, left: 0 }
          }
          className={panel({ class: className })}
        >
          {title || description ? (
            <div className="flex flex-col">
              {title ? (
                <div className="diatype-xs-medium text-primitives-white-light-100">{title}</div>
              ) : null}
              {description ? (
                <div className="diatype-xs-regular text-primitives-gray-dark-200">
                  {description}
                </div>
              ) : null}
            </div>
          ) : (
            content
          )}
          {showArrow && placement !== "auto" ? (
            <span
              className={arrow({ placement: position?.resolvedPlacement ?? placement })}
              style={
                position?.arrowOffset !== undefined &&
                (placement === "top" || placement === "bottom")
                  ? { left: `calc(50% + ${position.arrowOffset}px)` }
                  : undefined
              }
            />
          ) : null}
        </motion.div>
      )}
    </AnimatePresence>
  );

  return (
    <div
      ref={triggerRef}
      onMouseEnter={trigger === "hover" ? handleOpen : undefined}
      onMouseLeave={trigger === "hover" ? handleClose : undefined}
      onMouseMove={trigger === "hover" ? handleMouseMove : undefined}
      onClick={trigger === "click" ? () => setIsOpen(!isOpen) : undefined}
      className="relative w-fit cursor-pointer text-[0px]"
    >
      {children ? children : <IconInfo className="text-ink-tertiary-500" />}
      {typeof document !== "undefined"
        ? createPortal(tooltipContent, document.body)
        : tooltipContent}
    </div>
  );
};

export const tooltipVariants = tv({
  slots: {
    panel:
      "fixed bg-primitives-gray-dark-950 text-primitives-gray-dark-200 p-3 rounded-2xl shadow-account-card max-w-[18rem] w-max text-left min-w-[8rem] diatype-xs-regular z-50",
    arrow: "absolute w-3 h-3 rotate-45 bg-primitives-gray-dark-950",
  },
  variants: {
    placement: {
      top: {
        arrow: "left-1/2 -translate-x-1/2 bottom-[-6px]",
      },
      bottom: {
        arrow: "left-1/2 -translate-x-1/2 top-[-6px]",
      },
      left: {
        arrow: "top-1/2 -translate-y-1/2 right-[-6px]",
      },
      right: {
        arrow: "top-1/2 -translate-y-1/2 left-[-6px]",
      },
      auto: {
        panel: "pointer-events-none z-[90]",
        arrow: "hidden",
      },
    },
  },
  defaultVariants: {
    placement: "top",
  },
});
