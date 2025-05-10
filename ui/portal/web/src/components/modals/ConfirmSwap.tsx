import { forwardRef, useImperativeHandle } from "react";
import { useApp } from "~/hooks/useApp";

import { Button, IconArrowDown, IconButton, IconClose } from "@left-curve/applets-kit";

import { formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices } from "@left-curve/store";

import { m } from "~/paraglide/messages";

import type { Coin } from "@left-curve/dango/types";
import type { ModalRef } from "./RootModal";

type ConfirmSwapProps = {
  input: {
    coin: Coin;
    amount: string;
  };
  output: {
    coin: Coin;
    amount: string;
  };
  fee: string;
  confirmSwap: () => void;
  rejectSwap: () => void;
};

export const ConfirmSwap = forwardRef<ModalRef, ConfirmSwapProps>(
  ({ input, output, fee, confirmSwap, rejectSwap }, ref) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;

    const { coins } = useConfig();
    const { getPrice } = usePrices();

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => rejectSwap(),
    }));

    const inputCoin = coins[input.coin.denom];
    const outputCoin = coins[output.coin.denom];

    return (
      <div className="flex flex-col bg-white-100 md:border border-gray-100 rounded-xl relative gap-4 w-full md:max-w-[25rem] p-6 pt-4">
        <IconButton
          className="hidden md:block absolute right-5 top-5"
          variant="link"
          onClick={() => [rejectSwap(), hideModal()]}
        >
          <IconClose />
        </IconButton>

        <div className="md:flex flex-col gap-4 md:pt-3 items-center hidden">
          <p className="text-gray-900 diatype-lg-medium">{m["dex.swap"]()}</p>
        </div>
        <div className="flex flex-col gap-3 items-center">
          <div className="flex flex-col gap-1 w-full">
            <p className="text-gray-300 exposure-sm-italic">{m["dex.swapping"]()}</p>
            <div className="flex w-full items-center justify-between">
              <p className="text-gray-700 h3-bold">
                {formatUnits(input.amount, inputCoin.decimals)} {inputCoin.symbol}
              </p>
              <img className="h-8 w-8" src={inputCoin.logoURI} alt={inputCoin.symbol} />
            </div>
            <p className="text-gray-500 diatype-sm-regular">
              {getPrice(input.amount, inputCoin.denom, { format: true, ...formatNumberOptions })}
            </p>
          </div>
          <div className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5">
            <IconArrowDown className="h-3 w-3 text-gray-300" />
          </div>
          <div className="flex flex-col gap-1 w-full">
            <div className="flex w-full items-center justify-between">
              <p className="text-gray-700 h3-bold">
                {formatUnits(output.amount, outputCoin.decimals)} {outputCoin.symbol}
              </p>
              <img className="h-8 w-8" src={outputCoin.logoURI} alt={outputCoin.symbol} />
            </div>
            <p className="text-gray-500 diatype-sm-regular">
              {getPrice(output.amount, outputCoin.denom, { format: true, ...formatNumberOptions })}
            </p>
          </div>
          <div className="flex w-full items-center justify-between pt-3">
            <p className="text-gray-500 diatype-sm-regular">{m["dex.fee"]()}</p>
            <p className=" diatype-sm-medium text-gray-700">{fee}</p>
          </div>
        </div>

        <Button fullWidth onClick={() => [confirmSwap(), hideModal()]}>
          {m["common.confirm"]()}
        </Button>
      </div>
    );
  },
);
