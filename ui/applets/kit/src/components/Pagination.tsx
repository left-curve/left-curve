import { twMerge } from "#utils/twMerge.js";
import type React from "react";
import { useState, useId } from "react";
import { tv, type VariantProps } from "tailwind-variants";
import { IconChevronRight } from "./icons/IconChevronRight";
import { IconChevronLeft } from "./icons/IconChevronLeft";

import { motion, AnimatePresence } from "framer-motion";

interface Props extends VariantProps<typeof pagnationVariants> {
  total: number;
  initialPage?: number; // The initial page (uncontrolled)
  page?: number; // The current page (controlled)
  onPageChange?: (page: number) => void; // Callback function to handle page changes.
  siblings?: number; // The number of pages to show before and after the current page.
  boundaries?: number; // The number of pages to show at the beginning and end of the pagination.
  id?: string; // Optional ID for the pagination component if more than one is used
}

export const Pagination: React.FC<Props> = ({
  total,
  page: controlledPage,
  initialPage = 1,
  onPageChange,
  siblings = 1,
  boundaries = 1,
  isDisabled,
  variant,
  id,
}) => {
  const styles = pagnationVariants({ variant, isDisabled });

  const [uncontrolledPage, setUncontrolledPage] = useState(initialPage);
  const currentPage = controlledPage ?? uncontrolledPage;

  const setPage = (newPage: number) => {
    if (onPageChange) onPageChange(newPage);
    else setUncontrolledPage(newPage);
  };

  const range = getPaginationRange({
    total,
    currentPage,
    siblings,
    boundaries,
  });

  return (
    <motion.div
      className={twMerge(styles.base(), "flex items-center justify-center gap-1 ")}
      layout
    >
      <button
        type="button"
        onClick={() => setPage(currentPage - 1)}
        disabled={currentPage === 1 || isDisabled}
        className={twMerge(styles.item(), currentPage === 1 && "opacity-60", "mr-3 text-blue-500")}
      >
        <IconChevronLeft className="w-5 h-5" />
      </button>

      <AnimatePresence>
        {range.map((item, index) => {
          const key = typeof item === "string" ? `ellipsis-${index}` : `page-${item}`;
          const isCurrent = item === currentPage;

          if (item === "...") {
            return (
              <span key={key} className="px-2 text-blue-400 select-none">
                ...
              </span>
            );
          }
          return (
            <motion.button
              layout
              type="button"
              key={key}
              onClick={() => setPage(item as number)}
              className={twMerge(styles.item(), "relative")}
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ duration: 0.2 }}
            >
              <span className="relative z-10">{item}</span>
              {isCurrent && (
                <motion.span
                  layoutId={`pagination-underline-${id ?? useId()}`}
                  className="absolute left-0 top-0 w-full h-full rounded-sm bg-blue-100"
                />
              )}
            </motion.button>
          );
        })}
      </AnimatePresence>

      <button
        type="button"
        onClick={() => setPage(currentPage + 1)}
        disabled={currentPage === total || isDisabled}
        className={twMerge(
          styles.item(),
          currentPage === total && "opacity-60",
          "ml-3 text-blue-500",
        )}
      >
        <IconChevronRight className="w-5 h-5" />
      </button>
    </motion.div>
  );
};

function getPaginationRange({
  total,
  currentPage,
  siblings,
  boundaries,
}: {
  total: number;
  currentPage: number;
  siblings: number;
  boundaries: number;
}): (number | "...")[] {
  const startPages = [...Array(boundaries)].map((_, i) => i + 1);
  const endPages = [...Array(boundaries)].map((_, i) => total - boundaries + 1 + i);

  const siblingStart = Math.max(currentPage - siblings, boundaries + 2);
  const siblingEnd = Math.min(currentPage + siblings, total - boundaries - 1);

  const pages = [
    ...startPages,
    ...(siblingStart > boundaries + 2
      ? ["..."]
      : siblingStart > boundaries + 1
        ? [boundaries + 1]
        : []),
    ...Array.from({ length: siblingEnd - siblingStart + 1 }, (_, i) => siblingStart + i),
    ...(siblingEnd < total - boundaries - 1
      ? ["..."]
      : siblingEnd < total - boundaries
        ? [total - boundaries]
        : []),
    ...endPages,
  ];

  return [...new Set(pages)] as (number | "...")[];
}

const pagnationVariants = tv(
  {
    slots: {
      base: "",
      item: "flex items-center justify-center w-8 h-8 rounded-sm exposure-sm-italic hover:bg-blue-50 transition-all text-blue-600",
    },
    variants: {
      variant: {
        default: "",
        text: "",
      },
      isDisabled: {
        true: "pointer-events-none cursor-not-allowed",
      },
    },
    defaultVariants: {
      size: "md",
      variant: "default",
      isDisabled: false,
    },
  },
  {
    twMerge: true,
  },
);
