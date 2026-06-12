import {
  useAccount,
  useBalances,
  useBridgeState,
  useBridgeWithdraw,
  usePrices,
} from "@left-curve/store";

import {
  AssetInputWithRange,
  AuthenticatedButton,
  createContext,
  FormattedNumber,
  IconDisconnect,
  Modals,
  useApp,
  useInputs,
  useTheme,
} from "@left-curve/applets-kit";

import { Link } from "@tanstack/react-router";
import { m } from "@left-curve/foundation/paraglide/messages.js";
import {
  SwapperIframe,
  WidgetEventName,
  type ComponentStyles,
  type SwapperStyles,
} from "@swapper-finance/deposit-sdk";
import { Decimal } from "@left-curve/utils";
import { Image } from "~/components/foundation/Image";

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

import { useEffect, useRef, useState } from "react";
import type React from "react";
import type { PropsWithChildren } from "react";
import type { AnyCoin } from "@left-curve/store/types";

const SWAPPER_DST_CHAIN_ID = "dango";
const SWAPPER_DST_TOKEN_ADDR = "usdc";
const SWAPPER_DEPOSIT_OPTIONS = ["transferCrypto", "depositWithCash", "walletDeposit"] as const;
const SWAPPER_LIGHT_COMPONENT_STYLES = {
  primaryColor: "#F57589",
  primaryButtonTextColor: "#FFFCF6",
  accentColor: "#F57589",
  sphereColor: "#F57589",
  backgroundColor: "#fffcf6",
  surfaceColor: "#f5efdf",
  surfaceAltColor: "#fffaed",
  textColor: "#292929",
} satisfies ComponentStyles;
const SWAPPER_DARK_COMPONENT_STYLES = {
  primaryColor: "#F57589",
  primaryButtonTextColor: "#2D2C2A",
  accentColor: "#F57589",
  sphereColor: "#F57589",
  backgroundColor: "#2D2C2A",
  surfaceColor: "#363432",
  surfaceAltColor: "#4D4B48",
  textColor: "#FFFCF6",
} satisfies ComponentStyles;
const SWAPPER_STYLES_BY_DANGO_THEME = {
  light: {
    themeMode: "light",
    componentStyles: SWAPPER_LIGHT_COMPONENT_STYLES,
  },
  dark: {
    themeMode: "dark",
    componentStyles: SWAPPER_DARK_COMPONENT_STYLES,
  },
} satisfies Record<"light" | "dark", SwapperStyles>;

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
  const { action } = state;
  const { account, isConnected, refreshUserStatus } = useAccount();
  const { refetch: refreshBalances } = useBalances({ address: account?.address });
  const { theme } = useTheme();
  const containerRef = useRef<HTMLDivElement>(null);
  const swapperIntegratorId = import.meta.env.PUBLIC_SWAPPER_INTEGRATOR_ID?.trim();

  useEffect(() => {
    const container = containerRef.current;
    const depositWalletAddress = account?.address;

    if (
      action !== "deposit" ||
      !container ||
      !isConnected ||
      !depositWalletAddress ||
      !swapperIntegratorId
    ) {
      return;
    }

    container.replaceChildren();

    const swapper = new SwapperIframe({
      container,
      integratorId: swapperIntegratorId,
      dstChainId: SWAPPER_DST_CHAIN_ID,
      dstTokenAddr: SWAPPER_DST_TOKEN_ADDR,
      depositWalletAddress,
      supportedDepositOptions: [...SWAPPER_DEPOSIT_OPTIONS],
      styles: SWAPPER_STYLES_BY_DANGO_THEME[theme === "dark" ? "dark" : "light"],
      iframeAttributes: {
        width: "100%",
        minWidth: "0",
        height: "620px",
        borderRadius: "12px",
      },
      onEvent: (event) => {
        if (event.type !== WidgetEventName.TRANSACTION_SUCCESS) return;
        refreshBalances();
        refreshUserStatus?.();
      },
    });

    return () => {
      swapper.destroy();
      container.replaceChildren();
    };
  }, [
    account?.address,
    action,
    isConnected,
    refreshBalances,
    refreshUserStatus,
    swapperIntegratorId,
    theme,
  ]);

  if (action !== "deposit") return null;

  if (!isConnected || !account?.address) {
    return (
      <AuthenticatedButton>
        <Button fullWidth>{m["common.signin"]()}</Button>
      </AuthenticatedButton>
    );
  }

  if (!swapperIntegratorId) {
    return <WarningContainer color="error" description={m["common.failedToLoad"]()} />;
  }

  return <div ref={containerRef} className="w-full" />;
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

