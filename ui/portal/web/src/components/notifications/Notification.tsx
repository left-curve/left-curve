import { useNavigate } from "@tanstack/react-router";
import { useConfig } from "@left-curve/store";

import { Decimal, formatNumber, formatUnits } from "@left-curve/dango/utils";
import {
  differenceInDays,
  differenceInHours,
  differenceInMinutes,
  format,
  isToday,
} from "date-fns";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  AddressVisualizer,
  IconClose,
  PairAssets,
  twMerge,
  useApp,
  IconSent,
  IconReceived,
  IconNewAccount,
  Badge,
  IconLimitOrder,
  IconMarketOrder,
} from "@left-curve/applets-kit";

import type { PropsWithChildren } from "react";
import type React from "react";
import { type Notification as NotificationType, useNotifications } from "~/hooks/useNotifications";
import { Direction, OrderType } from "@left-curve/dango/types";

const formatNotificationTimestamp = (timestamp: Date): string => {
  const now = new Date();
  if (isToday(timestamp)) {
    const minutesDifference = differenceInMinutes(now, timestamp);
    if (minutesDifference < 1) {
      return "1m";
    }

    if (minutesDifference < 60) {
      return `${minutesDifference}m`;
    }

    const hoursDifference = differenceInHours(now, timestamp);
    if (hoursDifference < 24) {
      return `${hoursDifference}h`;
    }
  }

  const daysDifference = differenceInDays(now, timestamp);
  if (daysDifference === 1) {
    return "1d";
  }

  return format(timestamp, "MM/dd");
};

export type NotificationProps = {
  notification: Notification[keyof Notification];
};

const Container: React.FC<PropsWithChildren> = ({ children }) => {
  return <>{children}</>;
};

type NotificationTransferProps = {
  notification: NotificationType<"transfer">;
};

