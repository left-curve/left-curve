import { useAccount, useBridgeState, usePrices } from "@left-curve/store";
import {
  AssetInputWithRange,
  ConnectWalletWithModal,
  createContext,
  DepositAddressBox,
  ethAddressMask,
  IconDisconnect,
  TruncateText,
  useApp,
  useInputs,
} from "@left-curve/applets-kit";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { capitalize } from "@left-curve/dango/utils";

import {
  Button,
  CoinSelector,
  IconArrowDown,
  Input,
  ResizerContainer,
  NetworkSelector,
  Tabs,
} from "@left-curve/applets-kit";

import type React from "react";
import type { PropsWithChildren } from "react";

const masks = {
  ethereum: ethAddressMask,
  base: ethAddressMask,
  arbitrum: ethAddressMask,
};

const networks = [
  { name: "Ethereum Network", id: "ethereum", time: "16 blocks | 5-30 mins" },
  { name: "Base Network", id: "base", time: "5-30 mins" },
  { name: "Arbitrum Network", id: "arbitrum", time: "5-30 mins" },
  /*       { name: "Bitcoin Network", id: "bitcoin", time: "10-60 mins" },
          { name: "Solana Network", id: "solana", time: "2-10 mins" }, */
];

const [BridgeProvider, useBridge] = createContext<{
  state: ReturnType<typeof useBridgeState>;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "BridgeContext",
});

type BridgeProps = {
  action: "deposit" | "withdraw";
  changeAction: (action: "deposit" | "withdraw") => void;
};

const BridgeContainer: React.FC<PropsWithChildren<BridgeProps>> = ({
  children,
  action,
  changeAction,
}) => {
  const controllers = useInputs();
  const state = useBridgeState({ action, controllers });

  return (
    <BridgeProvider value={{ state, controllers }}>
      <ResizerContainer
        layoutId="send-and-receive"
        className="max-w-[400px] flex flex-col gap-8 rounded-xl w-full"
      >
        <Tabs
          layoutId="tabs-send-and-receive"
          selectedTab={action}
          keys={["deposit", "withdraw"]}
          fullWidth
          onTabChange={() => changeAction(action === "deposit" ? "withdraw" : "deposit")}
        />
        {children}
      </ResizerContainer>
    </BridgeProvider>
  );
};

