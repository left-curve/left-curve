import { useAccount, useAppConfig, usePrices } from "@left-curve/store";
import { useNavigate } from "@tanstack/react-router";
import { useMemo } from "react";
import { useApp } from "~/hooks/useApp";

import {
  Button,
  Checkbox,
  CoinSelector,
  IconButton,
  IconChevronDownFill,
  IconUser,
  Input,
  Range,
  Tabs,
  numberMask,
  twMerge,
  type useInputs,
  useMediaQuery,
} from "@left-curve/applets-kit";
import { Sheet } from "react-modal-sheet";

import { Decimal, formatNumber } from "@left-curve/dango/utils";
import { m } from "~/paraglide/messages";

import type { useProTradeState } from "@left-curve/store";
import type React from "react";

export const TradeMenu: React.FC<TradeMenuProps> = (props) => {
  const { isLg } = useMediaQuery();
  return <>{isLg ? <Menu {...props} /> : <MenuMobile {...props} />}</>;
};

type TradeMenuProps = {
  className?: string;
  state: ReturnType<typeof useProTradeState>;
  controllers: ReturnType<typeof useInputs>;
};

const SpotTradeMenu: React.FC<TradeMenuProps> = ({ state, controllers }) => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { isConnected } = useAccount();
  const { data: appConfig } = useAppConfig();

  const { getPrice } = usePrices({ defaultFormatOptions: formatNumberOptions });

  const {
    operation,
    action,
    changeSizeCoin,
    sizeCoin,
    availableCoin,
    orderAmount,
    maxSizeAmount,
    baseCoin,
    quoteCoin,
    submission,
  } = state;
  const { register, setValue, inputs } = controllers;

  const navigate = useNavigate();

  const amount = inputs.size?.value || "0";

  const priceAmount = inputs.price?.value || "0";

  const rangeValue = useMemo(() => {
    if (maxSizeAmount === 0) return 0;
    return Math.min(100, (+amount / maxSizeAmount) * 100);
  }, [maxSizeAmount, amount]);

  return (
    <div className="w-full flex flex-col justify-between h-full gap-4 flex-1">
      <div className="w-full flex flex-col gap-4 px-4">
        <div className="flex items-center justify-between gap-2">
          <p className="diatype-xs-regular text-tertiary-500">
            {m["dex.protrade.spot.availableToTrade"]()}
          </p>
          <p className="diatype-xs-medium text-secondary-700">
            {formatNumber(availableCoin.amount, {
              ...formatNumberOptions,
              maxSignificantDigits: 10,
            })}{" "}
            {availableCoin.symbol}
          </p>
        </div>
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
        <Input
          placeholder="0"
          isDisabled={!isConnected || submission.isPending}
          label="Size"
          {...register("size", {
            strategy: "onChange",
            mask: numberMask,
            validate: (v) => {
              if (Number(v) > Number(maxSizeAmount))
                return m["errors.validations.insufficientFunds"]();
              return true;
            },
          })}
          classNames={{
            base: "z-20",
            inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
            inputParent: "h-[34px] h3-medium",
            input: "!h3-medium",
          }}
          startText="right"
          startContent={
            <CoinSelector
              classNames={{
                trigger: "text-tertiary-500",
              }}
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
            const size = Decimal(maxSizeAmount).mul(Decimal(newValue).div(100));
            const length = size.toFixed().split(".")[1]?.length || 0;
            setValue("size", size.toFixed(length < 19 ? length : 18));
          }}
        />
      </div>
      <div className="flex flex-col gap-4 pb-4 lg:pb-6">
        <div className="px-4">
          {isConnected ? (
            <Button
              variant={action === "sell" ? "primary" : "tertiary"}
              fullWidth
              size="md"
              isDisabled={
                Decimal(amount).lte(0) || (operation === "limit" && Decimal(priceAmount).lte(0))
              }
              isLoading={submission.isPending}
              onClick={() => submission.mutateAsync()}
            >
              {m["dex.protrade.spot.triggerAction"]({ action })}
            </Button>
          ) : (
            <Button
              variant={action === "sell" ? "primary" : "tertiary"}
              fullWidth
              size="md"
              onClick={() => navigate({ to: "/signin" })}
            >
              {m["dex.protrade.spot.enableTrading"]()}
            </Button>
          )}
        </div>
        <div className="flex flex-col gap-1 px-4">
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-tertiary-500">
              {m["dex.protrade.spot.orderValue"]()}
            </p>
            <p className="diatype-xs-medium text-secondary-700">
              {getPrice(amount, sizeCoin.denom, { format: true })}
            </p>
          </div>
          <div className="flex items-center justify-between gap-2">
            <p className="flex gap-1 diatype-xs-regular text-tertiary-500">
              <span>{m["dex.protrade.spot.orderSize"]()}</span>
            </p>
            <p className="diatype-xs-medium text-secondary-700">
              {formatNumber(orderAmount.quoteAmount, { ...formatNumberOptions })} {quoteCoin.symbol}
            </p>
          </div>
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-tertiary-500" />
            <p className="diatype-xs-medium text-secondary-700">
              {formatNumber(orderAmount.baseAmount, { ...formatNumberOptions })} {baseCoin.symbol}
            </p>
          </div>
          {operation === "market" ? (
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">
                {m["dex.protrade.spot.slippage"]()}
              </p>
              <p className="diatype-xs-medium">-</p>
            </div>
          ) : null}
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-tertiary-500">{m["dex.protrade.spot.fees"]()}</p>
            <p className="diatype-xs-medium text-secondary-700">
              {Number(appConfig?.takerFeeRate) * 100} % / {Number(appConfig?.makerFeeRate) * 100} %
            </p>
          </div>
        </div>
        {/*  <span className="w-full h-[1px] bg-secondary-gray" />
        <div className="px-4 flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <p className="diatype-xs-bold">Account Equity</p>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Spot</p>
              <p className="diatype-xs-medium text-secondary-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Perps</p>
              <p className="diatype-xs-medium text-secondary-700">$10.00</p>
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <p className="diatype-xs-bold">Perp Overview</p>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Balance</p>
              <p className="diatype-xs-medium text-secondary-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Unrealized PNL</p>
              <p className="diatype-xs-medium text-secondary-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Cross Margin Ratio</p>
              <p className="diatype-xs-medium text-secondary-700">0.00%</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Maintenance</p>
              <p className="diatype-xs-medium text-secondary-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-tertiary-500">Cross Account Leverage</p>
              <p className="diatype-xs-medium text-secondary-700">0.00x</p>
            </div>
          </div>
        </div> */}
      </div>
    </div>
  );
};

