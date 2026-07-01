// The perps order-sizing maths (side-dependent "Available to trade",
// slider max, reduce-only cap) is non-obvious because the on-chain
// margin check treats closing and opening portions asymmetrically.
// See `./max-size-math.md` for the formulas and worked examples.

import {
  useAccount,
  useAppConfig,
  usePrices,
  usePerpsMaxSize,
  usePerpsSubmission,
  perpsTradeSettingsStore,
  useAllPerpsPairStats,
  usePerpsPairStatsByPairId,
  usePerpsUserState,
  usePerpsUserStateExtended,
  computeLiquidationPrice,
  useVolume,
  useFeeRateOverride,
  useStorage,
  usePerpsLiquidityDepth,
} from "@left-curve/store";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import {
  Button,
  Checkbox,
  CoinSelector,
  FormattedNumber,
  IconButton,
  IconChevronDownFill,
  IconEdit,
  IconUser,
  Input,
  InputSizeWithMax,
  Modals,
  Select,
  Range,
  Tabs,
  Tooltip,
  IconToastInfo,
  numberMask,
  twMerge,
  useApp,
  type useInputs,
  useMediaQuery,
  usePortalTarget,
} from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";

import { Decimal, formatNumber, resolveRateSchedule, shallowEqual } from "@left-curve/utils";
import { FEE_VOLUME_LOOKBACK_SECONDS, PERPS_DEFAULT_SLIPPAGE } from "~/constants";
import type { PerpsTimeInForce } from "@left-curve/types";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { MarketPair } from "@left-curve/foundation/market-pair";
import { useGeoblock } from "~/components/foundation/hooks/useGeoblock";
import { computeOtherPairsUsedMargin } from "../helpers/math";
import { getTopOfBookMidPrice } from "../helpers/orderBook";
import { perpsTradeHistoryKeys } from "../helpers/perpsTradeHistoryKeys";
import { useTPSLPriceSync } from "../hooks/useTPSLPriceSync";
import { useProTrade } from "./ProTrade";

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

const MID_PRICE_DEPTH_LIMIT = 1;
const MID_PRICE_NOTIFY_INTERVAL_MS = 500;

