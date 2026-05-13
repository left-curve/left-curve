import {
  Button,
  FormattedNumber,
  IconButton,
  IconClose,
  Modals,
  useApp,
} from "@left-curve/applets-kit";

import { TPSLPositionInfo } from "./TPSLPositionInfo";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { useAccount, useSigningClient, useSubmitTx } from "@left-curve/store";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef, useMemo, useState } from "react";

import type { ConditionalOrder, TriggerDirection } from "@left-curve/dango/types";

type ProSwapEditedSLProps = {
  pairId: string;
  symbol: string;
  entryPrice: string;
  markPrice: string;
  size: string;
  conditionalOrderAbove?: ConditionalOrder;
  conditionalOrderBelow?: ConditionalOrder;
};

export const ProSwapEditedSL = forwardRef<void, ProSwapEditedSLProps>(
  ({
    pairId,
    symbol,
    entryPrice,
    markPrice,
    size,
    conditionalOrderAbove,
    conditionalOrderBelow,
  }) => {
    const { hideModal, showModal } = useApp();
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const queryClient = useQueryClient();
    const [cancelingDirection, setCancelingDirection] = useState<TriggerDirection | null>(null);

    const sizeNum = Number(size);
    const isLong = sizeNum > 0;
    const absSize = Math.abs(sizeNum);
    const entryPriceNum = Number(entryPrice);

    const tpOrder = isLong ? conditionalOrderAbove : conditionalOrderBelow;
    const slOrder = isLong ? conditionalOrderBelow : conditionalOrderAbove;

    const tpDirection: TriggerDirection = isLong ? "above" : "below";
    const slDirection: TriggerDirection = isLong ? "below" : "above";

    const tpPnl = useMemo(() => {
      if (!tpOrder) return null;
      const trigger = Number(tpOrder.triggerPrice);
      return isLong ? (trigger - entryPriceNum) * absSize : (entryPriceNum - trigger) * absSize;
    }, [tpOrder, entryPriceNum, isLong, absSize]);

    const slPnl = useMemo(() => {
      if (!slOrder) return null;
      const trigger = Number(slOrder.triggerPrice);
      return isLong ? (trigger - entryPriceNum) * absSize : (entryPriceNum - trigger) * absSize;
    }, [slOrder, entryPriceNum, isLong, absSize]);

    const { mutateAsync: cancelOrder } = useSubmitTx({
      submission: {
        success: m["modals.tpsl.orderCanceled"](),
      },
      mutation: {
        mutationFn: async (direction: TriggerDirection) => {
          if (!signingClient) throw new Error("No signing client available");
          if (!account) throw new Error("No account found");

          setCancelingDirection(direction);

          await signingClient.cancelConditionalOrder({
            sender: account.address,
            request: {
              one: {
                pairId,
                triggerDirection: direction,
              },
            },
          });
        },
        onSuccess: () => {
          setCancelingDirection(null);
          queryClient.invalidateQueries({ queryKey: ["prices"] });
          queryClient.invalidateQueries({ queryKey: ["perpsTradeHistory", account?.address] });
        },
        onError: () => {
          setCancelingDirection(null);
        },
      },
    });

    const openEditModal = () => {
      showModal(Modals.ProSwapEditTPSL, {
        pairId,
        symbol,
        entryPrice,
        markPrice,
        size,
        conditionalOrderAbove,
        conditionalOrderBelow,
      });
    };

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[30rem]">
        <h2 className="text-ink-primary-900 h4-bold w-full">{m["modals.tpsl.title"]()}</h2>

        <TPSLPositionInfo
          symbol={symbol}
          isLong={isLong}
          absSize={absSize}
          entryPrice={entryPrice}
          markPrice={markPrice}
        />

        <div className="flex flex-col gap-3">
          {tpOrder ? (
            <div className="flex items-start justify-between gap-2 p-3 rounded-lg bg-surface-secondary-oat">
              <div className="flex flex-col gap-0.5">
                <p className="diatype-sm-medium text-ink-secondary-700">
                  {m["modals.tpsl.takeProfit"]()}
                </p>
                <p className="diatype-sm-regular text-ink-tertiary-500">
                  {isLong ? m["modals.tpsl.priceAbove"]() : m["modals.tpsl.priceBelow"]()}{" "}
                  <FormattedNumber
                    number={tpOrder.triggerPrice}
                    formatOptions={{ currency: "USD" }}
                    as="span"
                  />
                </p>
                {tpPnl !== null ? (
                  <p
                    className={`diatype-sm-regular ${tpPnl >= 0 ? "text-utility-success-600" : "text-utility-error-600"}`}
                  >
                    {m["modals.tpsl.expectedPl"]()}{" "}
                    <FormattedNumber
                      number={tpPnl.toFixed(2)}
                      formatOptions={{ currency: "USD" }}
                      as="span"
                    />
                  </p>
                ) : null}
              </div>
              <Button
                variant="link"
                size="sm"
                isLoading={cancelingDirection === tpDirection}
                onClick={() => cancelOrder(tpDirection)}
              >
                {m["modals.tpsl.cancel"]()}
              </Button>
            </div>
          ) : (
            <div className="flex items-center justify-between p-3 rounded-lg bg-surface-secondary-oat">
              <p className="diatype-sm-regular text-ink-tertiary-500">
                {m["modals.tpsl.noTakeProfitSet"]()}
              </p>
              <Button variant="link" size="sm" onClick={openEditModal}>
                {m["modals.tpsl.add"]()}
              </Button>
            </div>
          )}

          {slOrder ? (
            <div className="flex items-start justify-between gap-2 p-3 rounded-lg bg-surface-secondary-oat">
              <div className="flex flex-col gap-0.5">
                <p className="diatype-sm-medium text-ink-secondary-700">
                  {m["modals.tpsl.stopLoss"]()}
                </p>
                <p className="diatype-sm-regular text-ink-tertiary-500">
                  {isLong ? m["modals.tpsl.priceBelow"]() : m["modals.tpsl.priceAbove"]()}{" "}
                  <FormattedNumber
                    number={slOrder.triggerPrice}
                    formatOptions={{ currency: "USD" }}
                    as="span"
                  />
                </p>
                {slPnl !== null ? (
                  <p
                    className={`diatype-sm-regular ${slPnl >= 0 ? "text-utility-success-600" : "text-utility-error-600"}`}
                  >
                    {m["modals.tpsl.expectedPl"]()}{" "}
                    <FormattedNumber
                      number={slPnl.toFixed(2)}
                      formatOptions={{ currency: "USD" }}
                      as="span"
                    />
                  </p>
                ) : null}
              </div>
              <Button
                variant="link"
                size="sm"
                isLoading={cancelingDirection === slDirection}
                onClick={() => cancelOrder(slDirection)}
              >
                {m["modals.tpsl.cancel"]()}
              </Button>
            </div>
          ) : (
            <div className="flex items-center justify-between p-3 rounded-lg bg-surface-secondary-oat">
              <p className="diatype-sm-regular text-ink-tertiary-500">
                {m["modals.tpsl.noStopLossSet"]()}
              </p>
              <Button variant="link" size="sm" onClick={openEditModal}>
                {m["modals.tpsl.add"]()}
              </Button>
            </div>
          )}
        </div>

        <div className="flex flex-col gap-1">
          <p className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.tpsl.helpDefault"]()}
          </p>
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>
        <Button fullWidth variant="secondary" onClick={openEditModal}>
          {m["modals.tpsl.edit"]()}
        </Button>
      </div>
    );
  },
);
