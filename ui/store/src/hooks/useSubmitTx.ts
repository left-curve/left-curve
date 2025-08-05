import { useMutation } from "@tanstack/react-query";
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
    success?: string | ((error: TData) => string);
    error?: string | ((data: TError) => string);
    abort?: string;
  };
  mutation: Omit<UseMutationOptions<TData, TError, TVariables, TContext>, "mutationFn"> & {
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
  const { refetch: refreshBalances } = useBalances({ address: account?.address });

  const { mutationFn } = mutation;

  return useMutation<TData, TError, TVariables, TContext>(
    {
      ...mutation,
      onSuccess: (...params) => {
        refreshBalances();
        mutation.onSuccess?.(...params);
      },
      mutationFn: async (variables: TVariables) => {
        const controller = new AbortController();
        subscriptions.emit("submitTx", { isSubmitting: true });

        try {
          const data = await mutationFn(variables, {
            signal: controller.signal,
            abort: () => {
              throw controller.abort();
            },
          });

          const message = (() => {
            if (typeof submission.success === "function") return submission.success(data);
            return submission.success || "Transaction submitted successfully.";
          })();

          subscriptions.emit("submitTx", { isSubmitting: false, isSuccess: true, data, message });
          toast.success?.(data);

          return data;
        } catch (error) {
          if (error) {
            console.log(error);
            toast.error?.(error as TError);
          }

          const abortError = new Error(submission?.abort || "Transaction submission aborted.");

          const message = (() => {
            if (!error) return abortError.message;
            if (typeof submission.error === "function") return submission.error(error as TError);
            return submission.error || "An error occurred while submitting the transaction.";
          })();

          subscriptions.emit("submitTx", {
            isSubmitting: false,
            isSuccess: false,
            message,
          });

          throw error || abortError;
        }
      },
    },
    queryClient,
  );
}
