import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useInfiniteGraphqlQuery } from "../../../store/src/hooks/useInfiniteGraphqlQuery";
import { useQueryWithPagination } from "../../../store/src/hooks/useQueryWithPagination";
import { createQueryClientWrapper } from "./utils/query-client";

import type { GraphqlPagination, GraphqlQueryResult, PageInfo } from "@left-curve/types";
import type { QueryFunctionContext } from "@tanstack/react-query";

type Row = {
  id: string;
};

function createPage(nodes: Row[], pageInfo: PageInfo): GraphqlQueryResult<Row> {
  return {
    edge: nodes.map((node) => ({
      cursor: `${node.id}-cursor`,
      node,
    })),
    nodes,
    pageInfo,
  };
}

function getPageParam(mock: ReturnType<typeof vi.fn>, callIndex: number) {
  return (mock.mock.calls[callIndex]?.[0] as QueryFunctionContext<string[], GraphqlPagination>)
    .pageParam;
}

describe("GraphQL pagination hooks", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("passes backend cursor pagination through finite query navigation", async () => {
    const queryFn = vi.fn(
      async ({ pageParam }: QueryFunctionContext<string[], GraphqlPagination>) => {
        if (pageParam.after) {
          return createPage([{ id: "second-page" }], {
            endCursor: "second-end",
            hasNextPage: false,
            hasPreviousPage: true,
            startCursor: "second-start",
          });
        }

        if (pageParam.before) {
          return createPage([{ id: "previous-page" }], {
            endCursor: "previous-end",
            hasNextPage: true,
            hasPreviousPage: false,
            startCursor: "previous-start",
          });
        }

        return createPage([{ id: "first-page" }], {
          endCursor: "first-end",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "first-start",
        });
      },
    );

    const { result } = renderHook(
      () =>
        useQueryWithPagination<Row>({
          queryFn,
          queryKey: ["finitePagination"],
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("first-page"));

    expect(getPageParam(queryFn, 0)).toEqual({
      after: undefined,
      before: undefined,
      first: 10,
      last: undefined,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.hasNextPage).toBe(true);
    expect(result.current.pagination.hasPreviousPage).toBe(false);

    act(() => {
      result.current.pagination.goNext();
    });

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("second-page"));

    expect(getPageParam(queryFn, 1)).toEqual({
      after: "first-end",
      before: undefined,
      first: 10,
      last: undefined,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.hasNextPage).toBe(false);
    expect(result.current.pagination.hasPreviousPage).toBe(true);

    act(() => {
      result.current.pagination.goPrev();
    });

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("previous-page"));

    expect(getPageParam(queryFn, 2)).toEqual({
      after: undefined,
      before: "second-start",
      first: undefined,
      last: 10,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
  });

  it("does not run finite pagination queries while disabled", () => {
    const queryFn = vi.fn(async () =>
      createPage([{ id: "disabled" }], {
        hasNextPage: false,
        hasPreviousPage: false,
      }),
    );

    const { result } = renderHook(
      () =>
        useQueryWithPagination<Row>({
          enabled: false,
          queryFn,
          queryKey: ["disabledPagination"],
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(queryFn).not.toHaveBeenCalled();
    expect(result.current.pagination.hasNextPage).toBe(false);
    expect(result.current.pagination.hasPreviousPage).toBe(false);
  });

  it("passes custom finite pagination limits and sort order to the initial backend query", async () => {
    const queryFn = vi.fn(async () =>
      createPage([{ id: "custom-page" }], {
        endCursor: "custom-end",
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: "custom-start",
      }),
    );

    const { result } = renderHook(
      () =>
        useQueryWithPagination<Row>({
          limit: 25,
          queryFn,
          queryKey: ["customFinitePagination"],
          sortBy: "TIMESTAMP_ASC",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("custom-page"));

    expect(getPageParam(queryFn, 0)).toEqual({
      after: undefined,
      before: undefined,
      first: 25,
      last: undefined,
      sortBy: "TIMESTAMP_ASC",
    });
  });

  it("keeps finite page data visible while the next backend cursor request is pending", async () => {
    let resolveSecondPage!: (page: GraphqlQueryResult<Row>) => void;
    const secondPage = new Promise<GraphqlQueryResult<Row>>((resolve) => {
      resolveSecondPage = resolve;
    });

    const queryFn = vi.fn(
      async ({ pageParam }: QueryFunctionContext<string[], GraphqlPagination>) => {
        if (pageParam.after) return secondPage;

        return createPage([{ id: "first-page" }], {
          endCursor: "first-end",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "first-start",
        });
      },
    );

    const { result } = renderHook(
      () =>
        useQueryWithPagination<Row>({
          queryFn,
          queryKey: ["pendingFinitePagination"],
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("first-page"));

    act(() => {
      result.current.pagination.goNext();
    });

    await waitFor(() => expect(queryFn).toHaveBeenCalledTimes(2));

    expect(getPageParam(queryFn, 1)).toEqual({
      after: "first-end",
      before: undefined,
      first: 10,
      last: undefined,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.data?.nodes[0]?.id).toBe("first-page");

    await act(async () => {
      resolveSecondPage(
        createPage([{ id: "second-page" }], {
          endCursor: "second-end",
          hasNextPage: false,
          hasPreviousPage: true,
          startCursor: "second-start",
        }),
      );
    });

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("second-page"));
  });

  it("does not issue finite pagination requests when backend page info has no adjacent pages", async () => {
    const queryFn = vi.fn(async () =>
      createPage([{ id: "only-page" }], {
        endCursor: "only-end",
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: "only-start",
      }),
    );

    const { result } = renderHook(
      () =>
        useQueryWithPagination<Row>({
          queryFn,
          queryKey: ["singleFinitePaginationPage"],
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.nodes[0]?.id).toBe("only-page"));

    act(() => {
      result.current.pagination.goNext();
      result.current.pagination.goPrev();
    });

    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(getPageParam(queryFn, 0)).toEqual({
      after: undefined,
      before: undefined,
      first: 10,
      last: undefined,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.hasNextPage).toBe(false);
    expect(result.current.pagination.hasPreviousPage).toBe(false);
  });

  it("does not run infinite pagination queries while disabled", () => {
    const queryFn = vi.fn(async () =>
      createPage([{ id: "disabled-infinite" }], {
        hasNextPage: true,
        hasPreviousPage: true,
      }),
    );

    const { result } = renderHook(
      () =>
        useInfiniteGraphqlQuery<Row>({
          initialPage: 3,
          limit: 50,
          query: {
            enabled: false,
            queryFn,
            queryKey: ["disabledInfinitePagination"],
          },
          sortBy: "BLOCK_HEIGHT_ASC",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    expect(queryFn).not.toHaveBeenCalled();
    expect(result.current.pagination).toMatchObject({
      currentPage: 3,
      hasNextPage: false,
      hasPreviousPage: false,
    });

    act(() => {
      result.current.pagination.goNext();
      result.current.pagination.goPrev();
    });

    expect(queryFn).not.toHaveBeenCalled();
    expect(result.current.pagination.currentPage).toBe(3);
  });

  it("does not issue infinite pagination requests when backend page info has no adjacent pages", async () => {
    const queryFn = vi.fn(async () =>
      createPage([{ id: "single-infinite-page" }], {
        endCursor: "single-end",
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: "single-start",
      }),
    );

    const { result } = renderHook(
      () =>
        useInfiniteGraphqlQuery<Row>({
          initialPage: 6,
          limit: 4,
          query: {
            queryFn,
            queryKey: ["singleInfinitePaginationPage"],
          },
          sortBy: "BLOCK_HEIGHT_ASC",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() =>
      expect(result.current.data?.pages[0]?.nodes[0]?.id).toBe("single-infinite-page"),
    );

    act(() => {
      result.current.pagination.goNext();
      result.current.pagination.goPrev();
    });

    expect(queryFn).toHaveBeenCalledTimes(1);
    expect(getPageParam(queryFn, 0)).toEqual({
      first: 4,
      sortBy: "BLOCK_HEIGHT_ASC",
    });
    expect(result.current.pagination.currentPage).toBe(6);
    expect(result.current.pagination.hasNextPage).toBe(false);
    expect(result.current.pagination.hasPreviousPage).toBe(false);
  });

  it("derives infinite-query next page parameters from backend page info", async () => {
    const queryFn = vi.fn(
      async ({ pageParam }: QueryFunctionContext<unknown[], GraphqlPagination>) => {
        if (pageParam.after) {
          return createPage([{ id: "next-page" }], {
            endCursor: "next-end",
            hasNextPage: false,
            hasPreviousPage: true,
            startCursor: "next-start",
          });
        }

        return createPage([{ id: "initial-page" }], {
          endCursor: "initial-end",
          hasNextPage: true,
          hasPreviousPage: false,
          startCursor: "initial-start",
        });
      },
    );

    const { result } = renderHook(
      () =>
        useInfiniteGraphqlQuery<Row>({
          limit: 3,
          query: {
            queryFn,
            queryKey: ["infinitePagination"],
          },
          sortBy: "BLOCK_HEIGHT_DESC",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.pages[0]?.nodes[0]?.id).toBe("initial-page"));

    expect(getPageParam(queryFn, 0)).toEqual({
      first: 3,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.currentPage).toBe(1);

    await act(async () => {
      result.current.pagination.goNext();
    });

    await waitFor(() => expect(queryFn).toHaveBeenCalledTimes(2));

    expect(getPageParam(queryFn, 1)).toEqual({
      after: "initial-end",
      first: 3,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.currentPage).toBe(2);

    act(() => {
      result.current.pagination.goNext();
    });

    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("derives infinite-query previous page parameters from backend page info", async () => {
    const queryFn = vi.fn(
      async ({ pageParam }: QueryFunctionContext<unknown[], GraphqlPagination>) => {
        if (pageParam.before) {
          return createPage([{ id: "previous-page" }], {
            endCursor: "previous-end",
            hasNextPage: true,
            hasPreviousPage: false,
            startCursor: "previous-start",
          });
        }

        return createPage([{ id: "current-page" }], {
          endCursor: "current-end",
          hasNextPage: false,
          hasPreviousPage: true,
          startCursor: "current-start",
        });
      },
    );

    const { result } = renderHook(
      () =>
        useInfiniteGraphqlQuery<Row>({
          initialPage: 4,
          limit: 2,
          query: {
            queryFn,
            queryKey: ["infinitePreviousPagination"],
          },
          sortBy: "BLOCK_HEIGHT_DESC",
        }),
      {
        wrapper: createQueryClientWrapper(),
      },
    );

    await waitFor(() => expect(result.current.data?.pages[0]?.nodes[0]?.id).toBe("current-page"));
    expect(result.current.pagination.currentPage).toBe(4);

    await act(async () => {
      result.current.pagination.goPrev();
    });

    await waitFor(() => expect(queryFn).toHaveBeenCalledTimes(2));

    expect(getPageParam(queryFn, 1)).toEqual({
      before: "current-start",
      last: 2,
      sortBy: "BLOCK_HEIGHT_DESC",
    });
    expect(result.current.pagination.currentPage).toBe(3);
  });
});
