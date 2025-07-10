import { Badge, Button, IconButton, IconClose, PairAssets } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { forwardRef, useImperativeHandle } from "react";
import type { AnyCoin, WithAmount } from "@left-curve/store/types";
import { usePrices } from "@left-curve/store";

import { m } from "~/paraglide/messages";

type PoolAddLiquidityProps = {
  confirmAddLiquidity: () => void;
  rejectAddLiquidity?: () => void;
  coins: {
    base: WithAmount<AnyCoin>;
    quote: WithAmount<AnyCoin>;
  };
};

export const PoolAddLiquidity = forwardRef(
  ({ confirmAddLiquidity, rejectAddLiquidity, coins }: PoolAddLiquidityProps, ref) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;
    const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

    const { base, quote } = coins;

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => {
        if (rejectAddLiquidity) rejectAddLiquidity();
      },
    }));

    return (
      <div className="flex flex-col bg-bg-primary-rice md:border border-gray-100 pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-gray-900 diatype-lg-medium w-full text-center">
          {m["poolLiquidity.modal.addLiquidity"]()}
        </p>
        <div className=" flex flex-col gap-8">
          <div className="flex gap-2 items-center">
            <PairAssets assets={[base, quote]} />
            <p className="text-secondary-700 h4-bold">
              {base.symbol}/{quote.symbol}
            </p>
            <Badge color="green" size="s" text="Stable Strategy" />
          </div>
          <div className="flex flex-col gap-4">
            <p className="exposure-sm-italic text-gray-300 font-normal">
              {m["poolLiquidity.modal.depositing"]()}
            </p>
            <div className="flex flex-col">
              <div className="w-full flex items-center justify-between">
                <p className="text-secondary-700 h3-bold">
                  {base.amount} {base.symbol}
                </p>
                <img src={base.logoURI} alt={base.symbol} className="w-8 h-8" />
              </div>
              <p className="text-tertiary-500 diatype-sm-regular">
                {getPrice(base.amount, base.denom, { format: true })}
              </p>
            </div>
            <div className="flex flex-col">
              <div className="w-full flex items-center justify-between">
                <p className="text-secondary-700 h3-bold">
                  {quote.amount} {quote.symbol}
                </p>
                <img src={quote.logoURI} alt={quote.symbol} className="w-8 h-8" />
              </div>
              <p className="text-tertiary-500 diatype-sm-regular">
                {getPrice(quote.amount, quote.denom, { format: true })}
              </p>
            </div>
          </div>
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => {
            if (rejectAddLiquidity) rejectAddLiquidity();
            hideModal();
          }}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth onClick={() => [confirmAddLiquidity(), hideModal()]}>
          {m["common.confirm"]()}
        </Button>
      </div>
    );
  },
);
