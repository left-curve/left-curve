import { useCallback } from "react";

type ToastFn = (opts: { title: string; description: string }) => void;

type UseErrorHandlerOptions = {
  toast: ToastFn;
  title?: string;
  fallbackMessage?: string;
};

function extractErrorMessage(err: unknown): string | undefined {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err && typeof err.message === "string") {
    return err.message;
  }
}

function parseContractError(raw: string): string | null {
  const match = raw.match(/msg:\s*(.*?)(?:,\s*"backtrace":|$)/);
  return match?.[1]?.trim() || null;
}

export function useErrorHandler(options: UseErrorHandlerOptions) {
  const { toast, title = "Error", fallbackMessage = "Request failed" } = options;

  return useCallback(
    (err: unknown) => {
      const raw = extractErrorMessage(err);
      const description = (raw && parseContractError(raw)) ?? raw ?? fallbackMessage;
      toast({ title, description });
    },
    [toast, title, fallbackMessage],
  );
}