const NotificationTransfer: React.FC<NotificationTransferProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { getCoinInfo } = useConfig();
  const { id, blockHeight, createdAt, txHash } = notification;
  const { coins, type, fromAddress, toAddress } = notification.data;
  const { formatNumberOptions } = settings;
  const isSent = type === "sent";

  const originAddress = isSent ? fromAddress : toAddress;
  const targetAddress = isSent ? toAddress : fromAddress;

  const Icon = isSent ? IconSent : IconReceived;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div
        className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
        onClick={(event) => {
          const element = event.target as HTMLElement;
          if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
            return;
          }
          onNavigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`);
        }}
      >
        <div className="flex items-center justify-center bg-quaternary-rice w-7 h-7 rounded-sm">
          <Icon className={twMerge(isSent ? "text-red-bean-600" : "text-brand-green")} />
        </div>

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <span className="diatype-m-medium text-secondary-700">
            {m["notifications.notification.transfer.title"]({ action: type })}
          </span>
          <div className="flex gap-2">
            {Object.entries(coins).map(([denom, amount]) => {
              const coin = getCoinInfo(denom);
              return (
                <p
                  className={twMerge(
                    "diatype-m-bold flex-shrink-0 flex items-center justify-center gap-1",
                    {
                      "text-status-success": type === "received",
                      "text-status-fail": type === "sent",
                    },
                  )}
                >
                  <span>
                    {coin.type === "lp" ? (
                      <PairAssets assets={[coin.base, coin.quote]} />
                    ) : (
                      <img
                        src={coin.logoURI}
                        alt={coin.symbol}
                        className="w-5 h-5 select-none drag-none"
                        loading="lazy"
                      />
                    )}
                  </span>
                  {`${isSent ? "−" : "+"}${formatNumber(formatUnits(amount, coin.decimals), {
                    ...formatNumberOptions,
                    maxSignificantDigits: 4,
                  })}  ${coin.symbol}`}
                </p>
              );
            })}
          </div>
          <div className="flex diatype-m-medium text-tertiary-500 flex-wrap items-center gap-1">
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.first"]({ direction: type })}
              </span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={originAddress}
                withIcon
                onClick={onNavigate}
              />
            </div>
            <div className="flex flex-wrap items-center gap-1">
              <span>
                {m["notifications.notification.transfer.direction.second"]({ direction: type })}
              </span>
              <AddressVisualizer
                classNames={{ container: "address-visualizer" }}
                address={targetAddress}
                withIcon
                onClick={onNavigate}
              />{" "}
            </div>
          </div>
        </div>
      </div>
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px] remove-notification"
          onClick={() => deleteNotification(id)}
        />
        <p>{formatNotificationTimestamp(new Date(createdAt))}</p>
      </div>
    </div>
  );
};

type NotificationAccountProps = {
  notification: NotificationType<"account">;
};

const NotificationAccount: React.FC<NotificationAccountProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { address, accountType } = notification.data;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div className="flex items-start gap-2 max-w-full overflow-hidden">
        <div className="flex justify-center items-center bg-tertiary-green w-7 h-7 rounded-sm">
          <IconNewAccount className="text-brand-green h-4 w-4" />
        </div>
        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <div className="flex justify-center items-center gap-2 diatype-m-medium text-secondary-700 capitalize">
            <p>{m["notifications.notification.account.title"]()}</p>
            <Badge className="capitalize" text={accountType} />
          </div>
          <AddressVisualizer address={address} withIcon onClick={onNavigate} />
        </div>
      </div>
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px]"
          onClick={() => deleteNotification(notification.id)}
        />
        <p>{formatNotificationTimestamp(new Date(notification.createdAt))}</p>
      </div>
    </div>
  );
};

type NotificationOrderCreatedProps = {
  notification: NotificationType<"orderCreated">;
};

const NotificationOrderCreated: React.FC<NotificationOrderCreatedProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { getCoinInfo } = useConfig();
  const { id, blockHeight, createdAt, txHash } = notification;
  const { quote_denom, base_denom, price, kind, deposit: depositInfo } = notification.data;
  const { formatNumberOptions } = settings;

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const deposit = getCoinInfo(depositInfo.denom);

  const at = formatNumber(
    Decimal(price)
      .times(Decimal(10).pow(base.decimals - quote.decimals))
      .toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const width = formatNumber(
    Decimal(depositInfo.amount).div(Decimal(10).pow(deposit.decimals)).toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const isLimit = kind === OrderType.Limit;

  const Icon = isLimit ? IconLimitOrder : IconMarketOrder;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div
        className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
        onClick={(event) => {
          const element = event.target as HTMLElement;
          if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
            return;
          }
          onNavigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`);
        }}
      >
        <div
          className={twMerge(
            "flex items-center justify-center min-w-7 min-h-7 w-7 h-7 rounded-sm",
            isLimit ? "bg-blue-200" : "bg-tertiary-green",
          )}
        >
          <Icon className={twMerge(isLimit ? "text-secondary-blue" : "text-brand-green")} />
        </div>

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <div className="flex items-center gap-2 diatype-m-medium text-secondary-700 capitalize">
            <span>{m["notifications.notification.orderCreated.title"]({ orderType: kind })}</span>
            <Badge text="Spot" />
          </div>

          <div className={twMerge("flex-wrap flex items-center gap-1")}>
            <span>{m["common.for"]()}</span>
            <PairAssets
              assets={[base, quote]}
              className="w-5 h-5 min-w-5 min-h-5"
              mL={(i) => `${-i / 2}rem`}
            />
            <span className="diatype-m-bold">
              {base.symbol}-{quote.symbol}
            </span>
            <span>{m["common.at"]()}</span>
            <span className="diatype-m-bold">
              {at} {quote.symbol}
            </span>
            <span>{m["common.width"]()}</span>
            <span className="diatype-m-bold">
              {width} {deposit.symbol}
            </span>
          </div>
        </div>
      </div>
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px] remove-notification"
          onClick={() => deleteNotification(id)}
        />
        <p>{formatNotificationTimestamp(new Date(createdAt))}</p>
      </div>
    </div>
  );
};

type NotificationOrderFilledProps = {
  notification: NotificationType<"orderFilled">;
};

