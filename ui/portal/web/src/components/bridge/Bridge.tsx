import {
  Button,
  CoinSelector,
  IconArrowDown,
  Input,
  ResizerContainer,
  Tabs,
} from "@left-curve/applets-kit";

import { createContext, useInputs } from "@left-curve/applets-kit";

import { useState, type PropsWithChildren } from "react";
import type React from "react";
import { NetworkSelector } from "./NetworkSelector";
import { DepositAddressBox } from "./DepositAddressBox";

const [BridgeProvider, useBridge] = createContext<{
  state: {
    action: "deposit" | "withdraw";
  };
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "BridgeContext",
});

type BridgeProps = {};

const BridgeContainer: React.FC<PropsWithChildren<BridgeProps>> = ({ children }) => {
  const controllers = useInputs();
  const [action, setAction] = useState<"deposit" | "withdraw">("deposit");

  return (
    <BridgeProvider value={{ state: { action }, controllers }}>
      <ResizerContainer
        layoutId="send-and-receive"
        className="max-w-[400px] flex flex-col gap-8 rounded-xl w-full"
      >
        <Tabs
          layoutId="tabs-send-and-receive"
          selectedTab={action}
          keys={["deposit", "withdraw"]}
          fullWidth
          onTabChange={() => setAction(action === "deposit" ? "withdraw" : "deposit")}
        />
        {children}
      </ResizerContainer>
    </BridgeProvider>
  );
};

const BridgeDeposit: React.FC = () => {
  const { state } = useBridge();
  const { action } = state;

  if (action !== "deposit") return null;
  return (
    <>
      <CoinSelector
        variant="boxed"
        classNames={{
          base: "w-full",
          trigger: "h-[56px]",
        }}
        coins={[
          {
            type: "native",
            name: "USDC",
            logoURI: "/images/coins/usdc.svg",
            symbol: "USDC",
            denom: "bridge/usdc",
            decimals: 6,
          },
          {
            type: "native",
            name: "Ether",
            logoURI: "/images/coins/eth.svg",
            symbol: "ETH",
            denom: "bridge/eth",
            decimals: 18,
          },
          {
            type: "native",
            name: "USDT",
            logoURI: "/images/coins/usdt.svg",
            symbol: "USDT",
            denom: "bridge/usdt",
            decimals: 6,
          },
        ]}
      />

      <NetworkSelector
        classNames={{
          trigger: "h-[56px]",
        }}
        onNetworkChange={(network) => console.log(network)}
        networks={[
          { name: "Bitcoin Network", id: "bitcoin", time: "10-60 mins" },
          { name: "Ethereum Network", id: "ethereum", time: "16 blocks | 5-30 mins" },
          { name: "Base Network", id: "base", time: "5-30 mins" },
          { name: "Arbitrum Network", id: "arbitrum", time: "5-30 mins" },
          { name: "Solana Network", id: "solana", time: "2-10 mins" },
        ]}
      />

      <DepositAddressBox />
    </>
  );
};
const BridgeWithdraw: React.FC = () => {
  const { state } = useBridge();
  const { action } = state;

  if (action !== "withdraw") return null;
  return (
    <>
      <CoinSelector
        variant="boxed"
        classNames={{
          base: "w-full",
          trigger: "h-[56px]",
        }}
        coins={[
          {
            type: "native",
            name: "USDC",
            logoURI: "/images/coins/usdc.svg",
            symbol: "USDC",
            denom: "bridge/usdc",
            decimals: 6,
          },
          {
            type: "native",
            name: "Ether",
            logoURI: "/images/coins/eth.svg",
            symbol: "ETH",
            denom: "bridge/eth",
            decimals: 18,
          },
          {
            type: "native",
            name: "USDT",
            logoURI: "/images/coins/usdt.svg",
            symbol: "USDT",
            denom: "bridge/usdt",
            decimals: 6,
          },
        ]}
      />
      <NetworkSelector
        classNames={{
          trigger: "h-[56px]",
        }}
        onNetworkChange={(network) => console.log(network)}
        networks={[
          { name: "Bitcoin Network", id: "bitcoin", time: "10-60 mins" },
          { name: "Ethereum Network", id: "ethereum", time: "16 blocks | 5-30 mins" },
          { name: "Base Network", id: "base", time: "5-30 mins" },
          { name: "Arbitrum Network", id: "arbitrum", time: "5-30 mins" },
          { name: "Solana Network", id: "solana", time: "2-10 mins" },
        ]}
      />
      <div className="flex items-center justify-center flex-col gap-4">
        <Input
          classNames={{
            inputWrapper: "flex-col h-auto",
            inputParent: "h-[34px] h3-medium",
            input: "!h3-medium",
          }}
          placeholder="0"
          label="TP Price"
          insideBottomComponent={
            <div className="w-full flex">
              <span className="diatype-sm-regular text-ink-tertiary-500">$0.00</span>
            </div>
          }
        />
        <button
          type="button"
          className="flex items-center justify-center border border-primitives-gray-light-300 rounded-full h-5 w-5 cursor-pointer mt-4"
        >
          <IconArrowDown className="h-3 w-3 text-primitives-gray-light-300" />
        </button>
        <Input label="Withdrawal Wallet Address" placeholder="Enter your Ethereum wallet address" />
      </div>
      <Button fullWidth>Withdraw</Button>
    </>
  );
};

export const Bridge = Object.assign(BridgeContainer, {
  Deposit: BridgeDeposit,
  Withdraw: BridgeWithdraw,
});
