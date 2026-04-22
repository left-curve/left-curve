import {
  AssetInputWithRange,
  Button,
  IconTwoArrows,
  Modals,
  WarningContainer,
  createContext,
  useApp,
  useInputs,
  useWatchEffect,
} from "@left-curve/applets-kit";
import {
  perpsUserStateExtendedStore,
  useAccount,
  useBalances,
  useConfig,
  usePerpsUserStateExtended,
  usePublicClient,
  useSigningClient,
  useSubmitTx,
} from "@left-curve/store";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  AccountSearchInput,
  CoinSelector,
  Input,
  ResizerContainer,
  Tab,
  Tabs,
} from "@left-curve/applets-kit";
import { Link } from "@tanstack/react-router";
import type { Address } from "@left-curve/dango/types";
import { isValidAddress } from "@left-curve/dango";
import {
  Decimal,
  formatUnits,
  parseUnits,
  truncateDec,
  wait,
  withResolvers,
} from "@left-curve/dango/utils";
import { perpsMarginAsset } from "@left-curve/store";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import { WarningTransferAccounts } from "~/components/transfer/WarningTransferAccounts";

import type React from "react";
import type { PropsWithChildren } from "react";

type TransferAction = "send" | "spot-perp";

const [TransferProvider, useTransfer] = createContext<{
  action: TransferAction;
  changeAction: (action: string) => void;
  controllers: ReturnType<typeof useInputs>;
}>({
  name: "TransferContext",
});

type TransferProps = {
  action: TransferAction;
  changeAction: (action: string) => void;
};

const TransferContainer: React.FC<PropsWithChildren<TransferProps>> = ({
  children,
  action,
  changeAction,
}) => {
  const controllers = useInputs({ strategy: "onSubmit" });
  const { isConnected } = useAccount();

  useWatchEffect(isConnected, (v) => !v && changeAction("send"));

  return (
    <TransferProvider value={{ action, changeAction, controllers }}>
      <ResizerContainer
        layoutId="send-and-receive"
        className="max-w-[400px] flex flex-col gap-8 rounded-xl w-full"
      >
        <Tabs
          layoutId="tabs-send-and-receive"
          selectedTab={isConnected ? action : "send"}
          fullWidth
          onTabChange={changeAction}
        >
          <Tab title="send">{m["common.send"]()}</Tab>
          {isConnected && <Tab title="spot-perp">{m["accountMenu.spotPerp"]()}</Tab>}
        </Tabs>
        {children}
      </ResizerContainer>
    </TransferProvider>
  );
};