const PerpsTradeMenu: React.FC<TradeMenuProps> = ({ state }) => {
  const { isLg } = useMediaQuery();
  const { operation, setOperation, action } = state;

  return (
    <div className="w-full flex flex-col gap-4 p-4">
      <Tabs
        layoutId={!isLg ? "tabs-market-limit-mobile" : "tabs-market-limit"}
        selectedTab={operation}
        keys={["market", "limit"]}
        fullWidth
        onTabChange={(tab) => setOperation(tab as "market" | "limit")}
        color="line-red"
      />
      <div className="flex items-center justify-between gap-2">
        <p className="diatype-xs-medium text-tertiary-500">Current Position</p>
        <p className="diatype-xs-bold text-secondary-700">123.00 ETH</p>
      </div>
      <Input
        placeholder="0"
        label="Size"
        classNames={{
          base: "z-20",
          inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px]",
          inputParent: "h-[34px] h3-bold",
          input: "!h3-bold",
        }}
        startText="right"
        startContent={
          <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
            <div className="flex gap-2 items-center font-semibold">
              <img
                src="https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg"
                alt="usdc"
                className="w-8 h-8"
              />
              <p>USDC</p>
            </div>
          </div>
        }
        insideBottomComponent={
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-tertiary-500 diatype-sm-regular pl-4">
            <div className="flex items-center gap-2">
              <p>12.23</p>
              <Button
                type="button"
                variant="secondary"
                size="xs"
                className="bg-red-bean-50 text-red-bean-500 hover:bg-red-bean-100 focus:[box-shadow:0px_0px_0px_3px_#F575893D] py-[2px] px-[6px]"
              >
                {m["common.max"]()}
              </Button>
            </div>
          </div>
        }
      />
      <Input placeholder="0" label="Price" endContent={<p>USDC</p>} />
      <Range
        minValue={1.1}
        maxValue={100}
        defaultValue={20}
        label="Leverage"
        inputEndContent="x"
        withInput
        showSteps={[
          { value: 1.1, label: "1.1x" },
          { value: 50, label: "50x" },
          { value: 100, label: "100x" },
        ]}
      />
      <Checkbox radius="md" size="sm" label="Take Profit/Stop Loss" />
      <div className="grid grid-cols-2 gap-2">
        <Input placeholder="0" label="TP Price" />
        <Input placeholder="0" label="TP Price" endContent="%" />
        <Input placeholder="0" label="SL Price" />
        <Input placeholder="0" label="Loss" endContent="%" />
      </div>
      <Button variant={action === "sell" ? "primary" : "tertiary"} fullWidth>
        Enable Trading
      </Button>
    </div>
  );
};

const Menu: React.FC<TradeMenuProps> = ({ state, controllers, className }) => {
  const { isLg } = useMediaQuery();
  const { setTradeBarVisibility, setSidebarVisibility } = useApp();
  const { action, changeAction, type, submission, operation, setOperation } = state;

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
          isDisabled={submission.isPending}
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
          onTabChange={(tab) => changeAction(tab as "sell" | "buy")}
          color={action === "sell" ? "red" : "green"}
          isDisabled={submission.isPending}
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
      {type === "spot" ? <SpotTradeMenu state={state} controllers={controllers} /> : null}
      {type === "perps" ? <PerpsTradeMenu state={state} controllers={controllers} /> : null}
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
