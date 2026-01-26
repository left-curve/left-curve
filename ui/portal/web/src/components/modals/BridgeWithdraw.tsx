import { forwardRef } from "react";

import { Button, IconButton, IconClose, TruncateResponsive, useApp } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { AnyCoin } from "@left-curve/store/types";
import { usePrices, type useBridgeState, type UseSubmitTxReturnType } from "@left-curve/store";

interface BridgeWithdrawProps {
  coin: AnyCoin;
  config: NonNullable<ReturnType<typeof useBridgeState>["config"]>;
  amount: string;
  recipient: string;
  withdraw: UseSubmitTxReturnType<void, Error, void, unknown>;
  fee: string;
}

export const BridgeWithdraw = forwardRef((props: BridgeWithdrawProps, _ref) => {
  const { coin, config, amount, recipient, withdraw, fee } = props;
  const { hideModal, settings } = useApp();
  const { getPrice } = usePrices();

  const { formatNumberOptions } = settings;

  const feePrice = getPrice(fee, coin.denom, {
    format: true,
    formatOptions: formatNumberOptions,
  });

  const amountPrice = getPrice(amount, coin.denom, {
    format: true,
    formatOptions: formatNumberOptions,
  });

  return (
    <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray text-ink-secondary-700 pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-6 w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-4 top-4"
        variant="link"
        onClick={() => hideModal()}
      >
        <IconClose />
      </IconButton>
      <div className="flex items-center justify-items-center text-center w-full">
        <h2 className="text-ink-primary-900 diatype-lg-medium w-full">
          {m["bridge.withdraw.title"]()}
        </h2>
      </div>
      <div className="flex flex-col gap-1">
        <p className="exposure-sm-italic text-ink-disabled-300">{m["bridge.withdraw.action"]()}</p>
        <div className="flex items-center justify-between">
          <p className="h3-bold">
            {amount} {coin.symbol}
          </p>
          <img src={coin.logoURI} alt={coin.name} className="h-8 w-8" />
        </div>
        <p className="diatype-sm-regular text-ink-tertiary-500">{amountPrice}</p>
      </div>

      <div className="flex flex-col gap-1">
        <div className="flex items-center justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["bridge.withdraw.toWallet"]()}
          </p>
          <TruncateResponsive
            lastNumbers={4}
            text={recipient}
            className="diatype-sm-medium text-ink-secondary-700 max-w-[5.5rem]"
          />
        </div>

        <div className="flex items-center justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">{m["common.chain"]()}</p>
          <p className="diatype-sm-medium text-ink-secondary-700">
            {m["bridge.network"]({ network: config.chain.id })}
          </p>
        </div>

        <div className="flex items-center justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">{m["common.fee"]()}</p>
          <p className="diatype-sm-medium text-ink-secondary-700">{feePrice}</p>
        </div>

        <div className="flex items-center justify-between">
          <p className="diatype-sm-regular text-ink-tertiary-500">
            {m["bridge.withdraw.estArrival"]()}
          </p>
          <p className="diatype-sm-medium text-ink-secondary-700">
            {m["bridge.timeArrival"]({ network: config.chain.id })}
          </p>
        </div>
      </div>

      <Button
        onClick={() => {
          withdraw.mutate();
          hideModal();
        }}
        isLoading={withdraw.isPending}
        fullWidth
      >
        {m["common.confirm"]()}
      </Button>
    </div>
  );
});
