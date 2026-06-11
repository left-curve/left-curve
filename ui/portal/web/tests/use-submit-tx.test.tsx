import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useSubmitTx } from "../../../store/src/hooks/useSubmitTx";
import { createQueryClientWrapper, createTestQueryClient } from "./utils/query-client";

import type { UseSubmitTxParameters } from "../../../store/src/hooks/useSubmitTx";

type TestVariables = {
  amount: string;
};

type SettledResult = { status: "resolved"; value: string } | { status: "rejected"; error: unknown };

const storeHookMocks = vi.hoisted(() => ({
  refreshBalances: vi.fn(),
  subscriptionsEmit: vi.fn(),
  useAccount: vi.fn(),
  useBalances: vi.fn(),
  useConfig: vi.fn(),
}));

vi.mock("../../../store/src/hooks/useAccount.js", () => ({
  useAccount: storeHookMocks.useAccount,
}));

vi.mock("../../../store/src/hooks/useBalances.js", () => ({
  useBalances: storeHookMocks.useBalances,
}));

vi.mock("../../../store/src/hooks/useConfig.js", () => ({
  useConfig: storeHookMocks.useConfig,
}));

function renderSubmitTx(
  parameters: UseSubmitTxParameters<string, Error, TestVariables>,
  onSettled = vi.fn<(result: SettledResult) => void>(),
) {
  const queryClient = createTestQueryClient();

  function Consumer() {
    const submitTx = useSubmitTx(parameters);
    return (
      <button
        type="button"
        onClick={() => {
          submitTx
            .mutateAsync({ amount: "12" })
            .then((value) => onSettled({ status: "resolved", value }))
            .catch((error: unknown) => onSettled({ status: "rejected", error }));
        }}
      >
        Submit
      </button>
    );
  }

  render(<Consumer />, { wrapper: createQueryClientWrapper(queryClient) });

  return { onSettled, queryClient };
}

