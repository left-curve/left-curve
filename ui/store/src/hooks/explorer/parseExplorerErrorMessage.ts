export type ParsedExplorerError = {
  error?: unknown;
  backtrace?: string;
};

export function parseExplorerErrorMessage(errorMessage?: string): ParsedExplorerError {
  if (!errorMessage) return {};

  try {
    const parsed = JSON.parse(errorMessage) as { error?: unknown; backtrace?: string };
    return {
      error: parsed.error,
      backtrace: parsed.backtrace,
    };
  } catch {
    return {};
  }
}
