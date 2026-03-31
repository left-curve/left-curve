import {
  useAccount,
  useAppConfig,
  useConfig,
  usePrices,
  tradePairStore,
  toPerpsPairId,
  tradeInfoStore,
  useTradeCoins,
  useSpotMaxSize,
  usePerpsMaxSize,
  useSpotSubmission,
  usePerpsSubmission,
  useErrorHandler,
  perpsUserStateStore,
} from "@left-curve/store";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import {
  Button,
  Checkbox,
  CoinSelector,
  IconButton,
  IconChevronDownFill,
  IconUser,
  Input,
  InputSizeWithMax,
  Modals,
  Range,
  Tabs,
  numberMask,
  twMerge,
  useApp,
  type useInputs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";

import { Decimal, formatNumber, parseUnits } from "@left-curve/dango/utils";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { orderBookStore } from "@left-curve/store";

import { isFeatureEnabled } from "~/featureFlags";
import type React from "react";

const InfoRow: React.FC<{
  label: string;
  value: React.ReactNode;
  className?: string;
}> = ({ label, value, className }) => (
  <div className="flex items-center justify-between gap-2">
    <p className={twMerge("diatype-xs-regular text-ink-tertiary-500", className)}>{label}</p>
    <p className="diatype-xs-medium text-ink-secondary-700">{value}</p>
  </div>
);

const TradeSubmitButton: React.FC<{
  action: "buy" | "sell";
  label: string;
  isDisabled: boolean;
  isPending: boolean;
  onSubmit: () => void;
}> = ({ action, label, isDisabled, isPending, onSubmit }) => {
  const { isConnected } = useAccount();
  const { showModal } = useApp();

  if (!isConnected) {
    return (
      <div className="px-4">
        <Button
          variant={action === "sell" ? "primary" : "tertiary"}
          fullWidth
          size="md"
          onClick={() => showModal(Modals.Authenticate, { action: "signin" })}
        >
          {m["dex.protrade.spot.enableTrading"]()}
        </Button>
      </div>
    );
  }

  return (
    <div className="px-4">
      <Button
        variant={action === "sell" ? "primary" : "tertiary"}
        fullWidth
        size="md"
        isDisabled={isDisabled}
        isLoading={isPending}
        onClick={onSubmit}
      >
        {label}
      </Button>
    </div>
  );
};

type TradeMenuProps = {
  className?: string;
  controllers: ReturnType<typeof useInputs>;
};

export const TradeMenu: React.FC<TradeMenuProps> = (props) => {
  const { isLg } = useMediaQuery();
  return <>{isLg ? <Menu {...props} /> : <MenuMobile {...props} />}</>;
};

const SpotTradeMenu: React.FC<TradeMenuProps> = ({ controllers }) => {
  const { settings, toast } = useApp();
  const { formatNumberOptions } = settings;
  const { isConnected } = useAccount();
  const { data: appConfig } = useAppConfig();
  const { getPrice, isFetched } = usePrices({ defaultFormatOptions: formatNumberOptions });
  const queryClient = useQueryClient();
  const { account } = useAccount();
  const onError = useErrorHandler({
    toast: toast.error,
    title: m["dex.protrade.orderFailed"](),
    fallbackMessage: m["errors.failureRequest"](),
  });

  const pairId = tradePairStore((s) => s.pairId);
  const action = tradeInfoStore((s) => s.action);
  const operation = tradeInfoStore((s) => s.operation);

  const { baseCoin, quoteCoin } = useTradeCoins({ pairId, mode: "spot" });

  const [sizeCoinDenom, setSizeCoinDenom] = useState(pairId.quoteDenom);

  useEffect(() => {
    setSizeCoinDenom(pairId.quoteDenom);
  }, [pairId.quoteDenom]);

  useEffect(() => {
    setSizeCoinDenom(action === "buy" ? pairId.quoteDenom : pairId.baseDenom);
    controllers.setValue("size", "");
  }, [action]);

  const sizeCoin = sizeCoinDenom === baseCoin.denom ? baseCoin : quoteCoin;
  const availableCoin = action === "buy" ? quoteCoin : baseCoin;

  const { register, setValue, inputs } = controllers;
  const size = inputs.size?.value || "0";
  const priceValue = inputs.price?.value || "0";

  const maxSizeAmount = useSpotMaxSize({
    availableCoin,
    sizeCoin,
    action,
    operation,
    priceValue,
  });

  const amount = useMemo(() => {
    if (size === "0") return { base: "0", quote: "0" };

    const price = (() => {
      if (operation === "market") {
        const { orderBook } = orderBookStore.getState();
        if (!orderBook?.midPrice) return null;
        return parseUnits(orderBook.midPrice, baseCoin.decimals - quoteCoin.decimals, true);
      }
      if (priceValue === "0") return null;
      return priceValue;
    })();

    if (!price) return { base: "0", quote: "0" };

    const isBaseSize = sizeCoin.denom === baseCoin.denom;
    const isQuoteSize = !isBaseSize;

    return {
      base: isBaseSize ? size : Decimal(size).divFloor(price).toFixed(),
      quote: isQuoteSize ? size : Decimal(size).mulCeil(price).toFixed(),
    };
  }, [operation, sizeCoin, baseCoin, quoteCoin, size, priceValue]);

  useEffect(() => {
    setValue("price", getPrice(1, pairId.baseDenom).toFixed(4));
  }, [isFetched, pairId]);

  const submission = useSpotSubmission({
    pairId,
    baseCoin,
    quoteCoin,
    availableCoin,
    sizeCoin,
    action,
    operation,
    amount,
    priceValue,
    controllers,
    onError,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["ordersByUser", account?.address] });
      queryClient.invalidateQueries({ queryKey: ["tradeHistory", account?.address] });
      queryClient.invalidateQueries({ queryKey: ["quests", account?.username] });
      setValue("price", getPrice(1, pairId.baseDenom).toFixed(4));
    },
  });

  const changeSizeCoin = useCallback((denom: string) => {
    setSizeCoinDenom(denom);
    setValue("size", "");
  }, []);

  const rangeValue = useMemo(() => {
    if (maxSizeAmount === 0) return 0;
    return Math.min(100, (+size / maxSizeAmount) * 100);
  }, [maxSizeAmount, size]);

  return (
    <div className="w-full flex flex-col justify-between h-full gap-4 flex-1">
      <div className="w-full flex flex-col gap-4 px-4">
        <InfoRow
          label={m["dex.protrade.spot.availableToTrade"]()}
          value={`${formatNumber(availableCoin.amount, formatNumberOptions)} ${availableCoin.symbol}`}
        />
        {operation === "limit" ? (
          <Input
            placeholder="0"
            isDisabled={!isConnected || submission.isPending}
            label="Price"
            {...register("price", { mask: numberMask })}
            startText="right"
            endContent={quoteCoin.symbol}
          />
        ) : null}
        <InputSizeWithMax
          isDisabled={!isConnected || submission.isPending}
          maxSizeAmount={maxSizeAmount}
          availableAmount={availableCoin.amount}
          register={register}
          setValue={setValue}
          validationMessage={m["errors.validations.insufficientFunds"]()}
          startContent={
            <CoinSelector
              classNames={{ trigger: "text-ink-tertiary-500" }}
              onChange={changeSizeCoin}
              value={sizeCoin.denom}
              coins={[baseCoin, quoteCoin]}
            />
          }
        />
        <Range
          isDisabled={!isConnected || submission.isPending}
          minValue={0}
          maxValue={100}
          defaultValue={0}
          withInput
          inputEndContent="%"
          value={rangeValue}
          onChange={(v) => {
            const newValue = Math.min(100, v);
            const sizeVal = Decimal(maxSizeAmount).mul(Decimal(newValue).div(100));
            const length = sizeVal.toFixed().split(".")[1]?.length || 0;
            setValue("size", sizeVal.toFixed(length < 19 ? length : 18));
          }}
        />
      </div>
      <div className="flex flex-col gap-4 pb-4 lg:pb-6">
        <TradeSubmitButton
          action={action}
          label={`${m["dex.protrade.spot.triggerAction"]({ action })} ${baseCoin.symbol}`}
          isDisabled={Decimal(size).lte(0) || (operation === "limit" && Decimal(priceValue).lte(0))}
          isPending={submission.isPending}
          onSubmit={() => submission.mutateAsync()}
        />
        <div className="flex flex-col gap-1 px-4">
          <InfoRow
            label={m["dex.protrade.spot.orderValue"]()}
            value={getPrice(size, sizeCoin.denom, { format: true })}
          />
          <InfoRow
            label={m["dex.protrade.spot.orderSize"]()}
            value={`${formatNumber(amount.quote, formatNumberOptions)} ${quoteCoin.symbol}`}
          />
          <InfoRow
            label=""
            value={`${formatNumber(amount.base, formatNumberOptions)} ${baseCoin.symbol}`}
          />
          {operation === "market" ? (
            <InfoRow label={m["dex.protrade.spot.slippage"]()} value="-" />
          ) : null}
          <InfoRow
            label={m["dex.protrade.spot.fees"]()}
            value={`${Number(appConfig.takerFeeRate) * 100} % / ${Number(appConfig.makerFeeRate) * 100} %`}
          />
        </div>
      </div>
    </div>
  );
};