const BridgeDeposit: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { state, controllers } = useBridge();
  const { isConnected } = useAccount();
  const { getPrice } = usePrices();

  const {
    action,
    coins,
    coin,
    changeCoin,
    network,
    setNetwork,
    connector,
    setConnectorId,
    deposit,
    depositAddress,
    walletAddress,
    getAmount,
  } = state;

  if (action !== "deposit") return null;

  return (
    <>
      <CoinSelector
        isDisabled={!isConnected}
        label={m["bridge.selectCoin"]()}
        placeholder={m["bridge.selectCoin"]()}
        variant="boxed"
        classNames={{ base: "w-full", trigger: "h-[56px]", listboxWrapper: "top-[4rem]" }}
        value={coin ? coin.denom : undefined}
        onChange={(denom) => changeCoin(denom)}
        coins={coins}
        withName
        withPrice
      />

      <NetworkSelector
        isDisabled={!coin}
        value={network ? network : undefined}
        classNames={{ trigger: "h-[56px]" }}
        label={m["bridge.selectNetwork"]()}
        placeholder={m["bridge.selectNetwork"]()}
        onNetworkChange={({ id }) => setNetwork(id)}
        networks={networks}
      />

      {depositAddress && <DepositAddressBox address={depositAddress} network={network as string} />}

      {coin && network && !connector && (
        <ConnectWalletWithModal
          fullWidth
          isDisabled={!coin || !network}
          onWalletSelected={(id) => setConnectorId(id)}
        />
      )}

      {coin && connector && (
        <div className="flex flex-col items-center justify-center gap-6">
          <AssetInputWithRange
            name="amount"
            asset={coin}
            controllers={controllers}
            showRange
            label={
              <div className="flex justify-between w-full items-center">
                <p className="exposure-sm-italic text-ink-secondary-700">
                  {m["bridge.youDeposit"]()}
                </p>

                <div className="flex gap-2 items-center">
                  <img src={connector.icon} alt={connector.name} className="w-4 h-4 inline-block" />
                  <TruncateText
                    start={4}
                    end={4}
                    text={walletAddress.data || ""}
                    className="diatype-sm-medium text-ink-tertiary-500"
                  />
                  <IconDisconnect
                    className="w-4 h-4 inline-block text-ink-tertiary-500 hover:cursor-pointer hover:text-ink-primary-900"
                    onClick={() => setConnectorId(null)}
                  />
                </div>
              </div>
            }
          />
          <div className="flex items-center justify-center border border-primitives-gray-light-300 rounded-full h-5 w-5 cursor-pointer">
            <IconArrowDown className="h-3 w-3 text-primitives-gray-light-300" />
          </div>
          <Input
            placeholder="0"
            label={m["bridge.youGet"]()}
            value={getAmount}
            classNames={{
              base: "z-20",
              inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px] hover:bg-surface-secondary-rice",
              inputParent: "h-[34px] h3-bold",
              input: "!h3-bold",
            }}
            startText="right"
            startContent={
              <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
                <div className="flex gap-2 items-center font-semibold">
                  <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
                  <p>{coin.symbol}</p>
                </div>
              </div>
            }
            insideBottomComponent={
              <div className="flex justify-end w-full h-[22px] text-ink-tertiary-500 diatype-sm-regular">
                <p>
                  {getPrice(getAmount, coin.denom, {
                    format: true,
                    formatOptions: { ...formatNumberOptions, maximumTotalDigits: 6 },
                  })}
                </p>
              </div>
            }
          />

          <Button
            fullWidth
            onClick={() => deposit.mutate()}
            isLoading={deposit.isPending}
            className="mt-4"
          >
            {m["bridge.deposit"]()}
          </Button>
        </div>
      )}
    </>
  );
};
const BridgeWithdraw: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { isConnected } = useAccount();
  const { state, controllers } = useBridge();
  const { getPrice } = usePrices();
  const { action, coins, coin, changeCoin, network, setNetwork, getAmount, withdraw } = state;
  const { register } = controllers;

  if (action !== "withdraw") return null;
  return (
    <>
      <CoinSelector
        label={m["bridge.selectCoin"]()}
        placeholder={m["bridge.selectCoin"]()}
        isDisabled={!isConnected}
        variant="boxed"
        classNames={{ base: "w-full", trigger: "h-[56px]" }}
        value={coin ? coin.denom : undefined}
        onChange={(denom) => changeCoin(denom)}
        coins={coins}
      />
      <NetworkSelector
        label={m["bridge.selectNetwork"]()}
        placeholder={m["bridge.selectNetwork"]()}
        classNames={{ trigger: "h-[56px]" }}
        isDisabled={!coin}
        value={network ? network : undefined}
        onNetworkChange={({ id }) => setNetwork(id)}
        networks={networks}
      />

      {coin && network && (
        <div className="flex flex-col items-center justify-center gap-6">
          <AssetInputWithRange
            name="amount"
            asset={coin}
            controllers={controllers}
            showRange
            label={m["bridge.youWithdraw"]()}
          />
          <Input
            {...register("withdrawAddress", { mask: masks[network as keyof typeof masks] })}
            label={m["bridge.withdrawAddress"]()}
            placeholder={m["bridge.placeholderWithdrawAddress"]({ network: capitalize(network) })}
          />
          <Input
            placeholder="0"
            label={m["bridge.youGet"]()}
            value={getAmount}
            classNames={{
              base: "z-20",
              inputWrapper: "pl-0 py-3 flex-col h-auto gap-[6px] hover:bg-surface-secondary-rice",
              inputParent: "h-[34px] h3-bold",
              input: "!h3-bold",
            }}
            startText="right"
            startContent={
              <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
                <div className="flex gap-2 items-center font-semibold">
                  <img src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
                  <p>{coin.symbol}</p>
                </div>
              </div>
            }
            insideBottomComponent={
              <div className="flex justify-end w-full h-[22px] text-ink-tertiary-500 diatype-sm-regular">
                <p>
                  {getPrice(getAmount, coin.denom, {
                    format: true,
                    formatOptions: { ...formatNumberOptions, maximumTotalDigits: 6 },
                  })}
                </p>
              </div>
            }
          />

          <Button
            fullWidth
            onClick={() => withdraw.mutate()}
            isLoading={withdraw.isPending}
            className="mt-4"
          >
            {m["bridge.withdraw"]()}
          </Button>
        </div>
      )}
    </>
  );
};

export const Bridge = Object.assign(BridgeContainer, {
  Deposit: BridgeDeposit,
  Withdraw: BridgeWithdraw,
});
