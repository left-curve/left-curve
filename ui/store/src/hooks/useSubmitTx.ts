import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useConfig } from "./useConfig.js";
import { useBalances } from "./useBalances.js";
import { useAccount } from "./useAccount.js";

import type {
  DefaultError,
  QueryClient,
  UseMutationOptions,
  UseMutationResult,
} from "@tanstack/react-query";

export type UseSubmitTxParameters<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TContext = unknown,
> = {
  toast?: {
    success?: (data: TData) => void;
    error?: (error: TError) => void;
  };
  submission?: {
    success?: string | ((data: TData) => string);
  };
  mutation: Omit<UseMutationOptions<TData, TError, TVariables, TContext>, "mutationFn"> & {
    invalidateKeys?: unknown[][];
    mutationFn: (
      variables: TVariables,
      options: { signal: AbortSignal; abort: () => void },
    ) => Promise<TData>;
  };
};

export type UseSubmitTxReturnType<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TContext = unknown,
> = UseMutationResult<TData, TError, TVariables, TContext>;

function extractErrorMessage(err: unknown): string | undefined {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err && typeof err.message === "string") {
    return err.message;
  }
}

function parseContractError(raw: string): string | null {
  try {
    const parsed = JSON.parse(raw) as { error?: string; backtrace?: string };
    if (typeof parsed.error === "string") {
      const match = parsed.error.match(/msg:\s*(.*?)$/);
      return match?.[1]?.trim() || parsed.error;
    }
  } catch {
    // Not JSON — try regex directly on the raw string
    const match = raw.match(/msg:\s*(.*?)(?:,\s*"backtrace":|$)/);
    if (match?.[1]?.trim()) return match[1].trim();
  }
  return null;
}

function parseTxError(err: unknown): { title: string; description: string } {
  const raw = extractErrorMessage(err);
  const description = (raw && parseContractError(raw)) ?? raw ?? "An unexpected error occurred.";
  return { title: "Error", description };
}

export function useSubmitTx<
  TData = unknown,
  TError = DefaultError,
  TVariables = void,
  TContext = unknown,
>(
  parameters: UseSubmitTxParameters<TData, TError, TVariables, TContext>,
  queryClient?: QueryClient,
): UseSubmitTxReturnType<TData, TError, TVariables, TContext> {
  const { subscriptions } = useConfig();
  const { mutation, submission = {}, toast = {} } = parameters;
  const { account } = useAccount();
  // biome-ignore lint/correctness/useHookAtTopLevel: it needs to be at the top level for React Query to work correctly.
  const qClient = queryClient ?? useQueryClient();
  const { refetch: refreshBalances } = useBalances({ address: account?.address });

  const { mutationFn, invalidateKeys, meta = {} } = mutation;

  return useMutation<TData, TError, TVariables, TContext>(
    {
      ...mutation,
      meta: {
        invalidateKeys,
        ...meta,
      },
      onSuccess: (...params) => {
        refreshBalances();
        mutation.onSuccess?.(...params);
        for (const key of invalidateKeys || []) {
          qClient.invalidateQueries({ queryKey: key });
        }
      },
      mutationFn: async (variables: TVariables) => {
        const controller = new AbortController();
        subscriptions.emit({ key: "submitTx" }, { status: "pending" });

        try {
          const data = await mutationFn(variables, {
            signal: controller.signal,
            abort: () => {
              throw controller.abort();
            },
          });

          const message = (() => {
            if (typeof submission.success === "function") return submission.success(data);
            return submission.success;
          })();

          subscriptions.emit({ key: "submitTx" }, { status: "success", data, message });
          toast.success?.(data);

          return data;
        } catch (error) {
          if (error) {
            console.log(error);

            if (toast.error) {
              toast.error(error as TError);
            }

            const parsed = parseTxError(error);
            subscriptions.emit({ key: "submitTx" }, { status: "error", ...parsed });
          } else {
            subscriptions.emit({
              key: "submitTx",
            }, {
              status: "error",
              title: "Error",
              description: "Transaction submission aborted.",
            });
          }

          throw error || new Error("Transaction submission aborted.");
        }
      },
    },
    queryClient,
  );
}