const TransferSend: React.FC = () => {
  const { action, controllers } = useTransfer();
  const { showModal } = useApp();
  const queryClient = useQueryClient();
  const [selectedDenom, setSelectedDenom] = useState("bridge/usdc");

  const { register, reset, handleSubmit, inputs } = controllers;

  const { account, username, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();
  const publicClient = usePublicClient();

  const { refetch: refreshBalances, data: balances = {} } = useBalances({
    address: account?.address as Address,
  });

  const isValid20HexAddress = isValidAddress(inputs.address?.value || "");

  const { data: doesUserExist = false, isLoading } = useQuery({
    enabled: !!inputs.address?.value?.length,
    queryKey: ["transfer", inputs.address?.value],
    queryFn: async ({ signal }) => {
      await wait(450);
      if (signal.aborted || !isValid20HexAddress) return false;

      const account = await publicClient.getAccountInfo({
        address: inputs.address?.value as Address,
      });

      if (!account) return false;
      return true;
    },
  });

  const showAddressWarning =
    !isLoading &&
    action === "send" &&
    inputs.address?.value &&
    isValid20HexAddress &&
    !doesUserExist;

  const selectedCoin = coins.byDenom[selectedDenom];

  const { mutateAsync: onSubmit, isPending } = useSubmitTx<
    void,
    Error,
    { amount: string; address: string }
  >({
    submission: {
      success: m["sendAndReceive.sendSuccessfully"](),
    },
    mutation: {
      mutationFn: async ({ address, amount }, { abort }) => {
        if (!signingClient) throw new Error("error: no signing client");

        const parsedAmount = parseUnits(amount, selectedCoin.decimals);

        const { promise, resolve: confirmSend, reject: rejectSend } = withResolvers();

        showModal(Modals.ConfirmSend, {
          amount: parsedAmount,
          denom: selectedDenom,
          to: address as Address,
          confirmSend,
          rejectSend,
        });

        await promise.catch(abort);

        await signingClient.transfer({
          transfer: {
            [address]: {
              [selectedCoin.denom]: parsedAmount.toString(),
            },
          },
          sender: account!.address as Address,
        });
      },
      onSuccess: () => {
        reset();
        refreshBalances();
        queryClient.invalidateQueries({ queryKey: ["quests", username] });
      },
    },
  });

  if (action !== "send") return null;

  const transferHintParts = m["transfer.warning.transferWithdrawHint"]({ app: "{app}" }).split("{app}");

  return (
    <div className="flex flex-col w-full gap-4">
      <WarningContainer
        description={
          <p>
            {transferHintParts[0]}
            <Button as={Link} to="/bridge" search={{ action: "withdraw" }} variant="link" size="xs" className="p-0 h-fit m-0 inline">
              {m["transfer.warning.withdraw"]()}
            </Button>
            {transferHintParts[1]}
          </p>
        }
      />
      <form onSubmit={handleSubmit(onSubmit)}>
        <div className="flex flex-col w-full gap-4">
          <AssetInputWithRange
            name="amount"
            label="You're sending"
            asset={selectedCoin}
            balances={balances}
            controllers={controllers}
            isDisabled={isPending || !isConnected}
            shouldValidate
            showRange
            showCoinSelector
            onSelectCoin={(denom) => setSelectedDenom(denom)}
            renderSelector={({ value, onChange, isDisabled }) => (
              <CoinSelector
                coins={
                  isConnected
                    ? Object.keys({ "bridge/usdc": "", ...balances }).map((denom) =>
                        coins.getCoinInfo(denom),
                      )
                    : [coins.byDenom[selectedDenom]]
                }
                value={value}
                isDisabled={isDisabled}
                onChange={(k) => onChange(k)}
              />
            )}
          />
          <AccountSearchInput
            {...register("address", {
              validate: (v) => isValidAddress(v) || m["errors.validations.invalidAddress"](),
              mask: (v) => v.toLowerCase().replace(/[^a-z0-9_]/g, ""),
            })}
            label="To"
            placeholder="Wallet address or name"
            isDisabled={isPending || !isConnected}
          />
        </div>

        <Button
          type="submit"
          fullWidth
          className="mt-5"
          isLoading={isPending}
          isDisabled={
            !isConnected || !!inputs.amount?.error || !isValid20HexAddress || !!showAddressWarning
          }
        >
          {m["common.send"]()}
        </Button>
      </form>
      {showAddressWarning && <WarningTransferAccounts variant="send" />}
    </div>
  );
};

type SpotPerpDirection = "spot-to-perp" | "perp-to-spot";

const TransferSpotPerp: React.FC = () => {
  const { action, controllers } = useTransfer();
  const [direction, setDirection] = useState<SpotPerpDirection>("spot-to-perp");

  const { account, isConnected } = useAccount();
  const { coins } = useConfig();
  const { data: signingClient } = useSigningClient();

  usePerpsUserStateExtended();
  const availableMargin = perpsUserStateExtendedStore((s) => s.availableMargin);

  const { inputs, reset } = controllers;

  const { refetch: refreshBalances, data: balances = {} } = useBalances({
    address: account?.address as Address,
  });

  const usdcCoin = coins.byDenom["bridge/usdc"];

  const spotUsdcBalance = balances["bridge/usdc"] || "0";
  const availableMarginRaw = parseUnits(availableMargin || "0", perpsMarginAsset.decimals);

  const isSpotToPerp = direction === "spot-to-perp";

  const fromLabel = isSpotToPerp ? m["accountMenu.spotAccount"]() : m["accountMenu.perpAccount"]();
  const toLabel = isSpotToPerp ? m["accountMenu.perpAccount"]() : m["accountMenu.spotAccount"]();

  const flipDirection = () => {
    const newDirection = isSpotToPerp ? "perp-to-spot" : "spot-to-perp";
    const newBalance = newDirection === "spot-to-perp" ? spotUsdcBalance : availableMarginRaw;
    const newDecimals =
      newDirection === "spot-to-perp" ? usdcCoin.decimals : perpsMarginAsset.decimals;
    const maxHuman = formatUnits(newBalance, newDecimals);
    const currentAmount = inputs.amount?.value || "0";

    if (currentAmount !== "0" && Decimal(currentAmount).gt(Decimal(maxHuman))) {
      controllers.setValue("amount", maxHuman);
    }

    setDirection(newDirection);
  };

  const effectiveBalances = isSpotToPerp
    ? { "bridge/usdc": spotUsdcBalance }
    : { "bridge/usdc": availableMarginRaw };

  const amount = inputs.amount?.value || "0";

  const { mutateAsync: onSubmit, isPending } = useSubmitTx<void, Error, { amount: string }>({
    submission: {
      success: isSpotToPerp
        ? m["transfer.spotPerp.depositSuccess"]()
        : m["transfer.spotPerp.withdrawSuccess"](),
    },
    mutation: {
      mutationFn: async ({ amount }) => {
        if (!signingClient) throw new Error("error: no signing client");
        if (!account) throw new Error("error: no account");

        const sender = account.address as Address;

        if (isSpotToPerp) {
          const parsedAmount = parseUnits(amount, usdcCoin.decimals);
          await signingClient.depositMargin({
            sender,
            amount: parsedAmount.toString(),
          });
        } else {
          await signingClient.withdrawMargin({
            sender,
            amount: truncateDec(amount),
          });
        }
      },
      onSuccess: () => {
        reset();
        refreshBalances();
      },
    },
  });

  if (action !== "spot-perp") return null;

  return (
    <div className="flex flex-col w-full gap-4">
      <div className="flex flex-col items-center pb-2">
        <div className="flex flex-col gap-4 w-full mb-[-8px]">
          <Input readOnly name="from" label={m["transfer.spotPerp.from"]()} value={fromLabel} />

          <div className="flex items-center justify-center">
            <button
              type="button"
              data-testid="flip-direction"
              onClick={flipDirection}
              className="flex items-center justify-center cursor-pointer"
            >
              <IconTwoArrows className="h-7 w-7 text-surface-secondary-rice" />
            </button>
          </div>
        </div>

        <Input
          readOnly
          name="to"
          label={m["transfer.spotPerp.to"]()}
          value={toLabel}
          classNames={{ inputWrapper: "hover:bg-surface-secondary-rice cursor-default" }}
        />
      </div>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit({ amount });
        }}
      >
        <div className="flex flex-col w-full gap-4">
          <AssetInputWithRange
            name="amount"
            label={m["sendAndReceive.sending"]()}
            asset={isSpotToPerp ? usdcCoin : { ...usdcCoin, ...perpsMarginAsset }}
            balances={effectiveBalances}
            controllers={controllers}
            isDisabled={isPending || !isConnected}
            shouldValidate
            showRange
            hidePrice={!isSpotToPerp}
          />

          <Input
            placeholder="0"
            readOnly
            label={m["transfer.spotPerp.youReceive"]()}
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
                  <img
                    src={isSpotToPerp ? perpsMarginAsset.logoURI : usdcCoin.logoURI}
                    alt={isSpotToPerp ? perpsMarginAsset.symbol : usdcCoin.symbol}
                    className="w-8 h-8"
                  />
                  <p>{isSpotToPerp ? perpsMarginAsset.symbol : usdcCoin.symbol}</p>
                </div>
              </div>
            }
          />

          <Button
            type="submit"
            fullWidth
            className="mt-4"
            isLoading={isPending}
            isDisabled={!isConnected || !!inputs.amount?.error || amount === "0"}
          >
            {m["sendAndReceive.title"]()}
          </Button>
        </div>
      </form>
    </div>
  );
};

export const Transfer = Object.assign(TransferContainer, {
  Send: TransferSend,
  SpotPerp: TransferSpotPerp,
});
