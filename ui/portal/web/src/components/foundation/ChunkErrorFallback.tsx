import { Button } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type React from "react";

export const ChunkErrorFallback: React.FC<{ resetErrorBoundary: () => void }> = ({
  resetErrorBoundary,
}) => (
  <div className="flex flex-col items-center justify-center gap-3 p-6">
    <p className="text-ink-tertiary-500 text-sm text-center">{m["common.failedToLoad"]()}</p>
    <Button variant="secondary" size="sm" onClick={resetErrorBoundary}>
      {m["common.retry"]()}
    </Button>
  </div>
);