const TradeSubmitButton: React.FC<{
  action: "buy" | "sell";
  label: string;
  isDisabled: boolean;
  isPending: boolean;
  isRestricted?: boolean;
  onSubmit: () => void;
}> = ({ action, label, isDisabled, isPending, isRestricted, onSubmit }) => {
  const { isConnected } = useAccount();
  const showModal = useApp((state) => state.showModal);

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
        isDisabled={isDisabled || isRestricted}
        isLoading={isPending}
        onClick={onSubmit}
      >
        {isRestricted ? m["geoblock.accessRestricted"]() : label}
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

const PerpsTradeMenu: React.FC<TradeMenuProps> = ({ controllers }) => {
  const { isConnected } = useAccount();
  const showModal = useApp((state) => state.showModal);
  const formatNumberOptions = useApp((state) => state.settings.formatNumberOptions);
  const isGeoblocked = useGeoblock();

  const { data: appConfig } = useAppConfig();

  const { pair, action, orderType: operation, accountAddress } = useProTrade();
  const pairId = pair.id;
  const base = pair.base;
  const { getPrice } = usePrices();
  const { account } = useAccount();

  const [volumeRefreshKey, setVolumeRefreshKey] = useState(0);
  const feeLookbackSince = useMemo(
    () => Math.floor(Date.now() / 1000) - FEE_VOLUME_LOOKBACK_SECONDS,
    [volumeRefreshKey],
  );
  const { volume: userVolume } = useVolume({
    userAddress: account?.address,
    since: feeLookbackSince,
    enabled: isConnected,
  });

  const { override: feeRateOverride } = useFeeRateOverride({ enabled: isConnected });

  const [sizeCoinDenom, setSizeCoinDenom] = useState("usd");
  const [maxSlippage] = useStorage<string>("perps-max-slippage", {
    initialValue: PERPS_DEFAULT_SLIPPAGE,
  });

  useEffect(() => {
    setSizeCoinDenom("usd");
  }, [pairId]);

  const isBaseSize = sizeCoinDenom === base.denom;

  const activePairStats = usePerpsPairStatsByPairId({ pairId });

  const currentPrice = useMemo(() => {
    const fromStats = Number(activePairStats?.currentPrice ?? 0);
    if (fromStats > 0) return fromStats;
    const oraclePrice = Number(getPrice(1, base.denom) ?? 0);
    return Number.isFinite(oraclePrice) ? oraclePrice : 0;
  }, [activePairStats?.currentPrice, getPrice, base.denom]);

  const params = appConfig.perpsPairs[pairId];
  const midPriceBucketSize = params.bucketSizes[0] ?? params.tickSize;
  const { liquidityDepth: midPriceDepth } = usePerpsLiquidityDepth(
    (s) => ({ liquidityDepth: s.liquidityDepth }),
    {
      pairId,
      bucketSize: midPriceBucketSize,
      limit: MID_PRICE_DEPTH_LIMIT,
      enabled: operation === "limit" && Boolean(midPriceBucketSize),
      notifyIntervalMs: MID_PRICE_NOTIFY_INTERVAL_MS,
    },
    shallowEqual,
  );
  const midPriceValue = useMemo(
    () =>
      getTopOfBookMidPrice(midPriceDepth, {
        snapDirection: action === "buy" ? "down" : "up",
        tickSize: params.tickSize,
      }),
    [action, midPriceDepth, params.tickSize],
  );

  const accountState = usePerpsUserState(
    (s) => ({
      positions: s.userState?.positions,
      margin: s.userState?.margin ?? "0",
      reservedMargin: s.userState?.reservedMargin ?? "0",
    }),
    {
      accountAddress,
      enabled: isConnected,
    },
    shallowEqual,
  );
  const hasOtherPairPositions = useMemo(
    () =>
      Object.keys(accountState.positions ?? {}).some((positionPairId) => positionPairId !== pairId),
    [accountState.positions, pairId],
  );
  const statsByPairId = useAllPerpsPairStats((s) => s.perpsPairStatsByPairId, {
    enabled: isConnected && hasOtherPairPositions,
  });
  const extendedState = usePerpsUserStateExtended(
    (s) => ({ positions: s.positions, equity: s.equity }),
    {
      accountAddress,
      enabled: isConnected,
    },
    shallowEqual,
  );
  const extendedPositions = extendedState.positions;
  const equity = extendedState.equity ?? "0";
  const reservedMargin = accountState.reservedMargin;
  const quote = useMemo(
    () => ({ ...pair.quote, amount: accountState.margin }),
    [pair.quote, accountState.margin],
  );

  const otherPairsUsedMargin = useMemo(() => {
    const positions = accountState.positions;
    if (!positions) return 0;

    return computeOtherPairsUsedMargin(positions, pairId, appConfig.perpsPairs, (pid) => {
      const statsPrice = Decimal(statsByPairId[pid]?.currentPrice ?? 0);
      if (statsPrice.gt(0)) return statsPrice;
      const otherPair = MarketPair.fromPairId(pid);
      return Decimal(getPrice(1, otherPair.base.denom) ?? 0);
    });
  }, [accountState.positions, pairId, appConfig.perpsPairs, statsByPairId, getPrice]);

  const position = useMemo(() => {
    if (!accountState.positions?.[pairId]) return null;
    return accountState.positions[pairId];
  }, [accountState.positions, pairId]);

  const maxLeverage = useMemo(() => {
    const ratio = Number(params.initialMarginRatio);
    return ratio > 0 ? Math.floor(1 / ratio) : 100;
  }, [params]);

  const storedLeverage = perpsTradeSettingsStore((s) => s.leverageByPair[pairId]);
  const selectedLeverage = useMemo(() => {
    if (!storedLeverage) return maxLeverage;
    return Math.min(Math.max(Math.round(storedLeverage), 1), maxLeverage);
  }, [storedLeverage, maxLeverage]);

  const takerFeeRate = useMemo(() => {
    if (feeRateOverride) return Number(feeRateOverride.takerFeeRate);
    const schedule = appConfig?.perpsParam?.takerFeeRates;
    if (!schedule) return 0;
    const rate = Number(resolveRateSchedule(schedule, userVolume ?? "0"));
    return Number.isFinite(rate) ? rate : 0;
  }, [appConfig?.perpsParam, userVolume, feeRateOverride]);

  const [tpslEnabled, setTpslEnabled] = useState(false);
  const [manualReduceOnly, setReduceOnly] = useState(false);
  const [timeInForce, setTimeInForce] = useState<PerpsTimeInForce>("GTC");

  const reduceOnly = isGeoblocked || manualReduceOnly;

  useEffect(() => setTimeInForce("GTC"), [operation]);

  const { register, setValue, inputs, errors } = controllers;
  const size = inputs.size?.value || "0";
  const priceInputValue = inputs.price?.value || "";
  const priceValue = priceInputValue || "0";
  const tpPrice = inputs.tpPrice?.value || "";
  const slPrice = inputs.slPrice?.value || "";
  const hasErrors = Object.keys(errors).length > 0;
  const autoLimitPriceRef = useRef<string | null>(null);

  useEffect(() => {
    if (operation !== "limit" || !midPriceValue) {
      autoLimitPriceRef.current = null;
      return;
    }

    const previousAutoPrice = autoLimitPriceRef.current;
    const shouldAutoFill =
      priceInputValue === "" ||
      Decimal(priceInputValue || 0).lte(0) ||
      priceInputValue === previousAutoPrice;

    if (!shouldAutoFill) return;

    autoLimitPriceRef.current = midPriceValue;
    if (priceInputValue !== midPriceValue) setValue("price", midPriceValue);
  }, [operation, midPriceValue, priceInputValue, setValue]);

  const selectMidPrice = useCallback(() => {
    if (!midPriceValue) return;
    autoLimitPriceRef.current = midPriceValue;
    setValue("price", midPriceValue);
  }, [midPriceValue, setValue]);
  const priceInputRegistration = register("price", { mask: numberMask });

  useEffect(() => {
    setTpslEnabled(false);
    setReduceOnly(false);
    setValue("tpPrice", "");
    setValue("tpPercent", "");
    setValue("slPrice", "");
    setValue("slPercent", "");
  }, [pairId]);

  const referencePrice = useMemo(() => {
    if (operation === "limit" && Number(priceValue) > 0) return Number(priceValue);
    return currentPrice;
  }, [operation, priceValue, currentPrice]);

  const { onTpPriceChange, onTpPercentChange, onSlPriceChange, onSlPercentChange } =
    useTPSLPriceSync({
      setValue,
      referencePrice,
      leverage: selectedLeverage,
      isBuyDirection: action === "buy",
      enabled: tpslEnabled,
    });

  const tpslError = useMemo(() => {
    if (!tpslEnabled) return null;
    const tp = Number(tpPrice);
    const sl = Number(slPrice);
    if (tp > 0 && referencePrice > 0) {
      if (action === "buy" && tp <= referencePrice) {
        return m["dex.protrade.perps.errors.tpAboveForLongs"]();
      }
      if (action === "sell" && tp >= referencePrice) {
        return m["dex.protrade.perps.errors.tpBelowForShorts"]();
      }
    }
    if (sl > 0 && referencePrice > 0) {
      if (action === "buy" && sl >= referencePrice) {
        return m["dex.protrade.perps.errors.slBelowForLongs"]();
      }
      if (action === "sell" && sl <= referencePrice) {
        return m["dex.protrade.perps.errors.slAboveForShorts"]();
      }
    }
    return null;
  }, [tpPrice, slPrice, action, referencePrice, tpslEnabled]);

  const currentPositionSize = position?.size ?? "0";

  const changeSizeCoin = useCallback((denom: string) => {
    setSizeCoinDenom(denom);
    setValue("size", "");
  }, []);

  const { availToTrade, maxSize } = usePerpsMaxSize({
    equity: Number(equity),
    reservedMargin: Number(reservedMargin),
    otherPairsUsedMargin,
    currentPositionSize: Number(currentPositionSize),
    action,
    leverage: selectedLeverage,
    currentPrice,
    takerFeeRate,
    reduceOnly,
    isBaseSize,
  });
  const maxSizeAmount = maxSize;

  useEffect(() => {
    const currentSize = Number(size);
    if (currentSize > maxSizeAmount) {
      setValue("size", maxSizeAmount.toString());
    }
  }, [maxSizeAmount]);

  const orderValue = useMemo(() => {
    const s = Decimal(size || 0);
    if (s.lte(0)) return "-";
    const notional = isBaseSize ? s.mul(currentPrice) : s;
    return `$${formatNumber(notional.toString(), formatNumberOptions)}`;
  }, [size, isBaseSize, currentPrice, formatNumberOptions]);

  const unrealizedPnl = useMemo(() => {
    if (!position || currentPrice <= 0) return "0";
    const pnl = Decimal(position.size).mul(Decimal(currentPrice).minus(position.entryPrice));
    return pnl.toFixed();
  }, [position, currentPrice]);

  const sizeValue = useMemo(() => {
    if (isBaseSize) return size;
    if (currentPrice <= 0) return "0";
    return Decimal(size).div(currentPrice).toFixed(6);
  }, [size, isBaseSize, currentPrice]);

  const queryClient = useQueryClient();

  const submission = usePerpsSubmission({
    pairId,
    action,
    operation,
    sizeValue,
    priceValue,
    maxSlippage,
    tpPrice: tpslEnabled && Number(tpPrice) > 0 ? tpPrice : undefined,
    slPrice: tpslEnabled && Number(slPrice) > 0 ? slPrice : undefined,
    reduceOnly,
    timeInForce: operation === "limit" ? timeInForce : undefined,
    controllers,
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: perpsTradeHistoryKeys.account(account?.address),
      });
      queryClient.invalidateQueries({ queryKey: ["perpsVolume", account?.address] });
      setVolumeRefreshKey((k) => k + 1);
    },
  });

  const feesDisplay = useMemo(() => {
    if (feeRateOverride) {
      const maker = Decimal(feeRateOverride.makerFeeRate).mul(100).toFixed();
      const taker = Decimal(feeRateOverride.takerFeeRate).mul(100).toFixed();
      return `${taker}% / ${maker}%`;
    }
    const perpsParam = appConfig?.perpsParam;
    if (!perpsParam) return "-";
    const vol = userVolume ?? "0";
    const taker = Decimal(resolveRateSchedule(perpsParam.takerFeeRates, vol)).mul(100).toFixed();
    const maker = Decimal(resolveRateSchedule(perpsParam.makerFeeRates, vol)).mul(100).toFixed();
    return `${taker}% / ${maker}%`;
  }, [appConfig?.perpsParam, userVolume, feeRateOverride]);

  const requiredMargin = useMemo(() => {
    const s = Decimal(size || 0);
    if (s.lte(0)) return null;
    const notional = isBaseSize ? s.mul(currentPrice) : s;
    return notional.div(selectedLeverage);
  }, [size, isBaseSize, currentPrice, selectedLeverage]);

  const estLiquidationPrice = useMemo(() => {
    const s = Number(size);
    if (s <= 0 || selectedLeverage <= 1) return null;
    const entryPrice =
      operation === "limit" && Number(priceValue) > 0 ? Number(priceValue) : currentPrice;
    if (entryPrice <= 0) return null;

    const baseSize = isBaseSize ? s : currentPrice > 0 ? s / currentPrice : 0;
    const newSize = action === "buy" ? baseSize : -baseSize;

    return computeLiquidationPrice({
      margin: Number(accountState.margin),
      size: newSize,
      entryPrice,
      mmr: Number(params.maintenanceMarginRatio ?? 0),
      targetPairId: pairId,
      extendedPositions,
      pairPrices: statsByPairId,
      pairParams: appConfig.perpsPairs,
    });
  }, [
    size,
    selectedLeverage,
    action,
    operation,
    priceValue,
    currentPrice,
    params,
    isBaseSize,
    accountState.margin,
    extendedPositions,
    statsByPairId,
    pairId,
    appConfig.perpsPairs,
  ]);

  const minSizeAmount = useMemo(() => {
    if (!params.minOrderSize) return 0;
    const minNotional = Number(params.minOrderSize);
    if (minNotional <= 0 || currentPrice <= 0) return 0;
    return isBaseSize ? minNotional / currentPrice : minNotional;
  }, [params, isBaseSize, currentPrice]);

  const sizeRangeValue = useMemo(() => {
    if (maxSizeAmount === 0) return 0;
    return Math.min(100, (Number(size) / maxSizeAmount) * 100);
  }, [maxSizeAmount, size]);

  const onSizeRangeChange = useCallback(
    (percent: number) => {
      const clamped = Math.min(100, Math.max(0, percent));
      const sizeVal = Decimal(maxSizeAmount).mul(Decimal(clamped).div(100));
      const decimals = sizeVal.toFixed().split(".")[1]?.length || 0;
      setValue("size", sizeVal.toFixed(decimals < 19 ? decimals : 18));
    },
    [maxSizeAmount, setValue],
  );

  return (
    <div className="w-full flex flex-col justify-between h-full gap-4 flex-1">
      <div className="w-full flex flex-col gap-4 px-4">
        <div className="flex flex-col gap-2">
          <InfoRow
            label={m["dex.protrade.perps.availableToTrade"]()}
            value={
              <>
                <FormattedNumber number={availToTrade.toString()} as="span" /> USD
              </>
            }
          />
          <InfoRow
            label={m["dex.protrade.perps.currentPosition"]()}
            value={
              <>
                <FormattedNumber number={currentPositionSize} as="span" /> {base.symbol}
              </>
            }
          />
        </div>
        <InputSizeWithMax
          isDisabled={!isConnected || submission.isPending}
          maxSizeAmount={maxSizeAmount}
          availableAmount={maxSizeAmount.toString()}
          register={register}
          setValue={setValue}
          validationMessage={
            reduceOnly
              ? m["dex.protrade.perps.errors.exceedsClosable"]()
              : m["dex.protrade.perps.errors.exceedsMargin"]()
          }
          label={m["dex.protrade.perps.size"]()}
          minSizeAmount={minSizeAmount}
          minSizeMessage={m["dex.protrade.perps.errors.minOrderSize"]({
            minOrderSize: params.minOrderSize,
          })}
          hideMaxControls
          startContent={
            <CoinSelector
              classNames={{ trigger: "text-ink-tertiary-500" }}
              onChange={changeSizeCoin}
              value={sizeCoinDenom}
              coins={[base, quote]}
            />
          }
        />
        <Range
          isDisabled={!isConnected || submission.isPending || maxSizeAmount === 0}
          minValue={0}
          maxValue={100}
          step={1}
          defaultValue={0}
          withInput
          inputEndContent="%"
          showSteps={[
            { value: 0, label: "0%" },
            { value: 25, label: "25%" },
            { value: 50, label: "50%" },
            { value: 75, label: "75%" },
            { value: 100, label: "100%" },
          ]}
          value={sizeRangeValue}
          onChange={onSizeRangeChange}
        />
        {operation === "limit" ? (
          <Input
            placeholder="0"
            isDisabled={submission.isPending}
            label={m["dex.protrade.perps.priceWithQuote"]()}
            {...priceInputRegistration}
            onChange={(event) => {
              autoLimitPriceRef.current = null;
              priceInputRegistration.onChange(event);
            }}
            startText="right"
            endContent={
              <Button
                type="button"
                variant="secondary"
                size="xs"
                radius="sm"
                className="h-[25px] px-2"
                isDisabled={!midPriceValue || submission.isPending}
                onClick={selectMidPrice}
              >
                {m["dex.protrade.perps.midPrice"]()}
              </Button>
            }
          />
        ) : null}
        {operation === "limit" ? (
          <div className="flex items-center justify-between">
            <span className="diatype-sm-regular text-ink-tertiary-500">
              {m["dex.protrade.perps.timeInForce"]()}
            </span>
            <Select
              value={timeInForce}
              onChange={(v) => setTimeInForce(v as PerpsTimeInForce)}
              variant="plain"
              classNames={{ listboxWrapper: "right-0" }}
            >
              <Select.Item value="GTC">GTC</Select.Item>
              <Select.Item value="IOC">IOC</Select.Item>
              <Select.Item value="POST">Post Only</Select.Item>
            </Select>
          </div>
        ) : null}
        <Checkbox
          radius="md"
          size="sm"
          isDisabled={!isConnected || submission.isPending || isGeoblocked}
          label={m["dex.protrade.perps.reduceOnly"]()}
          checked={reduceOnly}
          onChange={() => setReduceOnly((prev) => !prev)}
        />
        {reduceOnly && maxSizeAmount === 0 && !isGeoblocked ? (
          <p className="diatype-xs-regular text-utility-warning-600">
            {m["dex.protrade.perps.errors.reduceOnlyNoPosition"]()}
          </p>
        ) : null}
        <Checkbox
          radius="md"
          size="sm"
          isDisabled={!isConnected || submission.isPending}
          label={m["dex.protrade.perps.tpsl"]()}
          checked={tpslEnabled}
          onChange={() => setTpslEnabled((prev) => !prev)}
        />
        {tpslEnabled ? (
          <div className="flex flex-col gap-2">
            <div className="grid grid-cols-2 gap-2">
              <Input
                placeholder="0"
                label={m["dex.protrade.perps.tpPrice"]()}
                {...register("tpPrice", { mask: numberMask })}
                onChange={(e) => onTpPriceChange(typeof e === "string" ? e : e.target.value)}
              />
              <Input
                placeholder="0"
                label={m["dex.protrade.perps.gain"]()}
                endContent="%"
                {...register("tpPercent", { mask: numberMask })}
                onChange={(e) => onTpPercentChange(typeof e === "string" ? e : e.target.value)}
              />
              <Input
                placeholder="0"
                label={m["dex.protrade.perps.slPrice"]()}
                {...register("slPrice", { mask: numberMask })}
                onChange={(e) => onSlPriceChange(typeof e === "string" ? e : e.target.value)}
              />
              <Input
                placeholder="0"
                label={m["dex.protrade.perps.loss"]()}
                endContent="%"
                {...register("slPercent", { mask: numberMask })}
                onChange={(e) => onSlPercentChange(typeof e === "string" ? e : e.target.value)}
              />
            </div>
            {operation === "market" ? (
              <p className="diatype-xs-regular text-ink-tertiary-500">
                {m["dex.protrade.perps.tpslMarketSlippageNote"]()}
              </p>
            ) : null}
            {tpslError ? (
              <p className="diatype-xs-regular text-utility-error-600">{tpslError}</p>
            ) : null}
          </div>
        ) : null}
      </div>
      <div className="flex flex-col gap-4 pb-4 lg:pb-6">
        <TradeSubmitButton
          action={action}
          label={`${m["dex.protrade.perps.triggerAction"]({ action })} ${base.symbol}`}
          isDisabled={
            Decimal(size).lte(0) ||
            (operation === "limit" && Decimal(priceValue).lte(0)) ||
            hasErrors ||
            tpslError !== null ||
            (reduceOnly && maxSizeAmount === 0)
          }
          isPending={submission.isPending}
          isRestricted={isGeoblocked && maxSizeAmount === 0}
          onSubmit={() => submission.mutateAsync()}
        />
        <div className="flex flex-col gap-1 px-4">
          <InfoRow label={m["dex.protrade.perps.orderValue"]()} value={orderValue} />
          {requiredMargin !== null ? (
            <InfoRow
              label={m["dex.protrade.perps.requiredMargin"]()}
              value={
                <FormattedNumber
                  number={requiredMargin.toString()}
                  formatOptions={{ currency: "USD" }}
                  as="span"
                />
              }
            />
          ) : null}
          {estLiquidationPrice !== null ? (
            <InfoRow
              label={m["dex.protrade.perps.estLiqPrice"]()}
              value={
                <FormattedNumber
                  number={estLiquidationPrice.toString()}
                  formatOptions={{ currency: "USD" }}
                  as="span"
                />
              }
            />
          ) : null}
          {operation === "market" ? (
            <div className="flex items-center justify-between gap-2">
              <Tooltip title={m["dex.protrade.perps.slippageTooltip"]()}>
                <p className="diatype-xs-regular text-ink-tertiary-500 cursor-help underline decoration-dashed underline-offset-[4px] decoration-current">
                  {m["dex.protrade.perps.slippage"]()}
                </p>
              </Tooltip>
              <div className="flex items-center gap-1">
                <p className="diatype-xs-medium text-ink-secondary-700">
                  {m["dex.protrade.perps.slippageDisplay"]({
                    max: Decimal(maxSlippage).mul(100).toFixed(2),
                  })}
                </p>
                <IconEdit
                  className="w-4 h-4 text-ink-tertiary-500 hover:text-ink-secondary-700 cursor-pointer"
                  onClick={() => showModal(Modals.AdjustSlippage)}
                />
              </div>
            </div>
          ) : null}
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-1">
              <p className="diatype-xs-regular text-ink-tertiary-500">
                {m["dex.protrade.perps.fees"]()}
              </p>
              <Tooltip
                trigger="click"
                title={
                  <div className="flex flex-col gap-1">
                    <p>
                      {m["dex.protrade.perps.feesTooltipTaker"]({
                        rate: `${Decimal(
                          feeRateOverride
                            ? feeRateOverride.takerFeeRate
                            : resolveRateSchedule(
                                appConfig.perpsParam.takerFeeRates,
                                userVolume ?? "0",
                              ),
                        )
                          .mul(100)
                          .toFixed()}%`,
                      })}
                    </p>
                    <p>
                      {m["dex.protrade.perps.feesTooltipMaker"]({
                        rate: `${Decimal(
                          feeRateOverride
                            ? feeRateOverride.makerFeeRate
                            : resolveRateSchedule(
                                appConfig.perpsParam.makerFeeRates,
                                userVolume ?? "0",
                              ),
                        )
                          .mul(100)
                          .toFixed()}%`,
                      })}
                    </p>
                    <button
                      type="button"
                      className="text-status-success diatype-xs-bold mt-1 text-left"
                      onClick={() => showModal(Modals.FeeTiers)}
                    >
                      {m["dex.protrade.perps.feesLearnMore"]()}
                    </button>
                  </div>
                }
              >
                <IconToastInfo className="w-4 h-4 text-ink-tertiary-500 cursor-help" />
              </Tooltip>
            </div>
            <p className="diatype-xs-medium text-ink-secondary-700">{feesDisplay}</p>
          </div>
        </div>
        <div className="flex flex-col gap-1 px-4 border-t border-outline-tertiary-rice pt-3">
          <InfoRow
            label={m["dex.protrade.perps.accountEquity"]()}
            value={
              <FormattedNumber number={equity} formatOptions={{ currency: "USD" }} as="span" />
            }
          />
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-ink-tertiary-500">
              {m["dex.protrade.perps.unrealizedPnl"]()}
            </p>
            <p
              className={twMerge(
                "diatype-xs-medium",
                Number(unrealizedPnl) >= 0 ? "text-utility-success-600" : "text-utility-error-600",
              )}
            >
              <FormattedNumber
                number={unrealizedPnl}
                formatOptions={{ currency: "USD" }}
                as="span"
              />
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};

