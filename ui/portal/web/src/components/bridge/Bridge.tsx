import {
  useAccount,
  useBalances,
  useBridgeEvmDeposit,
  useBridgeState,
  useBridgeWithdraw,
  useEvmBalances,
  usePrices,
} from "@left-curve/store";

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
import { Decimal, parseUnits } from "@left-curve/dango/utils";

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
import type { AnyCoin } from "@left-curve/store/types";
import type { NonNullablePropertiesBy } from "@left-curve/dango/types";

const masks = {
  ethereum: ethAddressMask,
  base: ethAddressMask,
  arbitrum: ethAddressMask,
  sepolia: ethAddressMask,
};

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
  const state = useBridgeState({ action, controllers, config: import.meta.env.HYPERLANE_CONFIG });

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
  const { state } = useBridge();
  const { action, network } = state;

  if (action !== "deposit") return null;

  return (
    <>
      <BridgeSelectors />

      {network === "bitcoin" && <BitcoinDeposit />}

      {network && !["bitcoin", "solana"].includes(network) && <EvmDeposit />}
    </>
  );
};

const BridgeSelectors: React.FC = () => {
  const { isConnected } = useAccount();

  const { state } = useBridge();
  const { coin, changeCoin, coins, network, setNetwork, networks } = state;

  return (
    <>
      <CoinSelector
        isDisabled={!isConnected}
        label={m["bridge.selectCoin"]()}
        placeholder={m["bridge.selectCoin"]()}
        variant="boxed"
        classNames={{ base: "w-full", trigger: "h-[56px]", listboxWrapper: "top-[4rem]" }}
        value={coin?.denom}
        onChange={changeCoin}
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
    </>
  );
};

const BitcoinDeposit: React.FC = () => {
  const depositAddress = "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh";
  return <DepositAddressBox address={depositAddress} network="bitcoin" />;
};

const EvmDeposit: React.FC = () => {
  const { getPrice } = usePrices();
  const { settings } = useApp();
  const { formatNumberOptions } = settings;

  const { controllers, state } = useBridge();
  const { inputs } = controllers;

  const { coin, connector, setConnectorId, config, reset } = state as NonNullablePropertiesBy<
    typeof state,
    "coin" | "config"
  >;

  const amount = inputs.amount?.value || "0";
  const parsedAmount = BigInt(parseUnits(amount, coin.decimals));

  const { wallet, allowanceQuery, allowanceMutation, deposit } = useBridgeEvmDeposit({
    config,
    connector,
    coin,
    amount,
  });

  const requiresAllowance = allowanceQuery.data < parsedAmount;

  const evmAddress = wallet.data?.account.address;

  const { data: balances = {}, refetch: refreshBalances } = useEvmBalances({
    chain: config.chain,
    address: evmAddress,
  });

  if (!connector || !coin) {
    return <ConnectWalletWithModal fullWidth onWalletSelected={(id) => setConnectorId(id)} />;
  }

  return (
    <div className="flex flex-col items-center justify-center gap-6">
      <AssetInputWithRange
        name="amount"
        asset={coin}
        balances={balances}
        controllers={controllers}
        showRange
        shouldValidate
        label={
          <div className="flex justify-between w-full items-center">
            <p className="exposure-sm-italic text-ink-secondary-700">{m["bridge.youDeposit"]()}</p>

            <div className="flex gap-2 items-center">
              <img src={connector.icon} alt={connector.name} className="w-4 h-4 inline-block" />
              <TruncateText
                start={4}
                end={4}
                text={evmAddress || ""}
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
        readOnly
        label={m["bridge.youGet"]()}
        value={amount}
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
              {getPrice(amount, coin.denom, {
                format: true,
                formatOptions: { ...formatNumberOptions, maximumTotalDigits: 6 },
              })}
            </p>
          </div>
        }
      />

      {requiresAllowance && (
        <Button
          fullWidth
          onClick={() => allowanceMutation.mutate()}
          isLoading={allowanceMutation.isPending || allowanceQuery.isLoading}
          className="mt-4"
        >
          {m["bridge.allow"]()}
        </Button>
      )}
      {!requiresAllowance && (
        <Button
          fullWidth
          onClick={async () => {
            await deposit.mutateAsync();
            await refreshBalances();
            reset();
          }}
          isLoading={deposit.isPending}
          isDisabled={amount === "0"}
          className="mt-4"
        >
          {m["bridge.deposit"]()}
        </Button>
      )}
    </div>
  );
};

const BridgeWithdraw: React.FC = () => {
  const { settings } = useApp();
  const { formatNumberOptions } = settings;
  const { account } = useAccount();
  const { state, controllers } = useBridge();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { getPrice } = usePrices();
  const { action, coin, network, config, reset } = state;
  const { register, inputs } = controllers;

  const amount = inputs.amount?.value || "0";
  const recipient = inputs.recipient?.value || "";

  const { withdraw, withdrawFee } = useBridgeWithdraw({
    coin: coin as AnyCoin,
    config: config as NonNullable<typeof config>,
    amount,
    recipient,
  });

  const fee = withdrawFee.data || "0";

  const feeSubtraction = Decimal(amount).minus(fee);
  const youGet = feeSubtraction.gt("0") ? feeSubtraction.toFixed() : "0";

  if (action !== "withdraw") return null;

  return (
    <>
      <BridgeSelectors />

      {coin && network && (
        <div className="flex flex-col items-center justify-center gap-6">
          <AssetInputWithRange
            name="amount"
            asset={coin}
            balances={balances}
            controllers={controllers}
            showRange
            label={m["bridge.youWithdraw"]()}
            bottomComponent={
              <div className="w-full flex justify-between">
                <p>{m["bridge.minimumWithdraw"]()}</p>
                <p className="flex gap-1">
                  <span>{`> ${fee}`}</span>
                  <span>{coin.symbol}</span>
                </p>
              </div>
            }
          />
          <Input
            {...register("recipient", { mask: masks[network as keyof typeof masks] })}
            label={m["bridge.withdrawAddress"]()}
            placeholder={m["bridge.placeholderWithdrawAddress"]({
              network: m["bridge.network"]({ network }),
            })}
          />
          <Input
            placeholder="0"
            readOnly
            label={m["bridge.youGet"]()}
            value={youGet}
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
            bottomComponent={
              <div className="w-full flex justify-between">
                <p>{m["bridge.fee"]()}</p>
                <p className="flex gap-1">
                  <span>{fee}</span>
                  <span>{coin.symbol}</span>
                </p>
              </div>
            }
            insideBottomComponent={
              <div className="flex justify-end w-full h-[22px] text-ink-tertiary-500 diatype-sm-regular">
                <p>
                  {getPrice(youGet, coin.denom, {
                    format: true,
                    formatOptions: { ...formatNumberOptions, maximumTotalDigits: 6 },
                  })}
                </p>
              </div>
            }
          />

          <Button
            fullWidth
            onClick={async () => {
              await withdraw.mutateAsync();
              reset();
            }}
            isDisabled={youGet === "0"}
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