const PerpsTradeMenu: React.FC<TradeMenuProps> = ({ controllers }) => {
  const { isConnected } = useAccount();
  const { settings, toast } = useApp();
  const { formatNumberOptions } = settings;
  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions, refetchInterval: 10_000 });
  const onError = useErrorHandler({
    toast: toast.error,
    title: m["dex.protrade.orderFailed"](),
    fallbackMessage: m["errors.failureRequest"](),
  });

  const pairId = tradePairStore((s) => s.pairId);
  const action = tradeInfoStore((s) => s.action);
  const operation = tradeInfoStore((s) => s.operation);

  const { coins } = useConfig();
  const { baseCoin, quoteCoin } = useTradeCoins({ pairId, mode: "perps" });

  const [sizeCoinDenom, setSizeCoinDenom] = useState("usd");

  useEffect(() => {
    setSizeCoinDenom("usd");
  }, [pairId]);

  const isBaseSize = sizeCoinDenom === baseCoin.denom;
  const currentPrice = getPrice(1, pairId.baseDenom);

  const perpsPairId = useMemo(() => {
    const baseSymbol = coins.byDenom[pairId.baseDenom]?.symbol;
    const quoteSymbol = coins.byDenom[pairId.quoteDenom]?.symbol ?? "USD";
    return baseSymbol ? toPerpsPairId(baseSymbol, quoteSymbol) : "";
  }, [pairId, coins]);

  const { data: appConfig } = useAppConfig();
  const perpsPairParam = perpsPairId ? (appConfig as any)?.perpsPairs?.[perpsPairId] : null;

  const userState = perpsUserStateStore((s) => s.userState);

  const margin = useMemo(() => userState?.margin ?? "0", [userState]);

  const position = useMemo(() => {
    if (!userState?.positions?.[perpsPairId]) return null;
    return userState.positions[perpsPairId];
  }, [userState, perpsPairId]);

  const availableMargin = useMemo(() => {
    if (!userState) return 0;
    const marginNum = Number(margin);

    let totalPnl = 0;
    let existingIM = 0;

    const allPerpsPairs = (appConfig as any)?.perpsPairs;
    if (userState.positions && allPerpsPairs) {
      for (const [pid, pos] of Object.entries(userState.positions)) {
        const param = allPerpsPairs[pid];
        if (!param) continue;
        const size = Number(pos.size);
        const absSize = Math.abs(size);
        const imr = Number(param.initialMarginRatio);
        const entryPrice = Number(pos.entryPrice);
        const price = pid === perpsPairId ? currentPrice : entryPrice;

        totalPnl += size * (price - entryPrice);
        existingIM += absSize * price * imr;
      }
    }

    const equity = marginNum + totalPnl;
    const reserved = Number(userState.reservedMargin ?? "0");
    return Math.max(0, equity - existingIM - reserved);
  }, [margin, userState, perpsPairId, currentPrice, appConfig]);

  const maxLeverage = useMemo(() => {
    if (!perpsPairParam?.initialMarginRatio) return 100;
    const ratio = Number(perpsPairParam.initialMarginRatio);
    return ratio > 0 ? Math.floor(1 / ratio) : 100;
  }, [perpsPairParam]);

  const [tpslEnabled, setTpslEnabled] = useState(false);

  const { register, setValue, inputs } = controllers;
  const size = inputs.size?.value || "0";
  const priceValue = inputs.price?.value || "0";

  const changeSizeCoin = useCallback((denom: string) => {
    setSizeCoinDenom(denom);
    setValue("size", "");
  }, []);

  const maxSizeAmount = usePerpsMaxSize({ availableMargin, leverage: maxLeverage, currentPrice, isBaseSize });

  useEffect(() => {
    const currentSize = Number(size);
    if (currentSize > maxSizeAmount && maxSizeAmount > 0) {
      setValue("size", maxSizeAmount.toString());
    }
  }, [maxSizeAmount]);

  const orderValue = useMemo(() => {
    const s = Number(size);
    if (s <= 0) return "-";
    const notional = isBaseSize ? s * currentPrice : s;
    return `$${formatNumber(notional.toString(), formatNumberOptions)}`;
  }, [size, isBaseSize, currentPrice, formatNumberOptions]);

  const unrealizedPnl = useMemo(() => {
    if (!position) return "0";
    const currentPrice = getPrice(1, pairId.baseDenom);
    if (!currentPrice || currentPrice === 0) return "0";
    const pnl = Decimal(position.size).mul(Decimal(currentPrice).minus(position.entryPrice));
    return pnl.toFixed();
  }, [position, pairId, getPrice]);

  const accountEquity = useMemo(() => {
    return Decimal(margin).plus(unrealizedPnl).toFixed();
  }, [margin, unrealizedPnl]);

  const sizeValue = useMemo(() => {
    if (isBaseSize) return size;
    if (currentPrice <= 0) return "0";
    return Decimal(size).div(currentPrice).toFixed(6);
  }, [size, isBaseSize, currentPrice]);

  const queryClient = useQueryClient();
  const { account } = useAccount();

  const submission = usePerpsSubmission({
    perpsPairId,
    action,
    operation,
    sizeValue,
    priceValue,
    controllers,
    onError,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["perpsTradeHistory", account?.address] });
    },
  });

  const feesDisplay = useMemo(() => {
    const perpsParam = appConfig?.perpsParam;
    if (!perpsParam) return "-";
    const taker = Number(perpsParam.takerFeeRates.base) * 100;
    const maker = Number(perpsParam.makerFeeRates.base) * 100;
    return `${taker}% / ${maker}%`;
  }, [appConfig?.perpsParam]);

  const requiredMargin = useMemo(() => {
    const s = Number(size);
    if (s <= 0) return null;
    const notional = isBaseSize ? s * currentPrice : s;
    return notional / maxLeverage;
  }, [size, isBaseSize, currentPrice, maxLeverage]);

  const estLiquidationPrice = useMemo(() => {
    const s = Number(size);
    if (s <= 0 || maxLeverage <= 1) return null;
    const entryPrice =
      operation === "limit" && Number(priceValue) > 0 ? Number(priceValue) : currentPrice;
    if (entryPrice <= 0) return null;

    const mmr = Number(perpsPairParam?.maintenanceMarginRatio ?? 0);
    return action === "buy"
      ? (entryPrice * (1 - 1 / maxLeverage)) / (1 - mmr)
      : (entryPrice * (1 + 1 / maxLeverage)) / (1 + mmr);
  }, [size, maxLeverage, action, operation, priceValue, currentPrice, perpsPairParam]);

  const minSizeAmount = useMemo(() => {
    if (!perpsPairParam?.minOrderSize) return 0;
    const minNotional = Number(perpsPairParam.minOrderSize);
    if (minNotional <= 0 || currentPrice <= 0) return 0;
    return isBaseSize ? minNotional / currentPrice : minNotional;
  }, [perpsPairParam, isBaseSize, currentPrice]);

  const currentPositionSize = position?.size ?? "0";

  return (
    <div className="w-full flex flex-col justify-between h-full gap-4 flex-1">
      <div className="w-full flex flex-col gap-4 px-4">
        <div className="flex flex-col gap-2">
          <InfoRow
            label="Available to Trade"
            value={`${formatNumber(availableMargin.toFixed(2), formatNumberOptions)} USDC`}
          />
          <InfoRow
            label="Current Position"
            value={`${formatNumber(currentPositionSize, formatNumberOptions)} ${baseCoin.symbol}`}
          />
        </div>
        <InputSizeWithMax
          isDisabled={!isConnected || submission.isPending}
          maxSizeAmount={maxSizeAmount}
          availableAmount={maxSizeAmount.toString()}
          register={register}
          setValue={setValue}
          validationMessage="Exceeds available margin"
          label="Size"
          minSizeAmount={minSizeAmount}
          minSizeMessage={`Min order size: $${perpsPairParam?.minOrderSize ?? "0"}`}
          hideMaxControls
          startContent={
            <CoinSelector
              classNames={{ trigger: "text-ink-tertiary-500" }}
              onChange={changeSizeCoin}
              value={sizeCoinDenom}
              coins={[baseCoin, quoteCoin]}
            />
          }
        />
        {operation === "limit" ? (
          <Input
            placeholder="0"
            isDisabled={!isConnected || submission.isPending}
            label="Price"
            {...register("price", { mask: numberMask })}
            startText="right"
            endContent="USD"
          />
        ) : null}
        {isFeatureEnabled("stopLoss") ? (
          <>
            <Checkbox
              radius="md"
              size="sm"
              label="Take Profit/Stop Loss"
              checked={tpslEnabled}
              onChange={() => setTpslEnabled(!tpslEnabled)}
            />
            {tpslEnabled ? (
              <div className="grid grid-cols-2 gap-2">
                <Input
                  placeholder="0"
                  label="TP Price"
                  {...register("tpPrice", { mask: numberMask })}
                />
                <Input
                  placeholder="0"
                  label="Gain"
                  endContent="%"
                  {...register("tpPercent", { mask: numberMask })}
                />
                <Input
                  placeholder="0"
                  label="SL Price"
                  {...register("slPrice", { mask: numberMask })}
                />
                <Input
                  placeholder="0"
                  label="Loss"
                  endContent="%"
                  {...register("slPercent", { mask: numberMask })}
                />
              </div>
            ) : null}
          </>
        ) : null}
      </div>
      <div className="flex flex-col gap-4 pb-4 lg:pb-6">
        <TradeSubmitButton
          action={action}
          label={`${action === "buy" ? "Buy" : "Sell"} ${baseCoin.symbol}`}
          isDisabled={Decimal(size).lte(0) || (operation === "limit" && Decimal(priceValue).lte(0))}
          isPending={submission.isPending}
          onSubmit={() => submission.mutateAsync()}
        />
        <div className="flex flex-col gap-1 px-4">
          <InfoRow label="Order Value" value={orderValue} />
          {requiredMargin !== null ? (
            <InfoRow
              label="Required Margin"
              value={`$${formatNumber(requiredMargin.toString(), formatNumberOptions)}`}
            />
          ) : null}
          {estLiquidationPrice !== null ? (
            <InfoRow
              label="Est. Liq. Price"
              value={`$${formatNumber(estLiquidationPrice.toFixed(2), formatNumberOptions)}`}
            />
          ) : null}
          {operation === "market" ? <InfoRow label="Slippage" value="Max: 0.1%" /> : null}
          <InfoRow label="Fees" value={feesDisplay} />
        </div>
        <div className="flex flex-col gap-1 px-4 border-t border-outline-tertiary-rice pt-3">
          <InfoRow
            label="Account Equity"
            value={`$${formatNumber(accountEquity, formatNumberOptions)}`}
          />
          <InfoRow label="Max Leverage" value={`${maxLeverage}x`} />
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-ink-tertiary-500">Unrealized PnL</p>
            <p
              className={twMerge(
                "diatype-xs-medium",
                Number(unrealizedPnl) >= 0 ? "text-utility-success-600" : "text-utility-error-600",
              )}
            >
              ${formatNumber(unrealizedPnl, formatNumberOptions)}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

const Menu: React.FC<TradeMenuProps> = ({ controllers, className }) => {
  const { isLg } = useMediaQuery();
  const { setTradeBarVisibility, setSidebarVisibility } = useApp();

  const mode = tradePairStore((s) => s.mode);
  const action = tradeInfoStore((s) => s.action);
  const operation = tradeInfoStore((s) => s.operation);
  const setAction = tradeInfoStore((s) => s.setAction);
  const setOperation = tradeInfoStore((s) => s.setOperation);

  return (
    <div className={twMerge("w-full flex items-center flex-col gap-4 relative", className)}>
      <div className="w-full flex items-center justify-between px-4 gap-2">
        <Tabs
          layoutId={!isLg ? "tabs-market-limit-mobile" : "tabs-market-limit"}
          selectedTab={operation}
          keys={["market", "limit"]}
          fullWidth
          onTabChange={(tab) => setOperation(tab as "market" | "limit")}
          color="line-red"
          classNames={{ button: "exposure-xs-italic" }}
        />
      </div>
      <div className="w-full flex items-center justify-between px-4 gap-2">
        <IconButton
          variant="utility"
          size="md"
          type="button"
          className="lg:hidden"
          onClick={() => setTradeBarVisibility(false)}
        >
          <IconChevronDownFill className="h-4 w-4" />
        </IconButton>
        <Tabs
          layoutId={!isLg ? "tabs-sell-and-buy-mobile" : "tabs-sell-and-buy"}
          selectedTab={action}
          keys={["buy", "sell"]}
          fullWidth
          classNames={{ base: "h-[44px] lg:h-auto", button: "exposure-sm-italic" }}
          onTabChange={(tab) => setAction(tab as "sell" | "buy")}
          color={action === "sell" ? "red" : "light-green"}
        />
        <IconButton
          variant="utility"
          size="md"
          type="button"
          className="lg:hidden"
          onClick={() => [setTradeBarVisibility(false), setSidebarVisibility(true)]}
        >
          <IconUser className="h-6 w-6" />
        </IconButton>
      </div>
      {mode === "spot" ? <SpotTradeMenu controllers={controllers} /> : null}
      {mode === "perps" ? <PerpsTradeMenu controllers={controllers} /> : null}
    </div>
  );
};

const MenuMobile: React.FC<TradeMenuProps> = (props) => {
  const { isTradeBarVisible, setTradeBarVisibility } = useApp();

  return (
    <Sheet isOpen={isTradeBarVisible} onClose={() => setTradeBarVisibility(false)} rootId="root">
      <Sheet.Container className="!bg-surface-primary-rice !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <Menu className="overflow-y-auto h-full" {...props} />
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setTradeBarVisibility(false)} />
    </Sheet>
  );
};