const NotificationOrderFilled: React.FC<NotificationOrderFilledProps> = ({ notification }) => {
  const navigate = useNavigate();
  const { settings, setNotificationMenuVisibility } = useApp();
  const { deleteNotification } = useNotifications();
  const { getCoinInfo } = useConfig();
  const { id, blockHeight, createdAt, txHash } = notification;
  const {
    kind,
    base_denom,
    quote_denom,
    clearing_price,
    cleared,
    direction,
    filled_base,
    filled_quote,
  } = notification.data;

  const { formatNumberOptions } = settings;

  const opInfo =
    direction === Direction.Buy
      ? {
          amount: filled_quote,
          denom: quote_denom,
        }
      : {
          amount: filled_base,
          denom: base_denom,
        };

  const base = getCoinInfo(base_denom);
  const quote = getCoinInfo(quote_denom);
  const deposit = getCoinInfo(opInfo.denom);

  const at = formatNumber(
    Decimal(clearing_price)
      .times(Decimal(10).pow(base.decimals - quote.decimals))
      .toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const width = formatNumber(
    Decimal(opInfo.amount).div(Decimal(10).pow(deposit.decimals)).toFixed(),
    { ...formatNumberOptions, minSignificantDigits: 8, maxSignificantDigits: 8 },
  ).slice(0, 7);

  const isLimit = kind === OrderType.Limit;

  const Icon = isLimit ? IconLimitOrder : IconMarketOrder;

  const onNavigate = (url: string) => {
    setNotificationMenuVisibility(false);
    navigate({ to: url });
  };

  return (
    <div className="flex items-end justify-between gap-2 p-2 rounded-lg hover:bg-surface-quaternary-rice max-w-full group">
      <div
        className="flex items-start gap-2 max-w-full overflow-hidden cursor-pointer"
        onClick={(event) => {
          const element = event.target as HTMLElement;
          if (element.closest(".address-visualizer") || element.closest(".remove-notification")) {
            return;
          }
          onNavigate(txHash ? `/tx/${txHash}` : `/block/${blockHeight}`);
        }}
      >
        <div
          className={twMerge(
            "flex items-center justify-center min-w-7 min-h-7 w-7 h-7 rounded-sm",
            isLimit ? "bg-blue-200" : "bg-tertiary-green",
          )}
        >
          <Icon className={twMerge(isLimit ? "text-secondary-blue" : "text-brand-green")} />
        </div>

        <div className="flex flex-col max-w-[calc(100%)] overflow-hidden">
          <div className="flex items-center gap-2 diatype-m-medium text-secondary-700 capitalize">
            <span>{m["notifications.notification.orderCreated.title"]({ orderType: kind })}</span>
            <Badge text="Spot" />
          </div>

          <div className={twMerge("flex-wrap flex items-center gap-1")}>
            <span>{m["common.for"]()}</span>
            <PairAssets
              assets={[base, quote]}
              className="w-5 h-5 min-w-5 min-h-5"
              mL={(i) => `${-i / 2}rem`}
            />
            <span className="diatype-m-bold">
              {base.symbol}-{quote.symbol}
            </span>
            <span>{m["common.at"]()}</span>
            <span className="diatype-m-bold">
              {at} {quote.symbol}
            </span>
            <span>{m["common.width"]()}</span>
            <span className="diatype-m-bold">
              {width} {deposit.symbol}
            </span>
          </div>
        </div>
      </div>
      <div className="flex flex-col diatype-sm-medium text-tertiary-500 min-w-fit items-center relative">
        <IconClose
          className="absolute w-6 h-6 cursor-pointer group-hover:block hidden top-[-26px] remove-notification"
          onClick={() => deleteNotification(id)}
        />
        <p>{formatNotificationTimestamp(new Date(createdAt))}</p>
      </div>
    </div>
  );
};

export const Notification = Object.assign(Container, {
  Transfer: NotificationTransfer,
  OrderCreated: NotificationOrderCreated,
  OrderFilled: NotificationOrderFilled,
  Account: NotificationAccount,
});
