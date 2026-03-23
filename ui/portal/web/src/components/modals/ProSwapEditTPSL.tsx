import { Button, IconButton, IconClose, Input, useApp } from "@left-curve/applets-kit";

import { formatNumber } from "@left-curve/dango/utils";
import {
  perpsOrdersByUserStore,
  useAccount,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { forwardRef, useMemo, useState } from "react";

type ProSwapEditTPSLProps = {
  pairId: string;
  size: string;
  entryPrice: string;
  currentPrice: number;
};

export const ProSwapEditTPSL = forwardRef<void, ProSwapEditTPSLProps>(
  ({ pairId, size, entryPrice, currentPrice }) => {
    const { hideModal, settings } = useApp();
    const { formatNumberOptions } = settings;
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const perpsOrders = perpsOrdersByUserStore((s) => s.orders);

    const isLong = Number(size) > 0;
    const label = pairId.replace("perp/", "").replace(/usd$/i, "").toUpperCase();

    const existingOrders = useMemo(() => {
      let tpOrderId: string | null = null;
      let slOrderId: string | null = null;
      let tpPrice = "";
      let slPrice = "";

      if (perpsOrders) {
        for (const [orderId, order] of Object.entries(perpsOrders)) {
          if (order.pairId !== pairId || !("conditional" in order.kind)) continue;
          const { triggerPrice, triggerDirection } = order.kind.conditional;
          const isTp = isLong ? triggerDirection === "above" : triggerDirection === "below";
          if (isTp) {
            tpOrderId = orderId;
            tpPrice = triggerPrice;
          } else {
            slOrderId = orderId;
            slPrice = triggerPrice;
          }
        }
      }

      return { tpOrderId, slOrderId, tpPrice, slPrice };
    }, [perpsOrders, pairId, isLong]);

    const [tpPrice, setTpPrice] = useState(existingOrders.tpPrice);
    const [slPrice, setSlPrice] = useState(existingOrders.slPrice);

    const { isPending, mutateAsync: submit } = useSubmitTx({
      mutation: {
        mutationFn: async () => {
          if (!signingClient || !account) throw new Error("No signing client available");

          const orderSize = isLong
            ? `-${Math.abs(Number(size))}`
            : Math.abs(Number(size)).toString();

          if (existingOrders.tpOrderId) {
            await signingClient.cancelConditionalOrder({
              sender: account.address,
              request: { one: existingOrders.tpOrderId },
            });
          }
          if (existingOrders.slOrderId) {
            await signingClient.cancelConditionalOrder({
              sender: account.address,
              request: { one: existingOrders.slOrderId },
            });
          }

          if (tpPrice) {
            await signingClient.submitConditionalOrder({
              sender: account.address,
              pairId,
              size: orderSize,
              triggerPrice: tpPrice,
              triggerDirection: isLong ? "above" : "below",
              maxSlippage: "0.05",
            });
          }

          if (slPrice) {
            await signingClient.submitConditionalOrder({
              sender: account.address,
              pairId,
              size: orderSize,
              triggerPrice: slPrice,
              triggerDirection: isLong ? "below" : "above",
              maxSlippage: "0.05",
            });
          }
        },
        onSuccess: () => {
          hideModal();
        },
      },
    });

    const tpPnl = useMemo(() => {
      if (!tpPrice) return null;
      const absSize = Math.abs(Number(size));
      const diff = isLong
        ? Number(tpPrice) - Number(entryPrice)
        : Number(entryPrice) - Number(tpPrice);
      return diff * absSize;
    }, [tpPrice, size, entryPrice, isLong]);

    const slPnl = useMemo(() => {
      if (!slPrice) return null;
      const absSize = Math.abs(Number(size));
      const diff = isLong
        ? Number(slPrice) - Number(entryPrice)
        : Number(entryPrice) - Number(slPrice);
      return diff * absSize;
    }, [slPrice, size, entryPrice, isLong]);

    return (
      <div className="flex flex-col bg-surface-primary-rice md:border border-outline-secondary-gray pt-0 md:pt-6 rounded-xl relative p-4 md:p-6 gap-5 w-full md:max-w-[30rem]">
        <h2 className="text-ink-primary-900 h4-bold w-full">TP/SL for Position</h2>

        <div className="flex flex-col gap-1">
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">Coin</p>
            <p className="diatype-sm-medium text-ink-secondary-700">{label}</p>
          </div>
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">Position</p>
            <p
              className={`diatype-sm-medium ${isLong ? "text-utility-success-600" : "text-utility-error-600"}`}
            >
              {isLong ? "Long" : "Short"} {Math.abs(Number(size))}
            </p>
          </div>
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">Entry Price</p>
            <p className="diatype-sm-medium text-ink-secondary-700">
              ${formatNumber(entryPrice, formatNumberOptions)}
            </p>
          </div>
          <div className="w-full flex gap-2 items-center justify-between">
            <p className="diatype-sm-regular text-ink-tertiary-500">Mark Price</p>
            <p className="diatype-sm-medium text-ink-secondary-700">
              ${formatNumber(currentPrice.toString(), formatNumberOptions)}
            </p>
          </div>
        </div>
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <Input
              placeholder="0"
              label="TP Price"
              value={tpPrice}
              onChange={(e) => setTpPrice(e.target.value.replace(/[^0-9.]/g, ""))}
            />
            {tpPnl !== null ? (
              <p
                className={`diatype-sm-regular text-right ${tpPnl >= 0 ? "text-utility-success-600" : "text-utility-error-600"}`}
              >
                Expected PnL: {tpPnl >= 0 ? "+" : ""}
                {formatNumber(tpPnl.toFixed(2), formatNumberOptions)} USD
              </p>
            ) : null}
          </div>
          <div className="flex flex-col gap-1">
            <Input
              placeholder="0"
              label="SL Price"
              value={slPrice}
              onChange={(e) => setSlPrice(e.target.value.replace(/[^0-9.]/g, ""))}
            />
            {slPnl !== null ? (
              <p
                className={`diatype-sm-regular text-right ${slPnl >= 0 ? "text-utility-success-600" : "text-utility-error-600"}`}
              >
                Expected PnL: {slPnl >= 0 ? "+" : ""}
                {formatNumber(slPnl.toFixed(2), formatNumberOptions)} USD
              </p>
            ) : null}
          </div>
        </div>
        <div className="flex flex-col gap-1">
          <p className="diatype-xs-regular text-ink-tertiary-500">
            Take-profit and stop-loss orders apply to the entire position. They automatically cancel
            after closing the position. A market order is triggered when the TP/SL price is reached.
          </p>
        </div>
        <IconButton
          className="hidden md:block absolute right-4 top-4"
          variant="link"
          onClick={() => hideModal()}
        >
          <IconClose />
        </IconButton>
        <Button
          fullWidth
          isLoading={isPending}
          isDisabled={!tpPrice && !slPrice}
          onClick={() => submit()}
        >
          Confirm
        </Button>
      </div>
    );
  },
);