const PerpsTopPills: React.FC = () => {
  const showModal = useApp((state) => state.showModal);
  const { data: appConfig } = useAppConfig();
  const { pair } = useProTrade();
  const pairId = pair.id;
  const base = pair.base;

  const params = appConfig.perpsPairs[pairId];

  const maxLeverage = useMemo(() => {
    const ratio = Number(params.initialMarginRatio);
    return ratio > 0 ? Math.floor(1 / ratio) : 100;
  }, [params]);

  const storedLeverage = perpsTradeSettingsStore((s) => s.leverageByPair[pairId]);
  const selectedLeverage = useMemo(() => {
    if (!storedLeverage) return maxLeverage;
    return Math.min(Math.max(Math.round(storedLeverage), 1), maxLeverage);
  }, [storedLeverage, maxLeverage]);

  const marginMode = perpsTradeSettingsStore((s) => s.marginModeByPair[pairId]) ?? "cross";

  const ticker = pair.ticker;

  const openMarginModeModal = useCallback(() => {
    showModal(Modals.PerpsMarginMode, { pairId, ticker });
  }, [showModal, pairId, ticker]);

  const openAdjustLeverageModal = useCallback(() => {
    showModal(Modals.PerpsAdjustLeverage, {
      pairId,
      baseSymbol: base.symbol,
      maxLeverage,
    });
  }, [showModal, pairId, base.symbol, maxLeverage]);

  return (
    <div className="w-full flex items-center gap-2 px-4">
      <Button
        variant="secondary"
        size="sm"
        radius="sm"
        fullWidth
        onClick={openMarginModeModal}
        className="capitalize"
      >
        {marginMode}
      </Button>
      <Button variant="secondary" size="sm" radius="sm" fullWidth onClick={openAdjustLeverageModal}>
        {selectedLeverage}x
      </Button>
    </div>
  );
};