describe("useSubmitTx", () => {
  beforeEach(() => {
    storeHookMocks.useConfig.mockReturnValue({
      subscriptions: {
        emit: storeHookMocks.subscriptionsEmit,
      },
    });
    storeHookMocks.useAccount.mockReturnValue({
      account: {
        address: "0x73656e6465720000000000000000000000000000",
      },
    });
    storeHookMocks.useBalances.mockReturnValue({
      refetch: storeHookMocks.refreshBalances,
    });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.clearAllMocks();
  });

  it("emits pending and success transaction states, refreshes balances, and invalidates query keys", async () => {
    const mutationFn = vi.fn(async () => "receipt-1");
    const onSuccess = vi.fn();
    const toastSuccess = vi.fn();
    const { onSettled, queryClient } = renderSubmitTx({
      toast: {
        success: toastSuccess,
      },
      submission: {
        success: (receipt) => `Submitted ${receipt}`,
      },
      mutation: {
        invalidateKeys: [["positions"], ["quests", "alice"]],
        mutationFn,
        onSuccess,
      },
    });
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() =>
      expect(onSettled).toHaveBeenCalledWith({ status: "resolved", value: "receipt-1" }),
    );

    expect(mutationFn).toHaveBeenCalledWith(
      { amount: "12" },
      expect.objectContaining({
        signal: expect.any(AbortSignal),
        abort: expect.any(Function),
      }),
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "success",
        data: "receipt-1",
        message: "Submitted receipt-1",
      },
    );
    expect(toastSuccess).toHaveBeenCalledWith("receipt-1");
    expect(storeHookMocks.refreshBalances).toHaveBeenCalledOnce();
    expect(onSuccess).toHaveBeenCalledOnce();
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["positions"] });
    expect(invalidateQueries).toHaveBeenCalledWith({ queryKey: ["quests", "alice"] });
  });

  it("emits static transaction success messages without recomputing them from receipts", async () => {
    const mutationFn = vi.fn(async () => "receipt-static");
    const toastSuccess = vi.fn();
    const { onSettled } = renderSubmitTx({
      toast: {
        success: toastSuccess,
      },
      submission: {
        success: "Static success",
      },
      mutation: {
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() =>
      expect(onSettled).toHaveBeenCalledWith({ status: "resolved", value: "receipt-static" }),
    );

    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "success",
        data: "receipt-static",
        message: "Static success",
      },
    );
    expect(toastSuccess).toHaveBeenCalledWith("receipt-static");
    expect(storeHookMocks.refreshBalances).toHaveBeenCalledOnce();
  });

  it("preserves caller mutation meta while exposing invalidate keys for query tooling", async () => {
    const mutationFn = vi.fn(async () => "receipt-1");
    const invalidateKeys = [
      ["positions"],
      ["balances", "0x73656e6465720000000000000000000000000000"],
    ];
    const { onSettled, queryClient } = renderSubmitTx({
      mutation: {
        invalidateKeys,
        meta: {
          source: "trade",
        },
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() =>
      expect(onSettled).toHaveBeenCalledWith({ status: "resolved", value: "receipt-1" }),
    );

    expect(queryClient.getMutationCache().getAll()).toHaveLength(1);
    expect(queryClient.getMutationCache().getAll()[0].options.meta).toEqual({
      invalidateKeys,
      source: "trade",
    });
  });

  it("surfaces parsed contract errors through the transaction subscription", async () => {
    const contractError = new Error(
      'rpc failed: log: {"error":"execute wasm failed: msg: insufficient collateral","backtrace":"trace"}',
    );
    const mutationFn = vi.fn(async () => {
      throw contractError;
    });
    const toastError = vi.fn();
    const onError = vi.fn();
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    const { onSettled, queryClient } = renderSubmitTx({
      toast: {
        error: toastError,
      },
      mutation: {
        invalidateKeys: [["positions"]],
        mutationFn,
        onError,
      },
    });
    const invalidateQueries = vi.spyOn(queryClient, "invalidateQueries");

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toBe(contractError);
    expect(consoleLog).toHaveBeenCalledWith(contractError);
    expect(toastError).toHaveBeenCalledWith(contractError);
    expect(onError).toHaveBeenCalledWith(contractError, { amount: "12" }, undefined);
    expect(storeHookMocks.refreshBalances).not.toHaveBeenCalled();
    expect(invalidateQueries).not.toHaveBeenCalled();
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "insufficient collateral",
      },
    );
  });

  it("surfaces non-JSON backend contract messages through the transaction subscription", async () => {
    const contractError = new Error(
      'execute wasm failed: msg: order would cross the spread, "backtrace":"trace"',
    );
    const mutationFn = vi.fn(async () => {
      throw contractError;
    });
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    const { onSettled } = renderSubmitTx({
      mutation: {
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toBe(contractError);
    expect(consoleLog).toHaveBeenCalledWith(contractError);
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "order would cross the spread",
      },
    );
  });

  it("falls back to the backend error message when no contract message is available", async () => {
    const backendError = new Error("rpc request failed before execution");
    const mutationFn = vi.fn(async () => {
      throw backendError;
    });
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    const { onSettled } = renderSubmitTx({
      mutation: {
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toBe(backendError);
    expect(consoleLog).toHaveBeenCalledWith(backendError);
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "rpc request failed before execution",
      },
    );
  });

  it("surfaces raw string backend errors through the transaction subscription", async () => {
    const backendError = "rpc transport rejected request";
    const mutationFn = vi.fn(async () => {
      throw backendError;
    });
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    const { onSettled } = renderSubmitTx({
      mutation: {
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toBe(backendError);
    expect(consoleLog).toHaveBeenCalledWith(backendError);
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "rpc transport rejected request",
      },
    );
  });

  it("surfaces object-shaped backend errors through the transaction subscription", async () => {
    const backendError = {
      message: "gateway simulation failed",
    };
    const mutationFn = vi.fn(async () => {
      throw backendError;
    });
    const consoleLog = vi.spyOn(console, "log").mockImplementation(() => {});
    const { onSettled } = renderSubmitTx({
      mutation: {
        mutationFn,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toBe(backendError);
    expect(consoleLog).toHaveBeenCalledWith(backendError);
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "gateway simulation failed",
      },
    );
  });

  it("reports user-aborted submissions with a stable fallback error", async () => {
    let capturedSignal: AbortSignal | undefined;
    const onError = vi.fn();
    const mutationFn = vi.fn(async (_variables: TestVariables, { abort, signal }) => {
      capturedSignal = signal;
      abort();
      return "unreachable";
    });
    const { onSettled } = renderSubmitTx({
      mutation: {
        mutationFn,
        onError,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "Submit" }));

    await waitFor(() => expect(onSettled).toHaveBeenCalledOnce());
    const result = onSettled.mock.calls[0][0];

    expect(result.status).toBe("rejected");
    expect(result.status === "rejected" ? result.error : undefined).toEqual(
      new Error("Transaction submission aborted."),
    );
    expect(capturedSignal?.aborted).toBe(true);
    expect(onError).toHaveBeenCalledWith(
      new Error("Transaction submission aborted."),
      { amount: "12" },
      undefined,
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      1,
      { key: "submitTx" },
      { status: "pending" },
    );
    expect(storeHookMocks.subscriptionsEmit).toHaveBeenNthCalledWith(
      2,
      { key: "submitTx" },
      {
        status: "error",
        title: "Error",
        description: "Transaction submission aborted.",
      },
    );
  });
});
