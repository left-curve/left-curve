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
import { useRouter } from "@tanstack/react-router";

import { forwardRef } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { format } from "date-fns";

import type { AnyCoin, WithAmount } from "@left-curve/store/types";
import { formatNumber, formatOrderId, formatUnits } from "@left-curve/dango/utils";

type NotificationSpotActionOrderProps = {
  status: "created" | "canceled" | "fulfilled";
  action: "buy" | "sell";
  base: AnyCoin;
  quote: AnyCoin;
  blockHeight: number;
  order: {
    type: "limit" | "market";
    amount?: string;
    id: string;
    timeCreated: string;
    fee?: string;
    timeCanceled?: string;
    refund?: WithAmount<AnyCoin>[];
    limitPrice?: string;
    averagePrice?: string;
    filled?: string;
    timeUpdated?: string;
  };
};

export const NotificationSpotActionOrder = forwardRef<undefined, NotificationSpotActionOrderProps>(
  ({ status, action, base, quote, order, blockHeight }, _) => {
    const { id, type, amount, limitPrice, refund, timeCreated, timeCanceled, fee, averagePrice } =
      order;

    const { hideModal, setSidebarVisibility, settings } = useApp();
    const { formatNumberOptions } = settings;
    const orderId = formatOrderId(id);
    const { navigate: _navigate_ } = useRouter();

    const navigate = (parameters: Parameters<typeof _navigate_>[0]) => {
      _navigate_(parameters);
      hideModal();
      setSidebarVisibility(false);
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

                <IconLink
                  className="text-tertiary-500 h-4 w-4 cursor-pointer"
                  onClick={() =>
                    navigate({
                      to: `/trade/${base.symbol}-${quote.symbol}`,
                      params: true,
                      replace: true,
                    })
                  }
                />
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
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.modal.blockHeight"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>{blockHeight}</p>
                  <IconLink
                    className="text-tertiary-500 h-4 w-4 cursor-pointer"
                    onClick={() => navigate({ to: `/block/${blockHeight}`, params: true })}
                  />
                </div>
              </div>

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
              {order.filled && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.filledAmount"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>
                      {order.filled} / {amount} {base.symbol}
                    </p>
                  </div>
                </div>
              )}
              {refund && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.tokenReceived"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    {refund.map((r) => (
                      <p
                        key={`refound-${r.denom}`}
                      >{`${formatNumber(formatUnits(r.amount, r.decimals), formatNumberOptions)} ${r.symbol}`}</p>
                    ))}
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
              {averagePrice && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.averagePrice"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>
                      {averagePrice} / {quote.symbol}
                    </p>
                  </div>
                </div>
              )}
              {fee && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.fee"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{fee}</p>
                  </div>
                </div>
              )}
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["notifications.notification.modal.id"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>{orderId}</p>
                  <TextCopy copyText={orderId} className="h-4 w-4" />
                </div>
              </div>
              {timeCreated ? (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.timeCreated"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{format(timeCreated, "dd/MM/yyyy hh:mm a")}</p>
                  </div>
                </div>
              ) : null}
              {timeCanceled && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.timeCanceled"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{format(timeCanceled, "dd/MM/yyyy hh:mm a")}</p>
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
