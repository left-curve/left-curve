type TransferParams = {
  action?: string | string[];
};

const firstValue = (value?: string | string[]) => {
  if (Array.isArray(value)) return value[0];
  return value;
};

export type TransferAction = "send" | "receive";

export const normalizeTransferAction = (
  params: TransferParams,
  isConnected: boolean,
): { action: TransferAction; changed: boolean } => {
  const requested = firstValue(params.action);

  const parsed: TransferAction = requested === "receive" ? "receive" : "send";
  const forced = isConnected ? parsed : "send";

  return {
    action: forced,
    changed: requested !== forced,
  };
};
