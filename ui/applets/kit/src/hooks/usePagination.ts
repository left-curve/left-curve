import { useCallback, useMemo } from "react";
import { useControlledState } from "./useControlledState";

export type UsePaginationParameters = {
  currentPage?: number;
  setCurrentPage?: (page: number) => void;
  initialPage?: number;
  totalPages: number;
  siblings?: number;
  boundaries?: number;
  isDisabled?: boolean;
};

export function usePagination(parameters: UsePaginationParameters) {
  const {
    currentPage: _currentPage_,
    setCurrentPage: _setCurrentPage_,
    initialPage = 1,
    totalPages,
    siblings = 1,
    boundaries = 1,
    isDisabled,
  } = parameters;

  const [currentPage, setCurrentPage] = useControlledState(
    _currentPage_,
    _setCurrentPage_,
    initialPage,
  );

  const pages = Array.from({ length: totalPages }, (_, i) => i + 1);

  const hasPreviousPage = currentPage > 1;
  const hasNextPage = currentPage < totalPages;

  const _isReachedToFirst = currentPage <= siblings;
  const _isReachedToLast = currentPage + siblings >= totalPages;

  const middlePages = useMemo(() => {
    const middlePageCount = siblings * 2 + 1;
    if (_isReachedToFirst) {
      return pages.slice(0, middlePageCount);
    }
    if (_isReachedToLast) {
      return pages.slice(-middlePageCount);
    }
    return pages.slice(currentPage - siblings - 1, currentPage + siblings);
  }, [currentPage, pages]);

  const _getAllPreviousPages = useMemo(() => {
    return pages.slice(0, middlePages[0] - 1);
  }, [pages, middlePages]);

  const _getAllNextPages = useMemo(() => {
    return pages.slice(middlePages[middlePages.length - 1], pages[pages.length]);
  }, [pages, middlePages]);

  const previousPages = useMemo(() => {
    if (_isReachedToFirst || _getAllPreviousPages.length < 1) {
      return [];
    }
    return pages.slice(0, boundaries).filter((p) => !middlePages.includes(p));
  }, [currentPage, pages]);

  const nextPages = useMemo(() => {
    if (_isReachedToLast) {
      return [];
    }
    if (_getAllNextPages.length < 1) {
      return [];
    }
    return pages
      .slice(pages.length - boundaries, pages.length)
      .filter((p) => !middlePages.includes(p));
  }, [middlePages, pages]);

  const isPreviousTruncable = useMemo(() => {
    return middlePages[0] > previousPages[previousPages.length - 1] + 1;
  }, [previousPages, middlePages]);

  const isNextTruncable = useMemo(() => {
    return middlePages[middlePages.length - 1] + 1 < nextPages[0];
  }, [nextPages, middlePages]);

  const nextPage = useCallback(() => {
    if (hasNextPage && !isDisabled) {
      setCurrentPage(currentPage + 1);
    }
  }, [currentPage, totalPages]);

  const previousPage = useCallback(() => {
    if (hasPreviousPage && !isDisabled) {
      setCurrentPage(currentPage - 1);
    }
  }, [currentPage, totalPages]);

  return {
    hasNextPage,
    hasPreviousPage,
    previousPages,
    nextPages,
    middlePages,
    isPreviousTruncable,
    isNextTruncable,
    currentPage,
    nextPage,
    previousPage,
    setCurrentPage,
  };
}
