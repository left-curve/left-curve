export function isPerpsUserStateNotFoundError(error: unknown) {
  const message =
    error instanceof Error ? error.message : typeof error === "string" ? error : undefined;

  return (
    !!message &&
    message.includes("data not found") &&
    message.includes("dango_types::perps::UserState")
  );
}

export function handlePerpsUserStateError(
  caught: unknown,
  handlers: { onNotFound: () => void; onError: (error: unknown) => void },
) {
  if (isPerpsUserStateNotFoundError(caught)) {
    handlers.onNotFound();
    return;
  }

  handlers.onError(caught);
}
