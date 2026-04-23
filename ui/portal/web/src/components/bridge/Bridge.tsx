import {
  useAccount,
  useAppConfig,
  useBalances,
  useBridgeEvmDeposit,
  useBridgeState,
  useBridgeWithdraw,
  useConfig,
  useEvmBalances,
  usePrices,
} from "@left-curve/store";

import {
  AssetInputWithRange,
  ConnectWalletWithModal,
  createContext,
  DepositAddressBox,
  ethAddressMask,
  FormattedNumber,
  IconDisconnect,
  Modals,
  TruncateText,
  useApp,
  useInputs,
} from "@left-curve/applets-kit";

import { Link } from "@tanstack/react-router";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import { Decimal, formatUnits, parseUnits } from "@left-curve/dango/utils";

import {
  Button,
  CoinSelector,
  IconArrowDown,
  Input,
  ResizerContainer,
  NetworkSelector,
  Tabs,
  WarningContainer,
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

      <WarningContainer description={m["bridge.rateLimitWarning"]()} />
    </>
  );
};

const BridgeSelectors: React.FC = () => {
  const { isConnected } = useAccount();
  const { chain } = useConfig();

  const { state } = useBridge();
  const { coin, changeCoin, coins, network, setNetwork, networks, action } = state;

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
        coins={coins.filter(
          (c) =>
            (chain.id === "dango-1" && c.name !== "Ether" && action === "deposit") ||
            action === "withdraw",
        )}
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
  const { userStatus } = useAccount();
  const { getPrice } = usePrices();
  const { showModal } = useApp();

  const { controllers, state } = useBridge();
  const { inputs, errors } = controllers;

  const { data: appConfig } = useAppConfig();
  const { minimumDeposit } = appConfig;

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

  const handleDeposit = () =>
    showModal(Modals.BridgeDeposit, {
      coin,
      config,
      amount,
      allowanceMutation,
      requiresAllowance,
      deposit,
      reset: () => {
        refreshBalances();
        reset();
      },
    });

  return (
    <div className="flex flex-col items-center justify-center gap-6">
      <div className="flex flex-col items-center gap-2 w-full">
        <AssetInputWithRange
          name="amount"
          asset={coin}
          balances={balances}
          controllers={controllers}
          showRange
          shouldValidate
          extendValidation={(v) => {
            if (userStatus === "active") return true;
            const minDeposit = minimumDeposit[coin.denom as keyof typeof minimumDeposit];
            if (!minDeposit) return true;

            const amount = formatUnits(minDeposit, coin.decimals);
            if (Number(v) < Number(amount))
              return m["bridge.activeAccount"]({ amount: `${amount} ${coin.symbol}` });
            return true;
          }}
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
        <div className="flex items-center justify-center mt-2">
          <div className="flex items-center justify-center border border-fg-tertiary-400 rounded-full h-5 w-5">
            <IconArrowDown className="h-3 w-3 text-ink-tertiary-500" />
          </div>
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
              <FormattedNumber
                number={getPrice(amount, coin.denom)}
                formatOptions={{ currency: "USD" }}
                as="p"
              />
            </div>
          }
        />
      </div>

      <Button
        fullWidth
        onClick={handleDeposit}
        isDisabled={!!errors.amount || amount === "0"}
        className="mt-4"
      >
        {m["bridge.deposit.title"]()}
      </Button>
    </div>
  );
};

const BridgeWithdraw: React.FC = () => {
  const { account } = useAccount();
  const { state, controllers } = useBridge();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { getPrice } = usePrices();
  const { action, coin, network, config, reset } = state;
  const { register, inputs } = controllers;
  const { showModal } = useApp();

  const amount = inputs.amount?.value || "0";
  const recipient = inputs.recipient?.value || "";

  const { withdraw, withdrawFee } = useBridgeWithdraw({
    coin: coin as AnyCoin,
    config: config as NonNullable<typeof config>,
    amount,
    recipient,
    reset,
  });

  const fee = withdrawFee.data || "0";

  const handleWithdraw = () =>
    showModal(Modals.BridgeWithdraw, {
      coin,
      config,
      amount,
      recipient,
      withdraw,
      fee,
    });

  const feeSubtraction = Decimal(amount).minus(fee);
  const youGet = feeSubtraction.gt("0") ? feeSubtraction.toFixed() : "0";

  if (action !== "withdraw") return null;

  const withdrawHintParts = m["bridge.withdrawTransferHint"]({ app: "{app}" }).split("{app}");

  return (
    <>
      <BridgeSelectors />

      <WarningContainer
        description={
          <ul className="list-disc pl-4 flex flex-col gap-1">
            <li>
              {withdrawHintParts[0]}
              <Button
                as={Link}
                to="/transfer"
                variant="link"
                size="xs"
                className="p-0 h-fit m-0 inline"
              >
                {m["sendAndReceive.title"]()}
              </Button>
              {withdrawHintParts[1]}
            </li>
            <li>{m["bridge.rateLimitWarning"]()}</li>
          </ul>
        }
      />

      {coin && network && (
        <div className="flex flex-col items-center justify-center gap-6">
          <div className="flex flex-col items-center gap-4 w-full">
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
            <div className="flex items-center justify-center">
              <div className="flex items-center justify-center border border-fg-tertiary-400 rounded-full h-5 w-5">
                <IconArrowDown className="h-3 w-3 text-ink-tertiary-500" />
              </div>
            </div>
            <Input
              {...register("recipient", { mask: masks[network as keyof typeof masks] })}
              label={m["bridge.withdrawAddress"]()}
              placeholder={m["bridge.placeholderWithdrawAddress"]({
                network: m["bridge.network"]({ network }),
              })}
            />
          </div>
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
                <FormattedNumber
                  number={getPrice(youGet, coin.denom)}
                  formatOptions={{ currency: "USD" }}
                  as="p"
                />
              </div>
            }
          />

          <Button fullWidth onClick={handleWithdraw} className="mt-4" isDisabled={!recipient}>
            {m["bridge.withdraw.title"]()}
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
