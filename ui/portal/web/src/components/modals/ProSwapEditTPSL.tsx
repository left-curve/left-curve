import {
  Button,
  Checkbox,
  FormattedNumber,
  IconButton,
  IconClose,
  Input,
  Range,
  numberMask,
  useApp,
  useInputs,
} from "@left-curve/applets-kit";

import { Decimal } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  perpsTradeSettingsStore,
  useAccount,
  useAppConfig,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { PERPS_DEFAULT_SLIPPAGE } from "~/constants";
import { useQueryClient } from "@tanstack/react-query";
import { forwardRef, useEffect, useMemo, useState } from "react";

import type { SubmitConditionalOrderInput } from "@left-curve/dango/actions";

import { TPSLPositionInfo } from "./TPSLPositionInfo";
import { useTPSLPriceSync } from "../dex/hooks/useTPSLPriceSync";

type ProSwapEditTPSLProps = {
  pairId: string;
  symbol: string;
  entryPrice: string;
  markPrice: string;
  size: string;
  conditionalOrderAbove?: { triggerPrice: string; maxSlippage: string };
  conditionalOrderBelow?: { triggerPrice: string; maxSlippage: string };
};


export const ProSwapEditTPSL = forwardRef<void, ProSwapEditTPSLProps>(
  ({
    pairId,
    symbol,
    entryPrice,
    markPrice,
    size,
    conditionalOrderAbove,
    conditionalOrderBelow,
  }) => {
    const { hideModal } = useApp();
    const { account } = useAccount();
    const { data: signingClient } = useSigningClient();
    const queryClient = useQueryClient();
    const { data: appConfig } = useAppConfig();

    const sizeNum = Number(size);
    const isLong = sizeNum > 0;
    const absSize = Math.abs(sizeNum);
    const entryPriceNum = Number(entryPrice);
    const markPriceNum = Number(markPrice);

    // Effective leverage on this position. Dango is cross-margin only
    // (isolated is "Coming Soon"), so positions don't carry a stored
    // leverage — we use the user's currently-selected leverage for the
    // pair, falling back to max leverage (1 / initialMarginRatio) when
    // nothing is stored. This matches the logic in TradeMenu.tsx and
    // keeps the two TP/SL entry points consistent.
    const maxLeverage = useMemo(() => {
      const params = appConfig?.perpsPairs?.[pairId];
      const ratio = Number(params?.initialMarginRatio ?? 0);
      return ratio > 0 ? Math.floor(1 / ratio) : 100;
    }, [appConfig, pairId]);
    const storedLeverage = perpsTradeSettingsStore((s) => s.leverageByPair[pairId]);
    const selectedLeverage = useMemo(() => {
      if (!storedLeverage) return maxLeverage;
      return Math.min(Math.max(Math.round(storedLeverage), 1), maxLeverage);
    }, [storedLeverage, maxLeverage]);

    const existingTp = isLong ? conditionalOrderAbove : conditionalOrderBelow;
    const existingSl = isLong ? conditionalOrderBelow : conditionalOrderAbove;

    const controllers = useInputs();
    const { register, setValue, inputs } = controllers;

    const tpPrice = inputs.tpPrice?.value || "";
    const slPrice = inputs.slPrice?.value || "";

    const [configureAmount, setConfigureAmount] = useState(false);
    const [sizePercent, setSizePercent] = useState(100);

    const { onTpPriceChange, onTpPercentChange, onSlPriceChange, onSlPercentChange } =
      useTPSLPriceSync({
        setValue,
        // Use entry price (not mark) as the reference so the ROI% shown
        // in the input matches the "Expected P/L" USD line below, which
        // is also computed relative to entry. Validation still guards
        // against immediate triggers by comparing to the mark price
        // separately, below.
        referencePrice: entryPriceNum,
        leverage: selectedLeverage,
        isBuyDirection: isLong,
      });

    useEffect(() => {
      if (existingTp) {
        onTpPriceChange(existingTp.triggerPrice);
      }
      if (existingSl) {
        onSlPriceChange(existingSl.triggerPrice);
      }
    }, []);

    const orderSize = useMemo(() => {
      if (!configureAmount) return undefined;
      return Decimal(absSize).mul(Decimal(sizePercent).div(100)).toFixed(6);
    }, [configureAmount, sizePercent, absSize]);

    const validationError = useMemo(() => {
      const tp = Number(tpPrice);
      const sl = Number(slPrice);

      // Validation uses mark price (falling back to entry) because its job
      // is to reject triggers that would fire immediately against the
      // *current* market. This is intentionally different from the ROI
      // reference used for percent display, which is entry-based so that
      // the percent and the Expected P/L USD line agree.
      const validationReference = markPriceNum > 0 ? markPriceNum : entryPriceNum;
      if (tp > 0) {
        if (isLong && tp <= validationReference) return m["modals.tpsl.errors.tpAboveForLongs"]();
        if (!isLong && tp >= validationReference) return m["modals.tpsl.errors.tpBelowForShorts"]();
      }
      if (sl > 0) {
        if (isLong && sl >= validationReference) return m["modals.tpsl.errors.slBelowForLongs"]();
        if (!isLong && sl <= validationReference) return m["modals.tpsl.errors.slAboveForShorts"]();
      }
      return null;
    }, [tpPrice, slPrice, isLong, markPriceNum, entryPriceNum]);

    const expectedTpPnl = useMemo(() => {
      const tp = Number(tpPrice);
      if (tp <= 0) return null;
      const effectiveSize = orderSize ? Number(orderSize) : absSize;
      return isLong ? (tp - entryPriceNum) * effectiveSize : (entryPriceNum - tp) * effectiveSize;
    }, [tpPrice, entryPriceNum, isLong, absSize, orderSize]);

    const expectedSlPnl = useMemo(() => {
      const sl = Number(slPrice);
      if (sl <= 0) return null;
      const effectiveSize = orderSize ? Number(orderSize) : absSize;
      return isLong ? (sl - entryPriceNum) * effectiveSize : (entryPriceNum - sl) * effectiveSize;
    }, [slPrice, entryPriceNum, isLong, absSize, orderSize]);

    const { isPending, mutateAsync: submitOrders } = useSubmitTx({
      submission: {
        success: m["modals.tpsl.tpslUpdated"](),
      },
      mutation: {
        mutationFn: async () => {
          if (!signingClient) throw new Error("No signing client available");
          if (!account) throw new Error("No account found");

          const tp = Number(tpPrice);
          const sl = Number(slPrice);

          const sizeField = orderSize ? { size: isLong ? `-${orderSize}` : orderSize } : {};

          const orders: SubmitConditionalOrderInput[] = [];

          if (tp > 0) {
            orders.push({
              pairId,
              triggerPrice: tpPrice,
              triggerDirection: isLong ? "above" : "below",
              maxSlippage: PERPS_DEFAULT_SLIPPAGE,
              ...sizeField,
            });
          }

          if (sl > 0) {
            orders.push({
              pairId,
              triggerPrice: slPrice,
              triggerDirection: isLong ? "below" : "above",
              maxSlippage: PERPS_DEFAULT_SLIPPAGE,
              ...sizeField,
            });
          }

          if (orders.length === 0) return;

          await signingClient.submitConditionalOrders({
            sender: account.address,
            orders,
          });
        },
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: ["prices"] });
          queryClient.invalidateQueries({ queryKey: ["perpsTradeHistory", account?.address] });
          hideModal();
        },
      },
    });

    const hasInput = Number(tpPrice) > 0 || Number(slPrice) > 0;

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
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <div className="flex gap-2">
              <Input
                placeholder="0"
                label={m["modals.tpsl.tpPrice"]()}
                {...register("tpPrice", { mask: numberMask })}
                onChange={(e) => onTpPriceChange(typeof e === "string" ? e : e.target.value)}
              />
              <Input
                placeholder="0"
                label={m["modals.tpsl.gain"]()}
                classNames={{ base: "max-w-[6rem]" }}
                endContent="%"
                {...register("tpPercent", { mask: numberMask })}
                onChange={(e) => onTpPercentChange(typeof e === "string" ? e : e.target.value)}
              />
            </div>
            <p className="text-ink-tertiary-500 diatype-sm-regular text-right">
              {m["modals.tpsl.expectedPl"]()}{" "}
              {expectedTpPnl !== null ? (
                <FormattedNumber
                  number={expectedTpPnl.toFixed(2)}
                  formatOptions={{ currency: "USD" }}
                  as="span"
                />
              ) : (
                m["modals.tpsl.zeroUsd"]()
              )}
            </p>
          </div>
          <div className="flex flex-col gap-1">
            <div className="flex gap-2">
              <Input
                placeholder="0"
                label={m["modals.tpsl.slPrice"]()}
                {...register("slPrice", { mask: numberMask })}
                onChange={(e) => onSlPriceChange(typeof e === "string" ? e : e.target.value)}
              />
              <Input
                placeholder="0"
                label={m["modals.tpsl.loss"]()}
                classNames={{ base: "max-w-[6rem]" }}
                endContent="%"
                {...register("slPercent", { mask: numberMask })}
                onChange={(e) => onSlPercentChange(typeof e === "string" ? e : e.target.value)}
              />
            </div>
            <p className="text-ink-tertiary-500 diatype-sm-regular text-right">
              {m["modals.tpsl.expectedPl"]()}{" "}
              {expectedSlPnl !== null ? (
                <FormattedNumber
                  number={expectedSlPnl.toFixed(2)}
                  formatOptions={{ currency: "USD" }}
                  as="span"
                />
              ) : (
                m["modals.tpsl.zeroUsd"]()
              )}
            </p>
          </div>

          {validationError ? (
            <p className="diatype-xs-regular text-utility-error-600">{validationError}</p>
          ) : null}

          <Checkbox
            checked={configureAmount}
            onChange={() => setConfigureAmount((prev) => !prev)}
            label={m["modals.tpsl.configureAmount"]()}
            radius="md"
          />
          {configureAmount ? (
            <Range
              minValue={1}
              maxValue={100}
              defaultValue={100}
              value={sizePercent}
              onChange={(v) => setSizePercent(v)}
              inputEndContent={symbol}
              withInput
              classNames={{ input: "max-w-[10rem]" }}
            />
          ) : null}
        </div>
        <div className="flex flex-col gap-1">
          <p className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.tpsl.helpDefault"]()}
          </p>
          <p className="diatype-xs-regular text-ink-tertiary-500">
            {m["modals.tpsl.helpConfigured"]()}
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
          isDisabled={!hasInput || validationError !== null}
          onClick={() => submitOrders()}
        >
          {m["modals.tpsl.confirm"]()}
        </Button>
      </div>
    );
  },
);
