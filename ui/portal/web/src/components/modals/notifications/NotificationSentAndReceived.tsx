import {
  AddressVisualizer,
  IconButton,
  IconClose,
  IconLink,
  TruncateText,
  useApp,
} from "@left-curve/applets-kit";
import { useConfig, usePrices } from "@left-curve/store";
import { useRouter } from "@tanstack/react-router";

import { forwardRef } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { format } from "date-fns";
import { formatUnits } from "@left-curve/dango/utils";

import type { Address, Coins } from "@left-curve/dango/types";

type NotificationSentAndReceivedProps = {
  action?: "received" | "sent";
  from: Address;
  to: Address;
  time: string;
  txHash: string;
  coins: Coins;
  blockHeight: number;
};

export const NotificationSentAndReceived = forwardRef<undefined, NotificationSentAndReceivedProps>(
  ({ action = "received", from, to, time, txHash, coins, blockHeight }) => {
    const { hideModal, setSidebarVisibility, settings } = useApp();
    const { getCoinInfo } = useConfig();
    const { navigate: _navigate_ } = useRouter();
    const { getPrice } = usePrices();
    const { timeFormat, dateFormat } = settings;

    const navigate = (url: string) => {
      hideModal();
      setSidebarVisibility(false);
      _navigate_({ to: url });
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice rounded-xl relative w-full md:max-w-[25rem]">
        <IconButton
          className="hidden md:block absolute right-2 top-2"
          variant="link"
          onClick={hideModal}
        >
          <IconClose />
        </IconButton>
        <div className="p-4 flex flex-col gap-5">
          <h2 className="text-lg font-semibold text-center text-primary-900">
            {action === "received"
              ? m["notifications.notification.modal.received"]()
              : m["notifications.notification.modal.sent"]()}
          </h2>
          <div className="flex flex-col gap-4">
            {Object.entries(coins).map(([denom, amount]) => {
              const coin = getCoinInfo(denom);
              const humanAmount = formatUnits(amount, coin.decimals);
              return (
                <div className="flex flex-col gap-2 w-full" key={`transfer-coin-${denom}`}>
                  <div className="flex items-center justify-between  h3-bold text-secondary-700">
                    <p>
                      {humanAmount} {coin.symbol}
                    </p>
                    <img src={coin.logoURI} alt={`${coin.symbol} logo`} className="h-8 w-8" />
                  </div>
                  <p className="text-tertiary-500 diatype-sm-regular">
                    {getPrice(humanAmount, denom, { format: true })}
                  </p>
                </div>
              );
            })}
            <span className="w-full h-[1px] bg-secondary-gray my-2" />
            <div className="flex flex-col gap-2 w-full">
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500 capitalize">
                  {m["notifications.notification.transfer.direction.first"]({ direction: action })}
                </p>
                <div className="flex items-center gap-1">
                  <AddressVisualizer
                    classNames={{ container: "address-visualizer" }}
                    address={from}
                    withIcon
                    onClick={navigate}
                  />
                </div>
              </div>
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500 capitalize">
                  {m["notifications.notification.transfer.direction.second"]({ direction: action })}
                </p>
                <div className="flex items-center gap-1">
                  <AddressVisualizer
                    classNames={{ container: "address-visualizer" }}
                    address={to}
                    withIcon
                    onClick={navigate}
                  />
                </div>
              </div>
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.modal.time"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>{format(time, `${dateFormat} ${timeFormat}`)}</p>
                </div>
              </div>
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.link"]({
                    link: txHash ? "txHash" : "blockHeight",
                  })}
                </p>
                <div
                  className="flex items-center gap-1 cursor-pointer"
                  onClick={() => navigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`)}
                >
                  {txHash ? <TruncateText text={txHash} /> : <p>{blockHeight}</p>}
                  <IconLink className="h-4 w-4" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  },
);
