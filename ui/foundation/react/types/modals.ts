export const Modals = {
  AddKey: "add-key",
  RemoveKey: "remove-key",
  QRConnect: "qr-connect",
  ConfirmSend: "confirm-send",
  ConfirmAccount: "confirm-account",
  SignWithDesktop: "sign-with-desktop",
  ConfirmSwap: "confirm-swap",
  RenewSession: "renew-session",
  ProTradeCloseAll: "pro-trade-close-all",
  ProTradeCloseOrder: "pro-trade-close-order",
  ProTradeLimitClose: "pro-trade-limit-close",
  ProSwapMarketClose: "pro-swap-market-close",
  ProSwapEditTPSL: "pro-edit-tpsl",
  ProSwapEditedSL: "pro-edited-sl",
  PoolAddLiquidity: "pool-add-liquidity",
  PoolWithdrawLiquidity: "pool-withdraw-liquidity",
  ActivityTransfer: "activity-transfer",
  ActivityConvert: "activity-convert",
  ActivitySpotOrder: "activity-spot-order",
};

export type ModalRef = {
  triggerOnClose: () => void;
};

export type ModalDefinition = {
  component: React.LazyExoticComponent<React.ForwardRefExoticComponent<any>>;
  options?: {
    header?: string;
    disableClosing?: boolean;
  };
};
