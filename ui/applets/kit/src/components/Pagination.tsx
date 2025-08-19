import { usePagination } from "../hooks/usePagination.js";

import { AnimatePresence, motion } from "framer-motion";
import { Button } from "./Button";
import { IconChevronLeft } from "./icons/IconChevronLeft";
import { IconChevronRight } from "./icons/IconChevronRight";

import { tv } from "tailwind-variants";
import { twMerge } from "../utils/twMerge.js";

import type React from "react";
import type { VariantProps } from "tailwind-variants";
import type { UsePaginationParameters } from "../hooks/usePagination";

type PaginationProps = VariantProps<typeof paginationVariants> &
  Omit<UsePaginationParameters, "setCurrentPage"> & {
    labelOf?: string;
    labelPage?: string;
    onPageChange?: (page: number) => void;
  };

export const Pagination: React.FC<PaginationProps> = ({
  totalPages,
  currentPage: _currentPage_,
  initialPage = 1,
  onPageChange,
  siblings = 1,
  boundaries = 1,
  isDisabled,
  variant,
  maxDisplay = 7,
  labelPage = "Page",
  labelOf = "of",
}) => {
  const styles = paginationVariants({ variant, isDisabled });

  const {
    hasNextPage,
    hasPreviousPage,
    previousPages,
    middlePages,
    nextPages,
    isNextTruncable,
    isPreviousTruncable,
    currentPage,
    setCurrentPage,
    nextPage,
    previousPage,
  } = usePagination({
    initialPage,
    currentPage: _currentPage_,
    setCurrentPage: onPageChange,
    totalPages,
    siblings,
    boundaries,
    maxDisplay,
  });

  return (
    <motion.div
      className={twMerge(styles.base(), "flex items-center justify-center gap-1 ")}
      layout
    >
      <button
        type="button"
        onClick={previousPage}
        disabled={!hasPreviousPage || isDisabled}
        className={twMerge(
          styles.item(),
          !hasPreviousPage && "opacity-60",
          "mr-3 text-secondary-blue",
        )}
      >
        <IconChevronLeft className="w-5 h-5" />
      </button>

      {variant === "text" ? (
        <AnimatePresence mode="wait">
          <p>
            {labelPage}{" "}
            <motion.span
              key={currentPage}
              initial={{ scale: 1.4, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              transition={{
                duration: 0.2,
              }}
              className="min-w-[1rem] inline-block text-center"
            >
              {currentPage}
            </motion.span>{" "}
            {labelOf} {totalPages}
          </p>
        </AnimatePresence>
      ) : (
        <>
          {previousPages.map((page) => (
            <PaginationItem
              key={page}
              page={page}
              setCurrentPage={setCurrentPage}
              isCurrent={false}
            />
          ))}
          <TruncateElement isVisible={isPreviousTruncable} />
          {middlePages.map((page) => (
            <PaginationItem
              key={page}
              page={page}
              setCurrentPage={setCurrentPage}
              isCurrent={page === currentPage}
            />
          ))}
          <TruncateElement isVisible={isNextTruncable} />
          {nextPages.map((page) => (
            <PaginationItem
              key={page}
              page={page}
              setCurrentPage={setCurrentPage}
              isCurrent={false}
            />
          ))}
        </>
      )}

      <button
        type="button"
        onClick={nextPage}
        disabled={!hasNextPage || isDisabled}
        className={twMerge(styles.item(), !hasNextPage && "opacity-60", "ml-3 text-secondary-blue")}
      >
        <IconChevronRight className="w-5 h-5" />
      </button>
    </motion.div>
  );
};

type CursorPaginationProps = {
  isLoading: boolean;
  goNext: () => void;
  goPrev: () => void;
  className?: string;
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  nextLabel?: string;
  previousLabel?: string;
};

export const CursorPagination: React.FC<CursorPaginationProps> = ({
  goNext,
  goPrev,
  hasNextPage,
  nextLabel = "Next",
  hasPreviousPage,
  previousLabel = "Previous",
  isLoading,
  className,
}) => {
  return (
    <div className={twMerge(className)}>
      {hasPreviousPage ? (
        <Button
          variant="link"
          onClick={goPrev}
          isDisabled={!hasPreviousPage || isLoading}
          aria-label={previousLabel}
        >
          <IconChevronLeft className="w-5 h-5" />
          <span>{previousLabel}</span>
        </Button>
      ) : null}
      {hasNextPage ? (
        <Button
          variant="link"
          onClick={goNext}
          isDisabled={!hasNextPage || isLoading}
          aria-label={nextLabel}
        >
          <span>{nextLabel}</span>
          <IconChevronRight className="w-5 h-5" />
        </Button>
      ) : null}
    </div>
  );
};

type PaginationItemProps = {
  page: number;
  isCurrent: boolean;
  setCurrentPage: (page: number) => void;
};

const PaginationItem: React.FC<PaginationItemProps> = ({ page, setCurrentPage, isCurrent }) => {
  const styles = paginationVariants();
  return (
    <motion.button
      layout
      type="button"
      onClick={() => setCurrentPage(page)}
      className={twMerge(styles.item(), "relative")}
      initial={{ opacity: 0, scale: 0.9 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.2 }}
    >
      <span className="relative z-10">{page}</span>
      {isCurrent && (
        <motion.span className="absolute left-0 top-0 w-full h-full rounded-sm bg-primary-blue" />
      )}
    </motion.button>
  );
};

type TruncateElementProps = {
  isVisible: boolean;
};

const TruncateElement: React.FC<TruncateElementProps> = ({ isVisible }) => {
  if (!isVisible) return null;

  return <span className="px-2 text-secondary-blue select-none">...</span>;
};

const paginationVariants = tv(
  {
    slots: {
      base: " text-secondary-blue  exposure-sm-italic",
      item: "flex items-center justify-center w-8 h-8 rounded-sm hover:bg-surface-secondary-blue disabled:hover:bg-transparent transition-all exposure-sm-italic",
    },
    variants: {
      variant: {
        default: "",
        text: "",
      },
      isDisabled: {
        true: "pointer-events-none cursor-not-allowed ",
      },
    },
    defaultVariants: {
      variant: "default",
      isDisabled: false,
    },
    compoundVariants: [
      {
        isDisabled: true,
        item: "hover:bg-transparent pointer-events-none cursor-not-allowed ",
      },
    ],
  },
  {
    twMerge: true,
  },
);
