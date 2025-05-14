import { twMerge } from "#utils/twMerge.js";
import type React from "react";
import { useState, useId, useMemo } from "react";
import { tv, type VariantProps } from "tailwind-variants";
import { IconChevronRight } from "./icons/IconChevronRight";
import { IconChevronLeft } from "./icons/IconChevronLeft";

import { motion, AnimatePresence } from "framer-motion";

interface Props extends VariantProps<typeof pagnationVariants> {
  total: number;
  siblings?: number;
  boundaries?: number;
  id?: string;
  page?: number;
  initialPage?: number;
  onPageChange?: (page: number) => void;
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
  const internalId = useId();
  const [uncontrolledPage, setUncontrolledPage] = useState(initialPage);
  const currentPage = controlledPage ?? uncontrolledPage;

  const setPage = (newPage: number) => {
    if (onPageChange) onPageChange(newPage);
    else setUncontrolledPage(newPage);
  };

  const range = useMemo(
    () =>
      getPaginationRange({
        total,
        currentPage,
        siblings,
        boundaries,
      }),
    [total, currentPage, siblings, boundaries],
  );

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

      {variant === "text" ? (
        <AnimatePresence mode="wait">
          <p>
            Page{" "}
            <motion.span
              key={currentPage}
              initial={{ scale: 0, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0, opacity: 0 }}
              transition={{
                duration: 0.2,
              }}
              className="min-w-[1rem] inline-block text-center"
            >
              {currentPage}
            </motion.span>{" "}
            of {total}
          </p>
        </AnimatePresence>
      ) : (
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
                    layoutId={`pagination-underline-${id ?? internalId}`}
                    className="absolute left-0 top-0 w-full h-full rounded-sm bg-blue-100"
                  />
                )}
              </motion.button>
            );
          })}
        </AnimatePresence>
      )}

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
}): (number | string)[] {
  const totalBoundaryPages = boundaries * 2;

  if (total <= totalBoundaryPages) {
    return Array.from({ length: total }, (_, i) => i + 1);
  }

  const startPages: number[] = [];
  for (let i = 1; i <= Math.min(boundaries, total); i++) {
    startPages.push(i);
  }

  const endPages: number[] = [];
  for (let i = Math.max(1, total - boundaries + 1); i <= total; i++) {
    endPages.push(i);
  }

  const siblingStart = Math.max(1, currentPage - siblings);
  const siblingEnd = Math.min(total, currentPage + siblings);

  const middlePages: number[] = [];
  for (let i = siblingStart; i <= siblingEnd; i++) {
    middlePages.push(i);
  }

  let assembledPages: (number | "...")[] = [];

  assembledPages = [...startPages];

  if (middlePages.length > 0 && middlePages[0] > startPages[startPages.length - 1] + 1) {
    if (
      middlePages[0] > startPages[startPages.length - 1] + 2 &&
      startPages[startPages.length - 1] + 1 <= total - boundaries
    ) {
      assembledPages.push("...");
    } else if (middlePages[0] === startPages[startPages.length - 1] + 2) {
      if (!endPages.includes(startPages[startPages.length - 1] + 1)) {
        assembledPages.push(startPages[startPages.length - 1] + 1);
      }
    }
  }

  middlePages.forEach((p) => {
    if (p > startPages[startPages.length - 1] && p < endPages[0]) {
      assembledPages.push(p);
    }
  });

  const lastAssembledNumeric = [...assembledPages].filter((p) => typeof p === "number").pop() as
    | number
    | undefined;

  if (
    endPages.length > 0 &&
    lastAssembledNumeric !== undefined &&
    endPages[0] > lastAssembledNumeric + 1
  ) {
    if (endPages[0] > lastAssembledNumeric + 2 && lastAssembledNumeric + 1 > boundaries) {
      assembledPages.push("...");
    } else if (endPages[0] === lastAssembledNumeric + 2) {
      if (lastAssembledNumeric + 1 > boundaries) {
        assembledPages.push(lastAssembledNumeric + 1);
      }
    }
  }

  assembledPages = [...assembledPages, ...endPages];

  const uniquePages = [];
  const seen = new Set();
  for (const page of assembledPages) {
    if (page === "...") {
      if (uniquePages.length === 0 || uniquePages[uniquePages.length - 1] !== "...") {
        uniquePages.push(page);
      }
    } else if (!seen.has(page)) {
      uniquePages.push(page);
      seen.add(page);
    }
  }

  return uniquePages.filter(
    (page, index, self) => page !== "..." || (index > 0 && self[index - 1] !== "..."),
  );
}

const pagnationVariants = tv(
  {
    slots: {
      base: " text-blue-600  exposure-sm-italic",
      item: "flex items-center justify-center w-8 h-8 rounded-sm hover:bg-blue-50 transition-all",
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
