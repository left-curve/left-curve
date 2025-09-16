import { useApp } from "@left-curve/applets-kit";

import {
  Badge,
  formatDate,
  IconButton,
  IconClose,
  IconLink,
  PairAssets,
  TextCopy,
} from "@left-curve/applets-kit";

import { forwardRef } from "react";
import { twMerge } from "@left-curve/applets-kit";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { formatNumber, formatOrderId, formatUnits } from "@left-curve/dango/utils";

import type { useNavigate } from "@tanstack/react-router";
import type { AnyCoin, WithAmount } from "@left-curve/store/types";

type ActivitySpotOrderModalProps = {
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
  navigate: ReturnType<typeof useNavigate>;
};

export const ActivitySpotOrderModal = forwardRef<undefined, ActivitySpotOrderModalProps>(
  ({ status, action, base, quote, order, blockHeight, navigate: _navigate_ }, _) => {
    const { id, type, amount, limitPrice, refund, timeCreated, timeCanceled, fee, averagePrice } =
      order;

    const { hideModal, setSidebarVisibility, settings } = useApp();
    const { formatNumberOptions, timeFormat, dateFormat } = settings;
    const orderId = formatOrderId(id);

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
            {m["activities.activity.modal.spotOrderAction"]({ status })}
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
                    ? m["activities.activity.modal.spotOrderBuy"]({ orderType: type })
                    : m["activities.activity.modal.spotOrderSell"]({ orderType: type })}
                </p>
                <Badge text="Spot" color="blue" size="s" />
              </div>
            </div>
            <span className="w-full h-[1px] bg-secondary-gray my-2" />
            <div className="flex flex-col gap-2 w-full">
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["activities.activity.modal.blockHeight"]()}
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
                    {m["activities.activity.modal.amount"]()}
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
                    {m["activities.activity.modal.filledAmount"]()}
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
                    {m["activities.activity.modal.tokenReceived"]()}
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
                    {m["activities.activity.modal.limitPrice"]()}
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
                    {m["activities.activity.modal.averagePrice"]()}
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
                    {m["activities.activity.modal.fee"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{fee}</p>
                  </div>
                </div>
              )}
              <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                <p className="diatype-sm-regular text-tertiary-500">
                  {m["activities.activity.modal.id"]()}
                </p>
                <div className="flex items-center gap-1">
                  <p>{orderId}</p>
                  <TextCopy copyText={orderId} className="h-4 w-4" />
                </div>
              </div>
              {timeCreated ? (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["activities.activity.modal.timeCreated"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{formatDate(timeCreated, `${dateFormat} ${timeFormat}`)}</p>
                  </div>
                </div>
              ) : null}
              {timeCanceled && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["activities.activity.modal.timeCanceled"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>{formatDate(timeCanceled, `${dateFormat} ${timeFormat}`)}</p>
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
