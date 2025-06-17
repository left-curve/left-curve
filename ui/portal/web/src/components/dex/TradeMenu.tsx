import { Sheet } from "react-modal-sheet";
import { useApp } from "~/hooks/useApp";

import {
  Button,
  Checkbox,
  CoinSelector,
  IconButton,
  IconChevronDown,
  IconUser,
  Input,
  Range,
  Tabs,
  twMerge,
  useControlledState,
  type useInputs,
  useMediaQuery,
} from "@left-curve/applets-kit";

import { useState } from "react";

import { m } from "~/paraglide/messages";

import type { useProTrade } from "@left-curve/store";
import type React from "react";

export const TradeMenu: React.FC<TradeMenuProps> = (props) => {
  const { isLg } = useMediaQuery();
  return <>{isLg ? <Menu {...props} /> : <MenuMobile {...props} />}</>;
};

type TradeMenuProps = {
  className?: string;
  state: ReturnType<typeof useProTrade>;
  controllers: ReturnType<typeof useInputs>;
};

const SpotTradeMenu: React.FC<TradeMenuProps> = ({ state }) => {
  const { isLg } = useMediaQuery();
  const { operation, setOperation, action } = state;

  return (
    <div className="w-full flex flex-col justify-between h-full gap-4 flex-1">
      <div className="w-full flex flex-col gap-4 px-4">
        <Tabs
          layoutId={!isLg ? "tabs-market-limit-mobile" : "tabs-market-limit"}
          selectedTab={operation}
          keys={["market", "limit"]}
          fullWidth
          onTabChange={(tab) => setOperation(tab as "market" | "limit")}
          color="line-red"
          classNames={{ button: "exposure-xs-italic" }}
        />
        <div className="flex items-center justify-between gap-2">
          <p className="diatype-xs-regular text-gray-500">
            {m["dex.protrade.spot.availableToTrade"]()}
          </p>
          <p className="diatype-xs-medium text-gray-700">1.23 ETH</p>
        </div>
        <Input
          placeholder="0"
          label="Size"
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
                trigger: "!diatype-lg-medium text-gray-500 p-0",
                selectorIcon: "w-4 h-4",
              }}
              coins={[
                {
                  type: "native",
                  symbol: "ETH",
                  name: "Ethereum",
                  denom: "wei",
                  decimals: 18,
                  logoURI: "https://assets.coingecko.com/coins/images/279/large/ethereum.png",
                  coingeckoId: "ethereum",
                },
                {
                  type: "contract",
                  symbol: "USDC",
                  name: "USD Coin",
                  denom: "uusdc",
                  decimals: 6,
                  contractAddress: "juno1xyzcontractaddress1234567890",
                  logoURI: "https://assets.coingecko.com/coins/images/6319/large/USD_Coin_icon.png",
                  coingeckoId: "usd-coin",
                },
              ]}
            />
          }
        />
        <Range minValue={0} maxValue={100} defaultValue={50} withInput inputEndContent="%" />
      </div>
      <div className="flex flex-col gap-4 pb-4 lg:pb-6">
        <div className="px-4">
          <Button variant={action === "sell" ? "primary" : "tertiary"} fullWidth size="md">
            Enable Trading
          </Button>
        </div>
        <div className="flex flex-col gap-1 px-4">
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-gray-500">
              {m["dex.protrade.spot.orderValue"]()}
            </p>
            <p className="diatype-xs-medium text-gray-700">$12.345</p>
          </div>
          {operation === "market" ? (
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">
                {m["dex.protrade.spot.slippage"]()}
              </p>
              <p className="diatype-xs-medium text-status-success">Est: 0% / Max: 8.00%</p>
            </div>
          ) : null}
          <div className="flex items-center justify-between gap-2">
            <p className="diatype-xs-regular text-gray-500">{m["dex.protrade.spot.fees"]()}</p>
            <p className="diatype-xs-medium text-gray-700">0.035% / 0.0100%</p>
          </div>
        </div>
        {/*  <span className="w-full h-[1px] bg-gray-100" />
        <div className="px-4 flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <p className="diatype-xs-bold">Account Equity</p>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Spot</p>
              <p className="diatype-xs-medium text-gray-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Perps</p>
              <p className="diatype-xs-medium text-gray-700">$10.00</p>
            </div>
          </div>
          <div className="flex flex-col gap-2">
            <p className="diatype-xs-bold">Perp Overview</p>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Balance</p>
              <p className="diatype-xs-medium text-gray-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Unrealized PNL</p>
              <p className="diatype-xs-medium text-gray-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Cross Margin Ratio</p>
              <p className="diatype-xs-medium text-gray-700">0.00%</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Maintenance</p>
              <p className="diatype-xs-medium text-gray-700">$10.00</p>
            </div>
            <div className="flex items-center justify-between gap-2">
              <p className="diatype-xs-regular text-gray-500">Cross Account Leverage</p>
              <p className="diatype-xs-medium text-gray-700">0.00x</p>
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
        <p className="diatype-xs-medium text-gray-500">Current Position</p>
        <p className="diatype-xs-bold text-gray-700">123.00 ETH</p>
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
          <div className="flex items-center justify-between gap-2 w-full h-[22px] text-gray-500 diatype-sm-regular pl-4">
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
  const { action, setAction, type } = state;

  return (
    <div className={twMerge("w-full flex items-center flex-col gap-4 relative", className)}>
      <div className="w-full flex items-center justify-between px-4 gap-2">
        <IconButton
          variant="utility"
          size="lg"
          type="button"
          className="lg:hidden"
          onClick={() => setTradeBarVisibility(false)}
        >
          <IconChevronDown className="h-6 w-6" />
        </IconButton>
        <Tabs
          layoutId={!isLg ? "tabs-sell-and-buy-mobile" : "tabs-sell-and-buy"}
          selectedTab={action}
          keys={["buy", "sell"]}
          fullWidth
          classNames={{ base: "h-[44px] lg:h-auto", button: "exposure-sm-italic" }}
          onTabChange={(tab) => setAction(tab as "sell" | "buy")}
          color={action === "sell" ? "red" : "green"}
        />
        <IconButton
          variant="utility"
          size="lg"
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
      <Sheet.Container className="!bg-white-100 !rounded-t-2xl !shadow-none">
        <Sheet.Header />
        <Sheet.Content>
          <Menu className="overflow-y-auto h-full" {...props} />
        </Sheet.Content>
      </Sheet.Container>
      <Sheet.Backdrop onTap={() => setTradeBarVisibility(false)} />
    </Sheet>
  );
};