const Menu: React.FC<TradeMenuProps> = ({ controllers, className }) => {
  const { isLg } = useMediaQuery();
  const setTradeBarVisibility = useApp((state) => state.setTradeBarVisibility);
  const setSidebarVisibility = useApp((state) => state.setSidebarVisibility);

  const { action, orderType: operation, onChangeAction, onChangeOrderType } = useProTrade();

  return (
    <div className={twMerge("w-full flex items-center flex-col gap-4 relative", className)}>
      <PerpsTopPills />
      <div className="w-full flex items-center justify-between px-4 gap-2">
        <Tabs
          layoutId={!isLg ? "tabs-market-limit-mobile" : "tabs-market-limit"}
          selectedTab={operation}
          keys={["market", "limit"]}
          fullWidth
          onTabChange={(tab) => onChangeOrderType(tab as "market" | "limit")}
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
          onTabChange={(tab) => onChangeAction(tab as "sell" | "buy")}
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
      <PerpsTradeMenu controllers={controllers} />
    </div>
  );
};

const MenuMobile: React.FC<TradeMenuProps> = (props) => {
  const isTradeBarVisible = useApp((state) => state.isTradeBarVisible);
  const setTradeBarVisibility = useApp((state) => state.setTradeBarVisibility);
  const portalTarget = usePortalTarget("#root");

  if (!portalTarget) return null;

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
