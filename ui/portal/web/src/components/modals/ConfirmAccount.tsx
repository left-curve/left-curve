import { forwardRef } from "react";

import { Badge, Button, IconButton, IconChecked, IconClose, useApp } from "@left-curve/applets-kit";

import { capitalize, formatUnits, wait } from "@left-curve/dango/utils";
import { useAccount, useConfig, usePrices } from "@left-curve/store";
import { m } from "@left-curve/foundation/paraglide/messages.js";

import type { Address } from "@left-curve/dango/types";
import type { useNavigate } from "@tanstack/react-router";

type ConfirmAccountProps = {
  amount: string;
  accountType: string;
  accountAddress: Address;
  accountName: string;
  navigate: ReturnType<typeof useNavigate>;
  denom: string;
};

export const ConfirmAccount = forwardRef<undefined, ConfirmAccountProps>(
  ({ amount, accountName, accountType, accountAddress, denom, navigate }, _ref) => {
    const { hideModal, settings } = useApp();
    const { refreshAccounts, changeAccount } = useAccount();
    const { formatNumberOptions } = settings;

    const { coins } = useConfig();
    const coin = coins.byDenom[denom];

    const { getPrice } = usePrices();

    const humanAmount = formatUnits(amount, coin.decimals);

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-overlay-secondary-gray rounded-xl relative gap-4 w-full md:max-w-[25rem]">
        <IconButton
          className="hidden md:block absolute right-5 top-5"
          variant="link"
          onClick={hideModal}
        >
          <IconClose />
        </IconButton>

        <div className="flex flex-col gap-4 p-6 pb-0 pt-0 md:pt-6">
          <div className="h-12 w-12 bg-surface-quaternary-green rounded-full flex items-center justify-center">
            <IconChecked className="h-6 w-6 text-primitives-green-light-100" />
          </div>
          <p className="text-ink-primary-900 h4-bold">{m["modals.accountCreation.title"]()}</p>
        </div>

        <span className="h-[1px] w-full bg-overlay-secondary-gray" />

        <div className=" flex flex-col gap-4 p-4 py-0 md:p-6 md:py-0">
          <div className="flex flex-col gap-2 w-full">
            <p className="exposure-sm-italic text-primitives-gray-light-300">
              {m["modals.accountCreation.accountName"]()}
            </p>
            <div className="flex gap-1 items-center ">
              <p className=" text-ink-secondary-700 h3-bold">{accountName}</p>
              <Badge text={capitalize(accountType)} color="blue" />
            </div>
          </div>
          <div className="flex flex-col gap-1 w-full">
            <p className="exposure-sm-italic text-primitives-gray-light-300">
              {m["modals.accountCreation.accountBalance"]()}
            </p>
            <div className="flex items-center justify-between text-ink-secondary-700 h3-bold">
              <p>
                {humanAmount} {coin.symbol}
              </p>
              <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
            </div>
            <p className="text-ink-tertiary-500 diatype-sm-regular">
              {getPrice(humanAmount, denom, { format: true, formatOptions: formatNumberOptions })}
            </p>
          </div>
        </div>

        <div className="p-4 md:p-6 pt-0 md:pt-0">
          <Button
            fullWidth
            onClick={async () => {
              hideModal();
              await refreshAccounts?.();
              await wait(500);
              navigate({ to: "/" });
              changeAccount?.(accountAddress);
            }}
          >
            {m["modals.accountCreation.getStarted"]()}
          </Button>
        </div>
      </div>
    );
  },
);
