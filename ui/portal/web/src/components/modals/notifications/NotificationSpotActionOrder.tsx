import { Badge, IconButton, IconClose, IconLink, TextCopy, useApp } from "@left-curve/applets-kit";
import { forwardRef, Ref } from "react";
import { m } from "@left-curve/foundation/paraglide/messages.js";

type NotificationSpotActionOrderProps = {
  status: "created" | "canceled" | "fulfilled";
  action: "buy" | "sell";
  orderDetails: {
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

export const NotificationSpotActionOrder = forwardRef<NotificationSpotActionOrderProps, any>(
  ({ status, action, orderDetails }, ref) => {
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
                <div className="flex">
                  <img
                    src="https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg"
                    alt="BTC"
                    className="h-8 w-8"
                  />
                  <img
                    src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                    alt="USDC"
                    className="h-8 w-8 -ml-4"
                  />
                </div>
                <p className="h4-bold text-secondary-700">BTC/USDC</p>
                <IconLink className="text-tertiary-500 h-4 w-4" />
              </div>
              <div className="flex gap-2 diatype-sm-medium">
                <p className={action === "buy" ? "text-status-success" : "text-status-fail"}>
                  {action === "buy"
                    ? m["notifications.notification.modal.spotOrderBuy"]()
                    : m["notifications.notification.modal.spotOrderSell"]()}
                </p>
                <Badge text="Spot" color="blue" size="s" />
              </div>
            </div>
            <span className="w-full h-[1px] bg-secondary-gray my-2" />
            <div className="flex flex-col gap-2 w-full">
              {orderDetails.amount && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.amount"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>0.000123 BTC</p>
                  </div>
                </div>
              )}
              {orderDetails.filledAmount && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.filledAmount"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>0.000123 / 0.000123 BTC</p>
                  </div>
                </div>
              )}
              {orderDetails.tokenReceived && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.tokenReceived"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>0.000123 BTC</p>
                  </div>
                </div>
              )}
              {orderDetails.limitPrice && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.limitPrice"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>123.45 ETH</p>
                  </div>
                </div>
              )}
              {orderDetails.averagePrice && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.averagePrice"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>123.45 ETH</p>
                  </div>
                </div>
              )}
              {orderDetails.fee && (
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
              {orderDetails.timeCanceled && (
                <div className="flex items-center justify-between gap-2 diatype-sm-medium text-secondary-700">
                  <p className="diatype-sm-regular text-tertiary-500">
                    {m["notifications.notification.modal.timeCanceled"]()}
                  </p>
                  <div className="flex items-center gap-1">
                    <p>August 14, 2025 10:15 AM</p>
                  </div>
                </div>
              )}
              {orderDetails.timeUpdated && (
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