const BridgeWithdraw: React.FC = () => {
  const { account } = useAccount();
  const { state, controllers } = useBridge();
  const { data: balances = {} } = useBalances({ address: account?.address });
  const { getPrice } = usePrices();
  const { action, coin, network, config, reset } = state;
  const { inputs } = controllers;
  const { showModal } = useApp();

  const [destinationAddress, setDestinationAddress] = useState<{
    address: string;
    walletName?: string;
    walletIcon?: string;
  } | null>(null);

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

  const handleAddressSet = (address: string, walletName?: string, walletIcon?: string) => {
    setDestinationAddress({ address, walletName, walletIcon });
    controllers.setValue("recipient", address);
  };

  const handleDisconnect = () => {
    setDestinationAddress(null);
    controllers.setValue("recipient", "");
  };

  const feeSubtraction = Decimal(amount).minus(fee);
  const youGet = feeSubtraction.gt("0") ? feeSubtraction.toFixed() : "0";

  if (action !== "withdraw") return null;

  const withdrawHintParts = m["bridge.withdrawTransferHint"]({ app: "{app}" }).split("{app}");

  return (
    <>
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

      <BridgeSelectors />

      {coin && network && (
        <div className="flex flex-col items-center justify-center gap-6">
          <div className="flex flex-col items-center gap-4 w-full">
            {!destinationAddress ? (
              <Button
                variant="primary"
                fullWidth
                onClick={() =>
                  showModal(Modals.DestinationWallet, {
                    network,
                    onAddressSet: handleAddressSet,
                  })
                }
              >
                {m["bridge.setDestinationAddress"]()}
              </Button>
            ) : (
              <div className="flex flex-col gap-2 w-full">
                <div className="flex justify-between items-center w-full">
                  <p className="exposure-sm-italic text-ink-secondary-700">
                    {m["bridge.withdrawAddress"]()}
                  </p>
                  <div className="flex gap-2 items-center">
                    {destinationAddress.walletName && (
                      <>
                        {destinationAddress.walletIcon && (
                          <Image
                            src={destinationAddress.walletIcon}
                            alt={destinationAddress.walletName}
                            className="w-4 h-4 inline-block"
                          />
                        )}
                        <span className="diatype-sm-medium text-ink-tertiary-500">
                          {destinationAddress.walletName}
                        </span>
                      </>
                    )}
                    <IconDisconnect
                      className="w-4 h-4 inline-block text-ink-tertiary-500 hover:cursor-pointer hover:text-ink-primary-900"
                      onClick={handleDisconnect}
                    />
                  </div>
                </div>
                <div className="diatype-sm-regular text-ink-primary-900 break-all bg-surface-secondary-rice shadow-account-card rounded-lg p-3">
                  {destinationAddress.address}
                </div>
              </div>
            )}
          </div>

          {destinationAddress && (
            <>
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
                placeholder="0"
                readOnly
                label={m["bridge.youGet"]()}
                value={youGet}
                classNames={{
                  base: "z-20",
                  inputWrapper:
                    "pl-0 py-3 flex-col h-auto gap-[6px] hover:bg-surface-secondary-rice",
                  inputParent: "h-[34px] h3-bold",
                  input: "!h3-bold",
                }}
                startText="right"
                startContent={
                  <div className="inline-flex flex-row items-center gap-3 diatype-m-regular h-[46px] rounded-md min-w-14 p-3 bg-transparent justify-start">
                    <div className="flex gap-2 items-center font-semibold">
                      <Image src={coin.logoURI} alt={coin.symbol} className="w-8 h-8" />
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
            </>
          )}
        </div>
      )}
    </>
  );
};

export const Bridge = Object.assign(BridgeContainer, {
  Deposit: BridgeDeposit,
  Withdraw: BridgeWithdraw,
});
