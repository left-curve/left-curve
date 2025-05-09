import { forwardRef } from "react";
import { useApp } from "~/hooks/useApp";

import {
  Badge,
  Button,
  IconArrowDown,
  IconButton,
  IconChecked,
  IconClose,
} from "@left-curve/applets-kit";

import { capitalize, formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices } from "@left-curve/store";
import { m } from "~/paraglide/messages";

export const SwapSummary = forwardRef<undefined>(() => {
  const { hideModal, settings } = useApp();
  const { formatNumberOptions } = settings;

  const { coins } = useConfig();

  const { getPrice } = usePrices();

  return (
    <div className="flex flex-col bg-white-100 md:border border-gray-100 rounded-xl relative gap-4 w-full md:max-w-[25rem] p-6 pt-4">
      <IconButton
        className="hidden md:block absolute right-5 top-5"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>

      <div className="md:flex flex-col gap-4 md:pt-3 items-center hidden">
        <p className="text-gray-900 diatype-lg-medium">Swap</p>
      </div>
      <div className="flex flex-col gap-3 items-center">
        <div className="flex flex-col gap-1 w-full">
          <p className="text-gray-300 exposure-sm-italic">Swapping</p>
          <div className="flex w-full items-center justify-between">
            <p className="text-gray-700 h3-bold">20.00 USDC</p>
            <img
              className="h-8 w-8"
              src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
              alt="usdc"
            />
          </div>
          <p className="text-gray-500 diatype-sm-regular">$20.00</p>
        </div>
        <div className="flex items-center justify-center border border-gray-300 rounded-full h-5 w-5">
          <IconArrowDown className="h-3 w-3 text-gray-300" />
        </div>
        <div className="flex flex-col gap-1 w-full">
          <div className="flex w-full items-center justify-between">
            <p className="text-gray-700 h3-bold">20.00 USDC</p>
            <img
              className="h-8 w-8"
              src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
              alt="usdc"
            />
          </div>
          <p className="text-gray-500 diatype-sm-regular">$20.00</p>
        </div>
        <div className="flex w-full items-center justify-between pt-3">
          <p className="text-gray-500 diatype-sm-regular">Fee</p>
          <p className=" diatype-sm-medium text-gray-700">$1.2</p>
        </div>
      </div>

      <Button fullWidth onClick={hideModal}>
        Confirm
      </Button>
    </div>
  );
});
