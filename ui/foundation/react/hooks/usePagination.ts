import { useCallback, useMemo } from "react";
import { useControlledState } from "./useControlledState";

const ONE_ELLIPSIS_SLOT = 1;

export type UsePaginationParameters = {
  currentPage?: number;
  setCurrentPage?: (page: number) => void;
  initialPage?: number;
  totalPages: number;
  siblings?: number;
  boundaries?: number;
  isDisabled?: boolean;
  maxDisplay?: number;
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
    maxDisplay = 7,
  } = parameters;

  const [currentPage, setCurrentPage] = useControlledState(
    _currentPage_,
    _setCurrentPage_,
    initialPage,
  );

  const pages = useMemo(() => Array.from({ length: totalPages }, (_, i) => i + 1), [totalPages]);

  const hasPreviousPage = currentPage > 1;
  const hasNextPage = currentPage < totalPages;

  const BOUNDARY_PLUS_ELLIPSIS_SLOTS = boundaries + ONE_ELLIPSIS_SLOT;

  const _numPagesInEdgeCaseView = Math.max(
    ONE_ELLIPSIS_SLOT,
    maxDisplay - BOUNDARY_PLUS_ELLIPSIS_SLOTS,
  );

  const _isReachedToFirst = currentPage <= _numPagesInEdgeCaseView;
  const _isReachedToLast = currentPage + siblings + BOUNDARY_PLUS_ELLIPSIS_SLOTS >= totalPages;

  const middlePages = useMemo(() => {
    if (_isReachedToFirst && !_isReachedToLast) {
      return pages.slice(0, _numPagesInEdgeCaseView);
    }
    if (_isReachedToLast) {
      return pages.slice(Math.max(0, totalPages - _numPagesInEdgeCaseView));
    }

    return pages.slice(
      Math.max(0, currentPage - siblings - 1),
      Math.min(totalPages, currentPage + siblings),
    );
  }, [
    currentPage,
    pages,
    siblings,
    totalPages,
    _numPagesInEdgeCaseView,
    _isReachedToFirst,
    _isReachedToLast,
  ]);

  const _getAllPreviousPages = useMemo(() => {
    return pages.slice(0, middlePages[0] - 1);
  }, [pages, middlePages]);

  const _getAllNextPages = useMemo(() => {
    return pages.slice(middlePages[middlePages.length - 1], pages.length);
  }, [pages, middlePages]);

  const previousPages = useMemo(() => {
    if (_isReachedToFirst || _getAllPreviousPages.length === 0) return [];
    return pages.slice(0, boundaries).filter((p) => !middlePages.includes(p));
  }, [_isReachedToFirst, pages, boundaries, middlePages, _getAllPreviousPages]);

  const nextPages = useMemo(() => {
    if (_isReachedToLast || _getAllNextPages.length === 0) return [];

    return pages.slice(totalPages - boundaries, totalPages).filter((p) => !middlePages.includes(p));
  }, [_isReachedToLast, pages, boundaries, totalPages, middlePages, _getAllNextPages]);

  const isPreviousTruncable = useMemo(() => {
    if (previousPages.length === 0 || middlePages.length === 0) return false;
    return middlePages[0] > previousPages[previousPages.length - 1] + 1;
  }, [previousPages, middlePages]);

  const isNextTruncable = useMemo(() => {
    if (middlePages.length === 0 || nextPages.length === 0) return false;
    return middlePages[middlePages.length - 1] + 1 < nextPages[0];
  }, [nextPages, middlePages]);

  const nextPage = useCallback(() => {
    if (hasNextPage && !isDisabled) {
      setCurrentPage(currentPage + 1);
    }
  }, [currentPage, hasNextPage, isDisabled, setCurrentPage]);

  const previousPage = useCallback(() => {
    if (hasPreviousPage && !isDisabled) {
      setCurrentPage(currentPage - 1);
    }
  }, [currentPage, hasPreviousPage, isDisabled, setCurrentPage]);

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
