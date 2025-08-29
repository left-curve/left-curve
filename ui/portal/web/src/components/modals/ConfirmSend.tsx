import { Button, IconButton, IconClose, Skeleton, TruncateText } from "@left-curve/applets-kit";
import { useApp } from "~/hooks/useApp";

import type { Address } from "@left-curve/dango/types";
import { formatUnits } from "@left-curve/dango/utils";
import { useConfig, usePrices, usePublicClient } from "@left-curve/store";
import { useQuery } from "@tanstack/react-query";

import { forwardRef, useImperativeHandle } from "react";
import { m } from "~/paraglide/messages";

type ConfirmSendProps = {
  amount: string;
  denom: string;
  to: Address;
  confirmSend: () => void;
  rejectSend: () => void;
};

export const ConfirmSend = forwardRef(
  ({ amount, denom, to, confirmSend, rejectSend }: ConfirmSendProps, ref) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;
    const { coins } = useConfig();
    const client = usePublicClient();
    const coin = coins.byDenom[denom];

    const { data: username, isLoading } = useQuery({
      queryKey: ["username", to],
      queryFn: async () => {
        const response = await client.getAccountInfo({ address: to });
        if (!response) throw new Error("unexpected error: account not found");
        const { index, params } = response;
        const [type, config] = Object.entries(params)[0];
        return `${type === "multi" ? "Multisig" : config.owner} #${index}`;
      },
    });

    useImperativeHandle(ref, () => ({
      triggerOnClose: () => rejectSend(),
    }));

    const { getPrice } = usePrices();

    const humanAmount = formatUnits(amount, coin.decimals);

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[25rem]">
        <p className="text-primary-900 diatype-lg-medium w-full text-center">
          {m["modals.confirmSend.title"]()}
        </p>
        <div className=" flex flex-col gap-4">
          <div className="flex flex-col gap-2 w-full">
            <p className="exposure-sm-italic text-gray-300">{m["modals.confirmSend.sending"]()}</p>
            <div className="flex items-center justify-between text-secondary-700 h3-bold">
              <p>
                {humanAmount} {coin.symbol}
              </p>
              <img src={coin.logoURI} alt={coin.denom} className="w-8 h-8" />
            </div>
            <p className="text-tertiary-500 diatype-sm-regular">
              {getPrice(humanAmount, denom, { format: true, formatOptions: formatNumberOptions })}
            </p>
          </div>
          <div className="flex flex-col gap-2 w-full">
            <p className="exposure-sm-italic text-gray-300">{m["common.to"]()}</p>
            {isLoading ? (
              <Skeleton className="h-[34px] w-full max-w-36" />
            ) : (
              <p className=" text-secondary-700 h3-bold">{username}</p>
            )}
            <TruncateText className="text-tertiary-500 diatype-sm-regular " text={to} />
          </div>
          {/*  <div className="flex items-center justify-between ">
          <p className="text-tertiary-500 diatype-sm-regular">Fee</p>
          <p className="text-secondary-700 diatype-sm-medium">$1.2</p>
        </div> */}
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => [rejectSend(), hideModal()]}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth onClick={() => [confirmSend(), hideModal()]}>
          {m["modals.confirmSend.confirmButton"]()}
        </Button>
      </div>
    );
  },
);
