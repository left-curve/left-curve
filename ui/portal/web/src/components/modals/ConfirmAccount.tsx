import { Badge, Button, IconButton, IconChecked, IconClose } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import { capitalize, formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices } from "@left-curve/store";
import { m } from "~/paraglide/messages";

type ConfirmAccountProps = {
  amount: string;
  accountType: string;
  accountName: string;
  denom: string;
};

export function ConfirmAccount({ amount, accountName, accountType, denom }: ConfirmAccountProps) {
  const { hideModal, formatNumberOptions } = useApp();

  const { coins, state } = useConfig();
  const coin = coins[state.chainId][denom];

  const { getPrice } = usePrices();

  return (
    <div className="flex flex-col bg-white-100 md:border border-gray-100 rounded-3xl relative gap-4 w-full md:max-w-[25rem]">
      <IconButton
        className="hidden md:block absolute right-5 top-5"
        variant="link"
        onClick={hideModal}
      >
        <IconClose />
      </IconButton>

      <div className="flex flex-col gap-4 p-6 pb-0 pt-0 md:pt-6">
        <div className="h-12 w-12 bg-green-bean-300 rounded-full flex items-center justify-center">
          <IconChecked className="h-6 w-6 text-green-bean-100" />
        </div>
        <p className="text-gray-900 h4-bold">{m["modals.accountCreation.title"]()}</p>
      </div>

      <span className="h-[1px] w-full bg-gray-100" />

      <div className=" flex flex-col gap-4 p-4 py-0 md:p-6 md:py-0">
        <div className="flex flex-col gap-2 w-full">
          <p className="exposure-sm-italic text-gray-300">
            {m["modals.accountCreation.accountName"]()}
          </p>
          <div className="flex gap-1 items-center ">
            <p className=" text-gray-700 h3-bold">{accountName}</p>
            <Badge text={capitalize(accountType)} color="blue" />
          </div>
        </div>
        <div className="flex flex-col gap-1 w-full">
          <p className="exposure-sm-italic text-gray-300">
            {m["modals.accountCreation.accountBalance"]()}
          </p>
          <div className="flex items-center justify-between text-gray-700 h3-bold">
            <p>
              {formatUnits(amount, coin.decimals)} {coin.symbol}
            </p>
            <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
          </div>
          <p className="text-gray-500 diatype-sm-regular">
            {getPrice(amount, denom, { format: true, formatOptions: formatNumberOptions })}
          </p>
        </div>
      </div>

      <div className="p-4 md:p-6 pt-0 md:pt-0">
        <Button fullWidth onClick={hideModal}>
          {m["modals.accountCreation.getStarted"]()}
        </Button>
      </div>
    </div>
  );
}
