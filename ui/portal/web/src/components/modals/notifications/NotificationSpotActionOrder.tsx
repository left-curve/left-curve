import {
  Badge,
  IconButton,
  IconClose,
  IconLink,
  PairAssets,
  TextCopy,
  twMerge,
  useApp,
} from "@left-curve/applets-kit";
import { forwardRef } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import type { AnyCoin } from "@left-curve/store/types";

type NotificationSpotActionOrderProps = {
  status: "created" | "canceled" | "fulfilled";
  action: "buy" | "sell";
  base: AnyCoin;
  quote: AnyCoin;
  order: {
    type: "limit" | "market";
    amount?: string;
    id: string;
    timeCreated: string;
    fee?: string;
    timeCanceled?: string;
    tokenReceived?: string;
    limitPrice?: string;
    averagePrice?: string;
    filledAmount?: string;
    timeUpdated?: string;
  };
};

export const NotificationSpotActionOrder = forwardRef<undefined, NotificationSpotActionOrderProps>(
  ({ status, action, base, quote, order }, _) => {
    const { type, amount, limitPrice } = order;
    const { hideModal } = useApp();

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
          <h2 className="text-lg font-semibold text-center text-primary-900 capitalize">
            {m["notifications.notification.modal.spotOrderAction"]({ status })}
          </h2>
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1 items-center">
              <div className="flex gap-2 items-center justify-center">
                <PairAssets assets={[base, quote]} />
                <p className="h4-bold text-secondary-700">
                  {base.symbol}-{quote.symbol}
                </p>
                <IconLink className="text-tertiary-500 h-4 w-4" />
              </div>
              <div className="flex gap-2 diatype-sm-medium">
                <p
                  className={twMerge(
                    "capitalize",
                    action === "buy" ? "text-status-success" : "text-status-fail",
                  )}
                >
                  {action === "buy"
                    ? m["notifications.notification.modal.spotOrderBuy"]({ orderType: type })
                    : m["notifications.notification.modal.spotOrderSell"]({ orderType: type })}
                </p>
                <Badge text="Spot" color="blue" size="s" />
              </div>
            </div>
            <span className="w-full h-[1px] bg-secondary-gray my-2" />
            <div className="flex flex-col gap-2 w-full">
              {amount && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.amount"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>
                      {amount} {base.symbol}
                    </p>
                  </div>
                </div>
              )}
              {order.filledAmount && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.filledAmount"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>0.000123 / 0.000123 BTC</p>
                  </div>
                </div>
              )}
              {order.tokenReceived && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.tokenReceived"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>0.000123 BTC</p>
                  </div>
                </div>
              )}
              {limitPrice && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.limitPrice"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>
                      {limitPrice} {quote.symbol}
                    </p>
                  </div>
                </div>
              )}
              {order.averagePrice && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.averagePrice"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>123.45 ETH</p>
                  </div>
                </div>
              )}
              {order.fee && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.fee"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>$1.2</p>
                  </div>
                </div>
              )}
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.modal.id"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>331364</p>
                  <TextCopy copyText="331364" className="h-4 w-4" />
                </div>
              </div>
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.modal.timeCreated"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>August 14, 2025 10:15 AM</p>
                </div>
              </div>
              {order.timeCanceled && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.timeCanceled"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>August 14, 2025 10:15 AM</p>
                  </div>
                </div>
              )}
              {order.timeUpdated && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.timeUpdated"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>August 14, 2025 10:15 AM</p>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    );
  },
);
